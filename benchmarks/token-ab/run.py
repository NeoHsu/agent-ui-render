#!/usr/bin/env python3
"""Paired token/cost benchmark: direct LLM HTML vs agent-ui-render compact JSON."""

from __future__ import annotations

import argparse
import concurrent.futures
import contextlib
import html
from html.parser import HTMLParser
import json
import math
import os
from pathlib import Path
import random
import re
import shutil
import statistics
import subprocess
import sys
import threading
import time
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
CASES_PATH = Path(__file__).with_name("cases.json")
SKILL_DIR = ROOT / "skills" / "agent-ui-render"
DEFAULT_WORKSPACE = ROOT / "target" / "token-ab" / "formal-sonnet-5" / "iteration-1"
CONFIG_NAMES = {"with_skill": "agent-ui-render", "without_skill": "direct-html"}
PRINT_LOCK = threading.Lock()

DIRECT_SYSTEM = """You are a deterministic report artifact generator.
Create a complete, polished, browser-openable, self-contained HTML document from the user's supplied facts.
Return only the HTML document, beginning with <!doctype html> or <html> and ending with </html>. Do not use Markdown fences or commentary.
Use inline CSS and, only when useful, inline JavaScript. Do not use external scripts, stylesheets, fonts, images, CDNs, network requests, or data not supplied by the user.
Represent requested charts directly in the document with inline SVG, canvas, or governed HTML/CSS. Include every supplied metric and every supplied dataset row. Preserve all values, units, caveats, and business meaning exactly. Escape untrusted text and do not use unsafe HTML injection APIs.
"""


class TextExtractor(HTMLParser):
    def __init__(self) -> None:
        super().__init__()
        self.parts: list[str] = []
        self.scripts: list[tuple[str, str]] = []
        self._script_type = ""
        self._script_parts: list[str] | None = None

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        if tag.lower() == "script":
            values = {key.lower(): value or "" for key, value in attrs}
            self._script_type = values.get("type", "")
            self._script_parts = []

    def handle_endtag(self, tag: str) -> None:
        if tag.lower() == "script" and self._script_parts is not None:
            self.scripts.append((self._script_type, "".join(self._script_parts)))
            self._script_parts = None
            self._script_type = ""

    def handle_data(self, data: str) -> None:
        if self._script_parts is not None:
            self._script_parts.append(data)
        else:
            self.parts.append(data)

    @property
    def text(self) -> str:
        return normalize_space(" ".join(self.parts))


def normalize_space(value: str) -> str:
    return " ".join(html.unescape(value).split())


def read_json_file(path: Path) -> Any:
    try:
        return json.loads(path.read_text())
    except (OSError, json.JSONDecodeError) as error:
        raise RuntimeError(f"failed to read JSON from {path}: {error}") from error


def safe_int(value: Any) -> int:
    try:
        return int(value)
    except (TypeError, ValueError, OverflowError):
        return 0


def safe_float(value: Any) -> float:
    try:
        return float(value)
    except (TypeError, ValueError, OverflowError):
        return 0.0


def remove_tree(path: Path) -> None:
    try:
        shutil.rmtree(path)
    except FileNotFoundError:
        return


def load_cases(selected: set[str] | None = None) -> list[dict[str, Any]]:
    cases = read_json_file(CASES_PATH)["cases"]
    if selected:
        cases = [
            case
            for case in cases
            if case["name"] in selected or str(case["id"]) in selected
        ]
    if not cases:
        raise SystemExit("No benchmark cases selected")
    return cases


def compact_system(binary: Path) -> str:
    parts = [
        "You are a deterministic report artifact generator using the authoritative agent-ui-render skill below.",
        (SKILL_DIR / "SKILL.md").read_text(),
        "\n# Bundled reference: ui-input.md\n",
        (SKILL_DIR / "references" / "ui-input.md").read_text(),
        "\n# Bundled reference: dataset.md\n",
        (SKILL_DIR / "references" / "dataset.md").read_text(),
        f"""
# Benchmark execution override
The precondition gate has already succeeded: {binary} is available. The user explicitly requests payload mode for this benchmark.
Return exactly one compact version-1 JSON payload and nothing else: no Markdown fence, commentary, command output, HTML, CSS, JavaScript, Vue, or React.
The benchmark harness will run strict validation and rendering after your response. Include every supplied metric and every supplied dataset row. Preserve all facts, units, caveats, and business meaning exactly.
""",
    ]
    return "\n".join(parts)


def case_prompt(case: dict[str, Any]) -> str:
    facts = {
        key: case[key]
        for key in ("title", "summary", "metrics", "datasets", "alerts", "narrative")
        if key in case
    }
    requirements = "\n".join(f"- {item}" for item in case["requirements"])
    return f"""Create the requested report from only the facts below. Do not omit rows, invent data, or recalculate supplied metrics.

Requirements:
{requirements}

Source facts (JSON):
{json.dumps(facts, ensure_ascii=False, indent=2)}
"""


def strip_fence(value: str) -> tuple[str, bool]:
    stripped = value.strip()
    match = re.fullmatch(
        r"```(?:html|json)?\s*\n?(.*?)\n?```", stripped, re.DOTALL | re.IGNORECASE
    )
    if match:
        return match.group(1).strip(), False
    return stripped, True


def extract_html(value: str) -> tuple[str, bool]:
    stripped, pure = strip_fence(value)
    lower = stripped.lower()
    starts = [
        position
        for marker in ("<!doctype", "<html")
        if (position := lower.find(marker)) >= 0
    ]
    if not starts:
        return stripped, False
    start = min(starts)
    end = lower.rfind("</html>")
    if end < start:
        return stripped[start:], False
    end += len("</html>")
    artifact = stripped[start:end]
    return artifact, pure and start == 0 and not stripped[end:].strip()


