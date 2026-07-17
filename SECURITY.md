# Security Policy

## Supported versions

Security fixes are provided for the latest published release. Upgrade to the
latest release before reporting behavior that may already be fixed.

## Reporting a vulnerability

Do not disclose suspected vulnerabilities in a public issue, discussion, or
pull request.

Use GitHub's private vulnerability reporting flow for this repository:

1. Open the repository **Security** tab.
2. Choose **Report a vulnerability**.
3. Include the affected version, operating system, minimal reproduction,
   expected security boundary, and observed impact.

If private reporting is unavailable, open a public issue containing only a
request for a private contact channel; do not include exploit details.

Reports involving payload validation, Markdown links, generated HTML, Vega
resource loading, installer integrity, or resource exhaustion are in scope.
Arbitrary UI code execution is outside the supported payload contract and should
already be rejected as described in `docs/security-model.md`.

## Response expectations

Maintainers will acknowledge a complete report when practical, reproduce it
privately, assess affected releases, and coordinate disclosure with the
reporter. A fix may include a patched release, updated checksums and
attestations, and documentation for required mitigations.

## Release verification

Release archives include checksums and GitHub build-provenance attestations;
the release also publishes `agent-ui-render.spdx.json` as its software bill of
materials. After downloading an archive, verify the checksum and provenance
before installation:

```bash
gh attestation verify <archive> --repo NeoHsu/agent-ui-render
```

Use the checksum file published with the same GitHub Release and compare it with
your platform's SHA-256 tool.
