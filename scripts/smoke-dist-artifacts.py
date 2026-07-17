#!/usr/bin/env python3
"""Validate cargo-dist archives and installer scripts before publication."""

from __future__ import annotations

import argparse
import io
import json
import logging
import os
import platform
import shutil
import stat
import subprocess
import tarfile
import tempfile
import zipfile
from pathlib import Path, PurePosixPath

LOGGER = logging.getLogger("dist-artifact-smoke")
ARCHIVE_SUFFIXES = (".tar.gz", ".tar.xz", ".zip")
TARGETS = (
    "aarch64-apple-darwin",
    "aarch64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "x86_64-pc-windows-msvc",
    "x86_64-unknown-linux-gnu",
)


class ArtifactError(RuntimeError):
    """Raised when a release artifact fails its smoke check."""


def artifact_paths(path_list: Path) -> list[Path]:
    distribution_root = (Path.cwd() / "target" / "distrib").resolve()
    paths: list[Path] = []
    for line in path_list.read_text().splitlines():
        if not line.strip():
            continue
        path = Path(line).resolve(strict=True)
        if not path.is_relative_to(distribution_root):
            raise ArtifactError(f"artifact is outside target/distrib: {path}")
        paths.append(path)
    if not paths:
        raise ArtifactError(f"artifact path list is empty: {path_list}")
    return paths


def is_archive(path: Path) -> bool:
    return path.name.endswith(ARCHIVE_SUFFIXES)


def safe_member_name(name: str) -> PurePosixPath:
    normalized = PurePosixPath(name.replace("\\", "/"))
    if normalized.is_absolute() or ".." in normalized.parts:
        raise ArtifactError(f"archive contains unsafe path: {name!r}")
    return normalized


def expected_binary_name(archive: Path) -> str:
    return "agent-ui-render.exe" if "windows" in archive.name else "agent-ui-render"


def binary_from_zip(archive: Path) -> tuple[bytes, int]:
    expected = expected_binary_name(archive)
    found: list[tuple[bytes, int]] = []
    with zipfile.ZipFile(archive) as package:
        for member in package.infolist():
            path = safe_member_name(member.filename)
            mode = member.external_attr >> 16
            if stat.S_IFMT(mode) == stat.S_IFLNK:
                raise ArtifactError(f"archive contains unexpected symlink: {member.filename}")
            if not member.is_dir() and path.name == expected:
                found.append((package.read(member), mode))
    if len(found) != 1:
        raise ArtifactError(f"{archive} should contain exactly one {expected}, found {len(found)}")
    return found[0]


def binary_from_tar(archive: Path) -> tuple[bytes, int]:
    expected = expected_binary_name(archive)
    found: list[tuple[bytes, int]] = []
    archive_data = archive.read_bytes()
    with tarfile.open(fileobj=io.BytesIO(archive_data), mode="r:*") as package:
        for member in package.getmembers():
            path = safe_member_name(member.name)
            if member.issym() or member.islnk():
                raise ArtifactError(f"archive contains unexpected link: {member.name}")
            if member.isfile() and path.name == expected:
                source = package.extractfile(member)
                if source is None:
                    raise ArtifactError(f"could not read binary from {archive}")
                found.append((source.read(), member.mode))
    if len(found) != 1:
        raise ArtifactError(f"{archive} should contain exactly one {expected}, found {len(found)}")
    return found[0]


def validate_archive_members(archive: Path) -> None:
    if archive.suffix == ".zip":
        with zipfile.ZipFile(archive) as package:
            for member in package.infolist():
                safe_member_name(member.filename)
                mode = member.external_attr >> 16
                if stat.S_IFMT(mode) == stat.S_IFLNK:
                    raise ArtifactError(
                        f"archive contains unexpected symlink: {member.filename}"
                    )
        return
    archive_data = archive.read_bytes()
    with tarfile.open(fileobj=io.BytesIO(archive_data), mode="r:*") as package:
        for member in package.getmembers():
            safe_member_name(member.name)
            if member.issym() or member.islnk():
                raise ArtifactError(f"archive contains unexpected link: {member.name}")


def archive_target(archive: Path) -> str | None:
    return next((target for target in TARGETS if target in archive.name), None)


def native_target() -> str | None:
    machine = platform.machine().lower()
    architecture = {
        "amd64": "x86_64",
        "x86_64": "x86_64",
        "arm64": "aarch64",
        "aarch64": "aarch64",
    }.get(machine)
    system = platform.system()
    suffix = {
        "Darwin": "apple-darwin",
        "Linux": "unknown-linux-gnu",
        "Windows": "pc-windows-msvc",
    }.get(system)
    return f"{architecture}-{suffix}" if architecture and suffix else None


def execute_binary(data: bytes, mode: int, windows: bool) -> None:
    with tempfile.TemporaryDirectory(prefix="agent-ui-dist-smoke-") as directory:
        with tempfile.NamedTemporaryFile(
            dir=directory,
            prefix="agent-ui-render-",
            suffix=".exe" if windows else "",
            delete=False,
        ) as output:
            output.write(data)
            binary = Path(output.name)
        if not windows:
            binary.chmod(mode | stat.S_IXUSR)
        version = subprocess.run(
            [str(binary), "--version"],
            check=True,
            capture_output=True,
            text=True,
            timeout=30,
        )
        if "agent-ui-render" not in version.stdout:
            raise ArtifactError(f"unexpected --version output: {version.stdout!r}")
        schema = subprocess.run(
            [str(binary), "schema", "print", "config"],
            check=True,
            capture_output=True,
            text=True,
            timeout=30,
        )
        document = json.loads(schema.stdout)
        if document.get("$schema") != "https://json-schema.org/draft/2020-12/schema":
            raise ArtifactError("archive binary returned an unexpected config schema")