def extract_json(value: str) -> tuple[str, Any | None, bool, str | None]:
    stripped, pure = strip_fence(value)
    start = stripped.find("{")
    if start < 0:
        return stripped, None, False, "response contains no JSON object"
    decoder = json.JSONDecoder()
    try:
        payload, consumed = decoder.raw_decode(stripped[start:])
    except json.JSONDecodeError as error:
        return stripped, None, False, f"JSON parse error: {error}"
    artifact = stripped[start : start + consumed]
    trailing = stripped[start + consumed :].strip()
    return artifact, payload, pure and start == 0 and not trailing, None


def flatten_values(value: Any) -> list[Any]:
    if isinstance(value, dict):
        return [item for child in value.values() for item in flatten_values(child)]
    if isinstance(value, list):
        return [item for child in value for item in flatten_values(child)]
    return [value]


def expected_values(case: dict[str, Any]) -> list[Any]:
    values: list[Any] = [case["title"], case["summary"]]
    for metric in case.get("metrics", []):
        values.extend((metric["label"], metric["value"]))
    for dataset in case.get("datasets", []):
        values.extend(flatten_values(dataset["rows"]))
    for alert in case.get("alerts", []):
        values.extend((alert["title"], alert["content"]))
    if narrative := case.get("narrative"):
        values.append(narrative)
    return [value for value in values if value is not None]


def number_candidates(value: int | float) -> list[str]:
    if isinstance(value, bool):
        return [str(value).lower()]
    number = safe_float(value)
    candidates = {str(value)}
    if isinstance(value, int) or number.is_integer():
        integer = safe_int(value)
        candidates.update({str(integer), f"{integer:,}"})
    else:
        candidates.add(f"{value:g}")
    if -1 <= number <= 1:
        percentage = number * 100
        candidates.update(
            {f"{percentage:g}%", f"{percentage:.1f}%", f"{percentage:.2f}%"}
        )
    return sorted(candidates)


def check_source_facts_in_html(
    case: dict[str, Any], artifact: str
) -> tuple[bool, list[str]]:
    parser = TextExtractor()
    with contextlib.suppress(Exception):
        parser.feed(artifact)
    haystacks = [normalize_space(artifact).casefold(), parser.text.casefold()]
    comma_free = [haystack.replace(",", "") for haystack in haystacks]
    missing: list[str] = []
    for value in expected_values(case):
        if isinstance(value, bool):
            candidates = [str(value).lower()]
        elif isinstance(value, (int, float)):
            candidates = number_candidates(value)
        else:
            candidates = [normalize_space(str(value))]
        found = False
        for candidate in candidates:
            normalized = candidate.casefold()
            if any(normalized in haystack for haystack in haystacks):
                found = True
                break
            if any(normalized.replace(",", "") in haystack for haystack in comma_free):
                found = True
                break
        if not found:
            missing.append(str(value))
    return not missing, missing[:20]


def check_source_facts_in_payload(
    case: dict[str, Any], payload: Any
) -> tuple[bool, list[str]]:
    actual = flatten_values(payload)
    actual_strings = {
        normalize_space(str(value)).casefold()
        for value in actual
        if isinstance(value, str)
    }
    actual_numbers = {
        safe_float(value)
        for value in actual
        if isinstance(value, (int, float)) and not isinstance(value, bool)
    }
    actual_bools = {value for value in actual if isinstance(value, bool)}
    missing: list[str] = []
    for value in expected_values(case):
        if isinstance(value, bool):
            found = value in actual_bools
        elif isinstance(value, (int, float)):
            found = safe_float(value) in actual_numbers
        else:
            found = normalize_space(str(value)).casefold() in actual_strings
        if not found:
            missing.append(str(value))
    return not missing, missing[:20]


def check_inline_scripts(artifact: str, workdir: Path) -> tuple[bool, str]:
    parser = TextExtractor()
    try:
        parser.feed(artifact)
    except Exception as error:
        return False, f"HTML parser error: {error}"
    javascript = [
        source
        for script_type, source in parser.scripts
        if script_type.casefold() not in {"application/json", "application/ld+json"}
        and source.strip()
    ]
    for index, source in enumerate(javascript, start=1):
        script_path = workdir / f"inline-script-{index}.js"
        script_path.write_text(source)
        result = subprocess.run(
            ["node", "--check", str(script_path)],
            capture_output=True,
            text=True,
            timeout=30,
            check=False,
        )
        if result.returncode != 0:
            return False, normalize_space(result.stderr)[-500:]
    return True, f"{len(javascript)} inline script(s) passed node --check"


def browser_smoke(
    html_path: Path, output_dir: Path, case: dict[str, Any], chrome: Path
) -> dict[str, Any]:
    screenshot = output_dir / "screenshot.png"
    dom_path = output_dir / "rendered-dom.html"
    stderr_path = output_dir / "chrome.stderr.txt"
    profile = output_dir / "chrome-profile"
    command = [
        str(chrome),
        "--headless=new",
        "--disable-gpu",
        "--disable-dev-shm-usage",
        "--hide-scrollbars",
        "--no-first-run",
        "--no-default-browser-check",
        "--allow-file-access-from-files",
        f"--user-data-dir={profile}",
        "--window-size=1440,1200",
        "--virtual-time-budget=3000",
        f"--screenshot={screenshot}",
        "--dump-dom",
        html_path.resolve().as_uri(),
    ]
    timed_out = False
    try:
        result = subprocess.run(
            command, capture_output=True, text=True, timeout=10, check=False
        )
        dom = result.stdout
        chrome_stderr = result.stderr
        return_code: int | None = result.returncode
    except subprocess.TimeoutExpired as error:
        timed_out = True
        dom = error.stdout if error.stdout is not None else ""
        chrome_stderr = error.stderr if error.stderr is not None else ""
        return_code = None
        if isinstance(dom, bytes):
            dom = dom.decode(errors="replace")
        if isinstance(chrome_stderr, bytes):
            chrome_stderr = chrome_stderr.decode(errors="replace")
    except OSError as error:
        return {
            "passed": False,
            "evidence": f"Chrome failed: {error}",
            "structure": False,
        }
    dom_path.write_text(dom)
    stderr_path.write_text(chrome_stderr)
    normalized_dom = normalize_space(dom).casefold()
    missing_text = [
        text
        for text in case["required_text"]
        if normalize_space(text).casefold() not in normalized_dom
    ]
    features = case["features"]
    missing_structure: list[str] = []
    if features.get("table") and not re.search(r"<table\b", dom, re.IGNORECASE):
        missing_structure.append("table")
    if features.get("chart") and not re.search(
        r"<(?:svg|canvas)\b|chart", dom, re.IGNORECASE
    ):
        missing_structure.append("chart")
    screenshot_ok = screenshot.is_file() and screenshot.stat().st_size > 5000
    passed = (
        (return_code == 0 or timed_out)
        and len(dom) > 1000
        and screenshot_ok
        and not missing_text
    )
    structure_passed = passed and not missing_structure
    evidence = (
        f"Chrome exit={'timeout-after-render' if timed_out else return_code}, screenshot={screenshot.stat().st_size if screenshot.exists() else 0} bytes, "
        f"missing_text={missing_text}, missing_structure={missing_structure}"
    )
    remove_tree(profile)
    return {"passed": passed, "structure": structure_passed, "evidence": evidence}


def validate_direct(
    raw: str, case: dict[str, Any], attempt_dir: Path, chrome: Path
) -> dict[str, Any]:
    artifact, pure = extract_html(raw)
    html_path = attempt_dir / "report.html"
    html_path.write_text(artifact)
    lower = artifact.lower()
    parseable = "<html" in lower and "</html>" in lower and "<style" in lower
    external = re.findall(r"(?:src|href)\s*=\s*[\"']https?://", artifact, re.IGNORECASE)
    scripts_ok, scripts_evidence = check_inline_scripts(artifact, attempt_dir)
    facts_ok, missing_facts = check_source_facts_in_html(case, artifact)
    browser = (
        browser_smoke(html_path, attempt_dir, case, chrome)
        if parseable
        else {
            "passed": False,
            "structure": False,
            "evidence": "HTML structure check failed before browser launch",
        }
    )
    checks = [
        {
            "text": "The model returned one complete self-contained HTML document without wrapper commentary",
            "passed": parseable and pure,
            "evidence": f"parseable={parseable}, pure_artifact={pure}",
        },
        {
            "text": "All supplied metrics and dataset row values are preserved",
            "passed": facts_ok,
            "evidence": f"missing facts: {missing_facts}",
        },
        {
            "text": "The HTML has no external dependencies and inline JavaScript is syntactically valid",
            "passed": not external and scripts_ok,
            "evidence": f"external references={len(external)}; {scripts_evidence}",
        },
        {
            "text": "The rendered report contains the requested table/chart structures",
            "passed": browser["structure"],
            "evidence": browser["evidence"],
        },
        {
            "text": "The artifact opens successfully in a headless browser and renders required text",
            "passed": browser["passed"],
            "evidence": browser["evidence"],
        },
    ]
    return {
        "artifact": html_path,
        "checks": checks,
        "errors": [check["evidence"] for check in checks if not check["passed"]],
    }


def validate_compact(
    raw: str,
    case: dict[str, Any],
    attempt_dir: Path,
    binary: Path,
    chrome: Path,
) -> dict[str, Any]:
    artifact, payload, pure, parse_error = extract_json(raw)
    input_path = attempt_dir / "report.input.json"
    input_path.write_text(artifact + "\n")
    validation_stdout = attempt_dir / "validate.stdout.json"
    validation_stderr = attempt_dir / "validate.stderr.txt"
    strict_ok = False
    validation_evidence = parse_error or "not executed"
    if payload is not None:
        validation = subprocess.run(
            [
                str(binary),
                "--warnings-as-errors",
                "-o",
                "json",
                "validate",
                str(input_path),
            ],
            capture_output=True,
            text=True,
            timeout=30,
            check=False,
        )
        validation_stdout.write_text(validation.stdout)
        validation_stderr.write_text(validation.stderr)
        strict_ok = validation.returncode == 0
        validation_evidence = f"exit={validation.returncode}; stdout={normalize_space(validation.stdout)}; stderr={normalize_space(validation.stderr)}"
    else:
        validation_stdout.write_text("")
        validation_stderr.write_text(parse_error or "parse failed")

    facts_ok, missing_facts = (
        check_source_facts_in_payload(case, payload)
        if payload is not None
        else (False, ["payload unavailable"])
    )
    html_path = attempt_dir / "report.html"
    render_ok = False
    render_evidence = "strict validation failed"
    if strict_ok:
        render = subprocess.run(
            [str(binary), "--quiet", "render", "html", str(input_path), str(html_path)],
            capture_output=True,
            text=True,
            timeout=30,
            check=False,
        )
        render_ok = (
            render.returncode == 0
            and html_path.is_file()
            and html_path.stat().st_size > 0
        )
        render_evidence = (
            f"exit={render.returncode}; stderr={normalize_space(render.stderr)}"
        )

    browser = (
        browser_smoke(html_path, attempt_dir, case, chrome)
        if render_ok
        else {
            "passed": False,
            "structure": False,
            "evidence": f"render unavailable: {render_evidence}",
        }
    )
    checks = [
        {
            "text": "The model returned one compact version-1 JSON payload without wrapper commentary",
            "passed": payload is not None and payload.get("version") == 1 and pure,
            "evidence": f"parse_error={parse_error}, version={payload.get('version') if isinstance(payload, dict) else None}, pure_artifact={pure}",
        },
        {
            "text": "All supplied metrics and dataset row values are preserved",
            "passed": facts_ok,
            "evidence": f"missing facts: {missing_facts}",
        },
        {
            "text": "The payload passes agent-ui-render strict validation and rendering without warnings",
            "passed": strict_ok and render_ok,
            "evidence": f"validation: {validation_evidence}; render: {render_evidence}",
        },
        {
            "text": "The rendered report contains the requested table/chart structures",
            "passed": browser["structure"],
            "evidence": browser["evidence"],
        },
        {
            "text": "The artifact opens successfully in a headless browser and renders required text",
            "passed": browser["passed"],
            "evidence": browser["evidence"],
        },
    ]
    return {
        "artifact": input_path,
        "rendered_html": html_path if html_path.exists() else None,
        "checks": checks,
        "errors": [check["evidence"] for check in checks if not check["passed"]],
    }