def smoke_archives(paths: list[Path], execute_native: bool = True) -> None:
    archives = [path for path in paths if is_archive(path)]
    if not archives:
        raise ArtifactError("cargo-dist produced no archive artifacts")
    current_target = native_target()
    executed = 0
    for archive in archives:
        if not archive.is_file():
            raise ArtifactError(f"archive does not exist: {archive}")
        if archive.suffix == ".zip":
            data, mode = binary_from_zip(archive)
        else:
            data, mode = binary_from_tar(archive)
        target = archive_target(archive)
        if execute_native and target == current_target:
            execute_binary(data, mode, windows="windows" in archive.name)
            executed += 1
        LOGGER.info("archive smoke OK: %s (%s)", archive, target or "unknown target")
    LOGGER.info("archive summary: %d checked, %d executed natively", len(archives), executed)


def parse_powershell(path: Path, require_powershell: bool) -> None:
    powershell = shutil.which("pwsh")
    if powershell is None:
        if require_powershell:
            raise ArtifactError("pwsh is required to parse the PowerShell installer")
        LOGGER.info("PowerShell parser unavailable; structural check only: %s", path)
        return
    parser = (
        "$tokens = $null; $parseErrors = $null; "
        "[void][System.Management.Automation.Language.Parser]::ParseFile("
        "$env:AGENT_UI_INSTALLER_PATH, [ref]$tokens, [ref]$parseErrors); "
        "if ($parseErrors.Count -gt 0) { $parseErrors | Out-String | Write-Error; exit 1 }"
    )
    environment = os.environ.copy()
    environment["AGENT_UI_INSTALLER_PATH"] = str(path)
    subprocess.run(
        [powershell, "-NoProfile", "-NonInteractive", "-Command", parser],
        check=True,
        env=environment,
        timeout=30,
    )


def smoke_installers(paths: list[Path], require_powershell: bool = False) -> None:
    source_archives = [path for path in paths if is_archive(path)]
    shell_installers = [path for path in paths if path.suffix == ".sh"]
    powershell_installers = [path for path in paths if path.suffix == ".ps1"]
    if not source_archives:
        raise ArtifactError("cargo-dist produced no global source archive")
    if not shell_installers or not powershell_installers:
        raise ArtifactError("cargo-dist must produce both shell and PowerShell installers")
    for path in source_archives:
        validate_archive_members(path)
        LOGGER.info("source archive paths OK: %s", path)
    for path in shell_installers:
        source = path.read_text()
        if "agent-ui-render" not in source:
            raise ArtifactError(f"shell installer does not identify the package: {path}")
        subprocess.run(["bash", "-n", str(path)], check=True, timeout=30)
        LOGGER.info("shell installer syntax OK: %s", path)
    for path in powershell_installers:
        source = path.read_text(encoding="utf-8-sig")
        if "agent-ui-render" not in source:
            raise ArtifactError(f"PowerShell installer does not identify the package: {path}")
        parse_powershell(path, require_powershell)
        LOGGER.info("PowerShell installer syntax OK: %s", path)


def self_test() -> None:
    with tempfile.TemporaryDirectory(prefix="agent-ui-dist-self-test-") as directory:
        root = Path(directory)
        payload = b"fake binary"
        with tempfile.NamedTemporaryFile(
            dir=root,
            prefix="agent-ui-render-x86_64-unknown-linux-gnu-",
            suffix=".tar.xz",
            delete=False,
        ) as output:
            tar_path = Path(output.name)
            with tarfile.open(fileobj=output, mode="w:xz") as package:
                member = tarfile.TarInfo("agent-ui-render/agent-ui-render")
                member.mode = 0o755
                member.size = len(payload)
                package.addfile(member, io.BytesIO(payload))
        zip_path = root / "agent-ui-render-x86_64-pc-windows-msvc.zip"
        with zipfile.ZipFile(zip_path, mode="w") as package:
            package.writestr("agent-ui-render/agent-ui-render.exe", payload)
        smoke_archives([tar_path, zip_path], execute_native=False)

        unsafe = root / "unsafe.zip"
        with zipfile.ZipFile(unsafe, mode="w") as package:
            package.writestr("../agent-ui-render", payload)
        try:
            binary_from_zip(unsafe)
        except ArtifactError:
            pass
        else:
            raise ArtifactError("unsafe archive path should have been rejected")

        shell = root / "agent-ui-render-installer.sh"
        shell.write_text("#!/usr/bin/env bash\n# agent-ui-render\nexit 0\n")
        powershell = root / "agent-ui-render-installer.ps1"
        powershell.write_text("# agent-ui-render\nexit 0\n")
        smoke_installers([tar_path, shell, powershell])
    LOGGER.info("dist artifact smoke self-test OK")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    archives = subparsers.add_parser("archives")
    archives.add_argument("paths_file", type=Path)
    installers = subparsers.add_parser("installers")
    installers.add_argument("paths_file", type=Path)
    installers.add_argument("--require-powershell", action="store_true")
    subparsers.add_parser("self-test")
    arguments = parser.parse_args()

    if arguments.command == "archives":
        smoke_archives(artifact_paths(arguments.paths_file))
    elif arguments.command == "installers":
        smoke_installers(
            artifact_paths(arguments.paths_file),
            require_powershell=arguments.require_powershell,
        )
    else:
        self_test()


if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO, format="%(message)s")
    try:
        main()
    except (ArtifactError, OSError, subprocess.SubprocessError, json.JSONDecodeError) as error:
        LOGGER.error("dist artifact smoke failed: %s", error)
        raise SystemExit(1) from error