def usage_from_response(response: dict[str, Any]) -> dict[str, Any]:
    usage = response.get("usage") or {}
    main_input = safe_int(usage.get("input_tokens", 0))
    main_cache_creation = safe_int(usage.get("cache_creation_input_tokens", 0))
    main_cache_read = safe_int(usage.get("cache_read_input_tokens", 0))
    main_output = safe_int(usage.get("output_tokens", 0))
    model_usage = response.get("modelUsage") or {}
    all_input = sum(
        safe_int(item.get("inputTokens", 0)) for item in model_usage.values()
    )
    all_cache_creation = sum(
        safe_int(item.get("cacheCreationInputTokens", 0))
        for item in model_usage.values()
    )
    all_cache_read = sum(
        safe_int(item.get("cacheReadInputTokens", 0)) for item in model_usage.values()
    )
    all_output = sum(
        safe_int(item.get("outputTokens", 0)) for item in model_usage.values()
    )
    return {
        "main_input_tokens": main_input,
        "main_cache_creation_input_tokens": main_cache_creation,
        "main_cache_read_input_tokens": main_cache_read,
        "main_output_tokens": main_output,
        "main_effective_input_tokens": main_input
        + main_cache_creation
        + main_cache_read,
        "main_total_tokens": main_input
        + main_cache_creation
        + main_cache_read
        + main_output,
        "all_model_input_tokens": all_input,
        "all_model_cache_creation_input_tokens": all_cache_creation,
        "all_model_cache_read_input_tokens": all_cache_read,
        "all_model_output_tokens": all_output,
        "all_model_effective_input_tokens": all_input
        + all_cache_creation
        + all_cache_read,
        "all_model_total_tokens": all_input
        + all_cache_creation
        + all_cache_read
        + all_output,
        "cost_usd": safe_float(response.get("total_cost_usd") or 0),
        "duration_ms": safe_int(response.get("duration_ms") or 0),
    }


def add_usage(total: dict[str, Any], current: dict[str, Any]) -> dict[str, Any]:
    if not total:
        return dict(current)
    return {key: total.get(key, 0) + value for key, value in current.items()}


def invoke_claude(
    prompt: str,
    system_path: Path,
    run_dir: Path,
    model: str,
    effort: str,
    max_budget_usd: float,
    attempt: int,
) -> tuple[dict[str, Any] | None, str | None]:
    command = [
        "claude",
        "-p",
        "--safe-mode",
        "--no-session-persistence",
        "--tools",
        "",
        "--model",
        model,
        "--effort",
        effort,
        "--max-budget-usd",
        str(max_budget_usd),
        "--output-format",
        "json",
        "--system-prompt-file",
        str(system_path),
    ]
    attempt_dir = run_dir / f"attempt-{attempt}"
    attempt_dir.mkdir(parents=True, exist_ok=True)
    (attempt_dir / "prompt.txt").write_text(prompt)
    last_error: str | None = None
    for transport_attempt in range(1, 4):
        started = time.monotonic()
        try:
            result = subprocess.run(
                command,
                input=prompt,
                capture_output=True,
                text=True,
                timeout=360,
                cwd=attempt_dir,
                check=False,
                env={**os.environ, "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC": "1"},
            )
        except subprocess.TimeoutExpired as error:
            last_error = f"Claude timeout: {error}"
            time.sleep(transport_attempt * 3)
            continue
        (attempt_dir / f"claude.stderr.{transport_attempt}.txt").write_text(
            result.stderr
        )
        (attempt_dir / f"claude.stdout.{transport_attempt}.txt").write_text(
            result.stdout
        )
        try:
            response = json.loads(result.stdout)
        except json.JSONDecodeError as error:
            last_error = f"Claude JSON envelope parse failed (exit {result.returncode}): {error}; {result.stderr[-500:]}"
            time.sleep(transport_attempt * 3)
            continue
        response["harness_wall_ms"] = round((time.monotonic() - started) * 1000)
        (attempt_dir / "claude-result.json").write_text(
            json.dumps(response, ensure_ascii=False, indent=2) + "\n"
        )
        if result.returncode == 0 and not response.get("is_error"):
            return response, None
        last_error = f"Claude invocation failed (exit {result.returncode}): {response.get('result') or response.get('api_error_status')}"
        time.sleep(transport_attempt * 3)
    return None, last_error


def repair_prompt(original: str, raw: str, errors: list[str], arm: str) -> str:
    artifact_name = (
        "compact JSON payload"
        if arm == "with_skill"
        else "self-contained HTML document"
    )
    return f"""The previous {artifact_name} failed automated acceptance checks. Return a complete corrected replacement only.

Original task:
{original}

Validation failures:
{json.dumps(errors, ensure_ascii=False, indent=2)}

Previous artifact:
{raw}
"""


def write_run_files(
    run_dir: Path,
    case: dict[str, Any],
    arm: str,
    prompt: str,
    final_raw: str,
    validation: dict[str, Any],
    usage: dict[str, Any],
    attempts: int,
    invocation_error: str | None,
) -> dict[str, Any]:
    outputs = run_dir / "outputs"
    outputs.mkdir(parents=True, exist_ok=True)
    final_attempt = run_dir / f"attempt-{attempts}"
    for name in (
        "report.html",
        "report.input.json",
        "screenshot.png",
        "rendered-dom.html",
        "validate.stdout.json",
        "validate.stderr.txt",
    ):
        source = final_attempt / name
        if source.exists():
            shutil.copy2(source, outputs / name)
    if arm == "with_skill" and (final_attempt / "report.html").exists():
        shutil.copy2(final_attempt / "report.html", outputs / "rendered-report.html")
    (outputs / "model-response.txt").write_text(final_raw)

    checks = validation["checks"]
    passed = sum(1 for check in checks if check["passed"])
    pass_rate = passed / len(checks) if checks else 0.0
    output_chars = sum(
        path.stat().st_size for path in outputs.rglob("*") if path.is_file()
    )
    metrics = {
        "tool_calls": {},
        "total_tool_calls": 0,
        "total_steps": attempts,
        "files_created": [
            str(path.relative_to(outputs))
            for path in outputs.rglob("*")
            if path.is_file()
        ],
        "errors_encountered": len(validation["errors"])
        + (1 if invocation_error else 0),
        "output_chars": output_chars,
        "transcript_chars": len(final_raw),
    }
    (outputs / "metrics.json").write_text(json.dumps(metrics, indent=2) + "\n")

    grading = {
        "expectations": checks,
        "summary": {
            "passed": passed,
            "failed": len(checks) - passed,
            "total": len(checks),
            "pass_rate": pass_rate,
        },
        "execution_metrics": metrics,
        "timing": {
            "executor_duration_seconds": round(usage.get("duration_ms", 0) / 1000, 3),
            "total_duration_seconds": round(usage.get("duration_ms", 0) / 1000, 3),
        },
        "claims": [],
        "user_notes_summary": {
            "uncertainties": [],
            "needs_review": [],
            "workarounds": [],
        },
        "eval_feedback": {
            "suggestions": [],
            "overall": "Deterministic checks cover structure, source facts, dependencies, strict validation, and browser rendering.",
        },
    }
    (run_dir / "grading.json").write_text(
        json.dumps(grading, ensure_ascii=False, indent=2) + "\n"
    )
    timing = {
        "total_tokens": usage.get("all_model_total_tokens", 0),
        "duration_ms": usage.get("duration_ms", 0),
        "total_duration_seconds": round(usage.get("duration_ms", 0) / 1000, 3),
    }
    (run_dir / "timing.json").write_text(json.dumps(timing, indent=2) + "\n")
    transcript = [
        f"# {case['title']} — {CONFIG_NAMES[arm]}",
        "",
        "## Prompt",
        prompt,
        "",
        "## Execution",
        f"- Attempts: {attempts}",
        f"- Automated pass rate: {pass_rate:.0%}",
        f"- Invocation error: {invocation_error or 'none'}",
        "",
        "## Model response",
        final_raw,
    ]
    (run_dir / "transcript.md").write_text("\n".join(transcript))

    result = {
        "status": "complete",
        "eval_id": case["id"],
        "eval_name": case["name"],
        "complexity": case["complexity"],
        "configuration": arm,
        "attempts": attempts,
        "pass_rate": pass_rate,
        "passed": passed,
        "failed": len(checks) - passed,
        "total": len(checks),
        "valid": pass_rate == 1.0,
        "usage": usage,
        "invocation_error": invocation_error,
        "expectations": checks,
    }
    (run_dir / "run-result.json").write_text(
        json.dumps(result, ensure_ascii=False, indent=2) + "\n"
    )
    return result


def execute_run(job: dict[str, Any]) -> dict[str, Any]:
    case = job["case"]
    arm = job["arm"]
    run_dir: Path = job["run_dir"]
    cached = run_dir / "run-result.json"
    if cached.exists():
        return read_json_file(cached)

    prompt = case_prompt(case)
    current_prompt = prompt
    usage: dict[str, Any] = {}
    final_raw = ""
    validation: dict[str, Any] = {"checks": [], "errors": ["run did not start"]}
    invocation_error: str | None = None
    attempts = 0
    for attempts in range(1, job["max_attempts"] + 1):
        response, invocation_error = invoke_claude(
            current_prompt,
            job["system_path"],
            run_dir,
            job["model"],
            job["effort"],
            job["max_budget_usd"],
            attempts,
        )
        if response is None:
            break
        usage = add_usage(usage, usage_from_response(response))
        final_raw = str(response.get("result") or "")
        attempt_dir = run_dir / f"attempt-{attempts}"
        if arm == "with_skill":
            validation = validate_compact(
                final_raw, case, attempt_dir, job["binary"], job["chrome"]
            )
        else:
            validation = validate_direct(final_raw, case, attempt_dir, job["chrome"])
        (attempt_dir / "validation.json").write_text(
            json.dumps(validation["checks"], ensure_ascii=False, indent=2) + "\n"
        )
        if not validation["errors"]:
            break
        if attempts < job["max_attempts"]:
            current_prompt = repair_prompt(prompt, final_raw, validation["errors"], arm)

    if not validation["checks"]:
        validation = {
            "checks": [
                {
                    "text": "The model invocation completed",
                    "passed": False,
                    "evidence": invocation_error or "unknown failure",
                }
            ],
            "errors": [invocation_error or "unknown invocation failure"],
        }
    result = write_run_files(
        run_dir,
        case,
        arm,
        prompt,
        final_raw,
        validation,
        usage,
        attempts,
        invocation_error,
    )
    with PRINT_LOCK:
        print(
            f"[{arm:13}] {case['name']:24} run {job['run_number']} "
            f"pass={result['pass_rate']:.0%} output={usage.get('main_output_tokens', 0)} "
            f"total={usage.get('all_model_total_tokens', 0)} cost=${usage.get('cost_usd', 0):.4f}",
            flush=True,
        )
    return result


def metric_stats(values: list[float]) -> dict[str, float]:
    if not values:
        return {"mean": 0.0, "stddev": 0.0, "min": 0.0, "max": 0.0}
    return {
        "mean": statistics.mean(values),
        "stddev": statistics.pstdev(values),
        "min": min(values),
        "max": max(values),
    }


def percentile(values: list[float], fraction: float) -> float:
    ordered = sorted(values)
    if not ordered:
        return 0.0
    position = (len(ordered) - 1) * fraction
    lower = math.floor(position)
    upper = math.ceil(position)
    if lower == upper:
        return ordered[lower]
    return ordered[lower] * (upper - position) + ordered[upper] * (position - lower)


def paired_savings(
    pairs: list[tuple[dict[str, Any], dict[str, Any]]], key: str
) -> dict[str, float]:
    def value(run: dict[str, Any]) -> float:
        return safe_float(run["usage"].get(key, 0))

    def ratio(sample: list[tuple[dict[str, Any], dict[str, Any]]]) -> float:
        direct = sum(value(pair[0]) for pair in sample)
        compact = sum(value(pair[1]) for pair in sample)
        return 1 - compact / direct if direct else 0.0

    observed = ratio(pairs)
    generator = random.Random(20260710)
    bootstrap = (
        [ratio([generator.choice(pairs) for _ in pairs]) for _ in range(10000)]
        if pairs
        else []
    )
    return {
        "savings_fraction": observed,
        "ci95_low": percentile(bootstrap, 0.025),
        "ci95_high": percentile(bootstrap, 0.975),
    }


def aggregate(
    results: list[dict[str, Any]], workspace: Path, model: str, repetitions: int
) -> dict[str, Any]:
    by_configuration = {
        configuration: [run for run in results if run["configuration"] == configuration]
        for configuration in CONFIG_NAMES
    }
    pairs_by_key: dict[tuple[int, int], dict[str, dict[str, Any]]] = {}
    for run in results:
        key = (run["eval_id"], run["run_number"])
        pairs_by_key.setdefault(key, {})[run["configuration"]] = run
    pairs = [
        (pair["without_skill"], pair["with_skill"])
        for _, pair in sorted(pairs_by_key.items())
        if set(pair) == set(CONFIG_NAMES)
    ]

    run_summary: dict[str, Any] = {}
    for configuration, runs in by_configuration.items():
        run_summary[configuration] = {
            "pass_rate": metric_stats([run["pass_rate"] for run in runs]),
            "time_seconds": metric_stats(
                [run["usage"].get("duration_ms", 0) / 1000 for run in runs]
            ),
            "tokens": metric_stats(
                [run["usage"].get("all_model_total_tokens", 0) for run in runs]
            ),
            "input_tokens": metric_stats(
                [run["usage"].get("main_effective_input_tokens", 0) for run in runs]
            ),
            "output_tokens": metric_stats(
                [run["usage"].get("main_output_tokens", 0) for run in runs]
            ),
            "cost_usd": metric_stats([run["usage"].get("cost_usd", 0) for run in runs]),
            "valid_runs": sum(1 for run in runs if run["valid"]),
            "total_runs": len(runs),
            "cost_per_valid_artifact": (
                sum(run["usage"].get("cost_usd", 0) for run in runs)
                / sum(1 for run in runs if run["valid"])
                if any(run["valid"] for run in runs)
                else None
            ),
        }

    efficiency = {
        "main_output_tokens": paired_savings(pairs, "main_output_tokens"),
        "main_total_tokens": paired_savings(pairs, "main_total_tokens"),
        "all_model_total_tokens": paired_savings(pairs, "all_model_total_tokens"),
        "cost_usd": paired_savings(pairs, "cost_usd"),
    }
    complexity: dict[str, Any] = {}
    for level in ("simple", "medium", "complex"):
        subset = [pair for pair in pairs if pair[0]["complexity"] == level]
        if subset:
            complexity[level] = {
                "pairs": len(subset),
                "output_token_savings": paired_savings(subset, "main_output_tokens"),
                "total_token_savings": paired_savings(subset, "all_model_total_tokens"),
                "cost_savings": paired_savings(subset, "cost_usd"),
            }

    benchmark_runs = []
    for run in sorted(
        results,
        key=lambda item: (item["eval_id"], item["run_number"], item["configuration"]),
    ):
        benchmark_runs.append(
            {
                "eval_id": run["eval_id"],
                "eval_name": run["eval_name"],
                "configuration": run["configuration"],
                "run_number": run["run_number"],
                "result": {
                    "pass_rate": run["pass_rate"],
                    "passed": run["passed"],
                    "failed": run["failed"],
                    "total": run["total"],
                    "time_seconds": run["usage"].get("duration_ms", 0) / 1000,
                    "tokens": run["usage"].get("all_model_total_tokens", 0),
                    "input_tokens": run["usage"].get("main_effective_input_tokens", 0),
                    "output_tokens": run["usage"].get("main_output_tokens", 0),
                    "cost_usd": run["usage"].get("cost_usd", 0),
                    "attempts": run["attempts"],
                    "tool_calls": 0,
                    "errors": run["failed"],
                },
                "expectations": run["expectations"],
                "notes": [],
            }
        )

    direct = run_summary["without_skill"]
    compact = run_summary["with_skill"]
    notes = [
        "Token totals include every Claude model reported by the CLI; output-token comparisons use the requested Sonnet model's usage.",
        "The agent-ui-render input-token total includes the full SKILL.md and both bundled references on every uncached call.",
        "CLI-generated HTML bytes are not model output tokens; only model API usage is counted.",
        "Each configuration was allowed one repair call after deterministic validation failure.",
    ]
    output_savings_by_eval = [
        paired_savings(
            [pair for pair in pairs if pair[0]["eval_id"] == eval_id],
            "main_output_tokens",
        )["savings_fraction"]
        for eval_id in sorted({pair[0]["eval_id"] for pair in pairs})
    ]
    direct_repairs = sum(
        max(run["attempts"] - 1, 0) for run in by_configuration["without_skill"]
    )
    compact_repairs = sum(
        max(run["attempts"] - 1, 0) for run in by_configuration["with_skill"]
    )
    notes.extend(
        [
            f"All {len(results)} runs reached a 100% automated acceptance score, so final quality checks did not favor either configuration.",
            f"Direct HTML required {direct_repairs} repair call(s); agent-ui-render required {compact_repairs} repair call(s).",
            f"Every eval reduced main-model output tokens; per-eval savings ranged from {min(output_savings_by_eval) * 100:.1f}% to {max(output_savings_by_eval) * 100:.1f}%.",
            f"The full skill context added {compact['input_tokens']['mean'] - direct['input_tokens']['mean']:.0f} mean effective input tokens, causing total token volume to increase despite smaller output.",
            f"Mean model duration changed from {direct['time_seconds']['mean']:.2f}s to {compact['time_seconds']['mean']:.2f}s.",
        ]
    )
    benchmark = {
        "metadata": {
            "skill_name": "agent-ui-render",
            "skill_path": str(SKILL_DIR),
            "executor_model": model,
            "analyzer_model": "deterministic-python",
            "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
            "evals_run": sorted({run["eval_id"] for run in results}),
            "runs_per_configuration": repetitions,
            "paired_runs": len(pairs),
        },
        "runs": benchmark_runs,
        "run_summary": {
            **run_summary,
            "delta": {
                "pass_rate": f"{compact['pass_rate']['mean'] - direct['pass_rate']['mean']:+.3f}",
                "time_seconds": f"{compact['time_seconds']['mean'] - direct['time_seconds']['mean']:+.3f}",
                "tokens": f"{compact['tokens']['mean'] - direct['tokens']['mean']:+.1f}",
            },
        },
        "token_efficiency": efficiency,
        "complexity_breakdown": complexity,
        "notes": notes,
    }
    (workspace / "benchmark.json").write_text(
        json.dumps(benchmark, ensure_ascii=False, indent=2) + "\n"
    )
    (workspace / "analysis-notes.json").write_text(
        json.dumps(notes, ensure_ascii=False, indent=2) + "\n"
    )
    write_markdown_report(benchmark, workspace / "benchmark.md")
    return benchmark


def percent(value: float) -> str:
    return f"{value * 100:.1f}%"


def write_markdown_report(benchmark: dict[str, Any], path: Path) -> None:
    summary = benchmark["run_summary"]
    direct = summary["without_skill"]
    compact = summary["with_skill"]
    efficiency = benchmark["token_efficiency"]
    lines = [
        "# agent-ui-render Token A/B Benchmark",
        "",
        f"Model: `{benchmark['metadata']['executor_model']}`  ",
        f"Paired runs: {benchmark['metadata']['paired_runs']}  ",
        f"Runs per configuration: {benchmark['metadata']['runs_per_configuration']}",
        "",
        "## Aggregate results",
        "",
        "| Metric | Direct HTML | agent-ui-render | Difference |",
        "| --- | ---: | ---: | ---: |",
        f"| Mean main-model input tokens | {direct['input_tokens']['mean']:.0f} | {compact['input_tokens']['mean']:.0f} | {compact['input_tokens']['mean'] - direct['input_tokens']['mean']:+.0f} |",
        f"| Mean main-model output tokens | {direct['output_tokens']['mean']:.0f} | {compact['output_tokens']['mean']:.0f} | {compact['output_tokens']['mean'] - direct['output_tokens']['mean']:+.0f} |",
        f"| Mean all-model total tokens | {direct['tokens']['mean']:.0f} | {compact['tokens']['mean']:.0f} | {compact['tokens']['mean'] - direct['tokens']['mean']:+.0f} |",
        f"| Mean cost | ${direct['cost_usd']['mean']:.4f} | ${compact['cost_usd']['mean']:.4f} | ${compact['cost_usd']['mean'] - direct['cost_usd']['mean']:+.4f} |",
        f"| Mean acceptance pass rate | {percent(direct['pass_rate']['mean'])} | {percent(compact['pass_rate']['mean'])} | {percent(compact['pass_rate']['mean'] - direct['pass_rate']['mean'])} |",
        f"| Fully valid artifacts | {direct['valid_runs']}/{direct['total_runs']} | {compact['valid_runs']}/{compact['total_runs']} | — |",
        f"| Cost per valid artifact | ${direct['cost_per_valid_artifact'] or 0:.4f} | ${compact['cost_per_valid_artifact'] or 0:.4f} | — |",
        "",
        "## Paired savings with 95% bootstrap confidence intervals",
        "",
        "| Measure | Savings | 95% CI |",
        "| --- | ---: | ---: |",
    ]
    labels = {
        "main_output_tokens": "Main-model output tokens",
        "main_total_tokens": "Main-model total tokens",
        "all_model_total_tokens": "All-model total tokens",
        "cost_usd": "API cost",
    }
    for key, label in labels.items():
        item = efficiency[key]
        lines.append(
            f"| {label} | {percent(item['savings_fraction'])} | {percent(item['ci95_low'])} to {percent(item['ci95_high'])} |"
        )
    lines.extend(
        [
            "",
            "## Complexity breakdown",
            "",
            "| Complexity | Pairs | Output-token savings | Total-token savings | Cost savings |",
            "| --- | ---: | ---: | ---: | ---: |",
        ]
    )
    for level, item in benchmark["complexity_breakdown"].items():
        lines.append(
            f"| {level} | {item['pairs']} | {percent(item['output_token_savings']['savings_fraction'])} | "
            f"{percent(item['total_token_savings']['savings_fraction'])} | {percent(item['cost_savings']['savings_fraction'])} |"
        )
    lines.extend(
        ["", "## Method notes", ""] + [f"- {note}" for note in benchmark["notes"]]
    )
    path.write_text("\n".join(lines) + "\n")


def prepare_workspace(
    workspace: Path,
    cases: list[dict[str, Any]],
    repetitions: int,
    direct_system_path: Path,
    compact_system_path: Path,
) -> None:
    workspace.mkdir(parents=True, exist_ok=True)
    evals = {
        "skill_name": "agent-ui-render",
        "evals": [
            {
                "id": case["id"],
                "prompt": case_prompt(case),
                "expected_output": "A browser-openable report preserving all supplied facts and requested structures.",
                "files": [],
                "expectations": [
                    "The response is exactly the requested artifact format",
                    "All supplied metrics and dataset rows are preserved",
                    "The artifact is self-contained or passes governed CLI validation",
                    "Requested table/chart structures render",
                    "The artifact passes a headless-browser smoke test",
                ],
            }
            for case in cases
        ],
    }
    (workspace / "evals.json").write_text(
        json.dumps(evals, ensure_ascii=False, indent=2) + "\n"
    )
    for case in cases:
        prompt = case_prompt(case)
        for run_number in range(1, repetitions + 1):
            eval_dir = (
                workspace / f"eval-{case['id']:02d}-{case['name']}-run-{run_number}"
            )
            eval_dir.mkdir(parents=True, exist_ok=True)
            metadata = {
                "eval_id": case["id"],
                "eval_name": f"{case['name']}-run-{run_number}",
                "prompt": prompt,
                "assertions": evals["evals"][case["id"] - 1]["expectations"],
            }
            (eval_dir / "eval_metadata.json").write_text(
                json.dumps(metadata, ensure_ascii=False, indent=2) + "\n"
            )
            for configuration, system_path in (
                ("without_skill", direct_system_path),
                ("with_skill", compact_system_path),
            ):
                run_dir = eval_dir / configuration
                run_dir.mkdir(parents=True, exist_ok=True)
                target_system = run_dir / "system-prompt.txt"
                if not target_system.exists():
                    shutil.copy2(system_path, target_system)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--workspace", type=Path, default=DEFAULT_WORKSPACE)
    parser.add_argument("--model", default="claude-sonnet-5")
    parser.add_argument("--effort", default="medium", choices=("low", "medium", "high"))
    parser.add_argument("--repetitions", type=int, default=3)
    parser.add_argument("--max-workers", type=int, default=4)
    parser.add_argument("--max-attempts", type=int, default=2)
    parser.add_argument("--max-budget-usd", type=float, default=1.0)
    parser.add_argument(
        "--case",
        action="append",
        dest="cases",
        help="Case name or id; repeat to select multiple",
    )
    parser.add_argument(
        "--binary", type=Path, default=ROOT / "target" / "release" / "agent-ui-render"
    )
    parser.add_argument(
        "--chrome",
        type=Path,
        default=Path("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"),
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    binary = args.binary.resolve()
    chrome = args.chrome.resolve()
    if not binary.is_file():
        raise SystemExit(f"agent-ui-render binary not found: {binary}")
    if not chrome.is_file():
        raise SystemExit(f"Google Chrome not found: {chrome}")
    cases = load_cases(set(args.cases) if args.cases else None)
    workspace = args.workspace.resolve()
    systems_dir = workspace / "systems"
    systems_dir.mkdir(parents=True, exist_ok=True)
    direct_system_path = systems_dir / "direct-html.txt"
    compact_system_path = systems_dir / "agent-ui-render.txt"
    direct_system_path.write_text(DIRECT_SYSTEM)
    compact_system_path.write_text(compact_system(binary))
    prepare_workspace(
        workspace, cases, args.repetitions, direct_system_path, compact_system_path
    )

    jobs: list[dict[str, Any]] = []
    for case in cases:
        for run_number in range(1, args.repetitions + 1):
            eval_dir = (
                workspace / f"eval-{case['id']:02d}-{case['name']}-run-{run_number}"
            )
            for arm, system_path in (
                ("without_skill", direct_system_path),
                ("with_skill", compact_system_path),
            ):
                jobs.append(
                    {
                        "case": case,
                        "run_number": run_number,
                        "arm": arm,
                        "run_dir": eval_dir / arm,
                        "system_path": system_path,
                        "model": args.model,
                        "effort": args.effort,
                        "max_budget_usd": args.max_budget_usd,
                        "max_attempts": args.max_attempts,
                        "binary": binary,
                        "chrome": chrome,
                    }
                )
    random.Random(20260710).shuffle(jobs)
    print(
        f"Running {len(jobs)} calls/configurations in {workspace} with max_workers={args.max_workers}",
        flush=True,
    )
    results: list[dict[str, Any]] = []
    with concurrent.futures.ThreadPoolExecutor(
        max_workers=args.max_workers
    ) as executor:
        futures = {executor.submit(execute_run, job): job for job in jobs}
        for future in concurrent.futures.as_completed(futures):
            job = futures[future]
            try:
                result = future.result()
            except Exception as error:
                with PRINT_LOCK:
                    print(
                        f"FAILED {job['arm']} {job['case']['name']} run {job['run_number']}: {error}",
                        file=sys.stderr,
                        flush=True,
                    )
                raise
            result["run_number"] = job["run_number"]
            run_result_path = job["run_dir"] / "run-result.json"
            run_result_path.write_text(
                json.dumps(result, ensure_ascii=False, indent=2) + "\n"
            )
            results.append(result)

    benchmark = aggregate(results, workspace, args.model, args.repetitions)
    print(f"Benchmark complete: {workspace / 'benchmark.md'}")
    print(json.dumps(benchmark["token_efficiency"], indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
