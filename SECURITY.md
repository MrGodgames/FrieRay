# Security Policy

FrieRay is a local VPN/proxy client. It handles sensitive data and can change local network settings, so security reports are taken seriously.

## Supported versions

Security fixes are currently applied to the latest released version only.

| Version | Supported |
| --- | --- |
| Latest release | Yes |
| Older releases | No |

## Reporting a vulnerability

Please do not open a public issue for vulnerabilities or reports that include private connection details.

Use GitHub private vulnerability reporting if it is enabled for this repository. If it is not available, contact the maintainer privately through GitHub and share only the minimum information needed to start triage.

Please include:

- affected FrieRay version or commit
- macOS version and CPU architecture
- clear reproduction steps
- impact description
- whether the issue requires a malicious subscription, local access, or network access

Do not include real subscription URLs, server UUIDs, passwords, access tokens, or full generated Xray configs. Use redacted examples instead.

## Sensitive data handled by FrieRay

FrieRay may handle:

- subscription URLs and provider tokens
- server UUIDs, passwords, and protocol secrets
- generated Xray configuration files
- local runtime settings
- system proxy settings
- TUN helper installation and route configuration

## Current protections

- Remote subscription URLs must use HTTPS.
- Local runtime JSON files are written with private Unix permissions where supported.
- Generated Xray configs and temporary speed-test configs are written as private files.
- Subscription URLs are redacted in logs.
- The Tauri shell plugin is not exposed to the frontend.
- A Content Security Policy is configured for the Tauri webview.

## Known hardening work

The following items are known and tracked as future hardening work:

- Store server secrets in macOS Keychain instead of plaintext JSON.
- Verify downloaded `tun2socks` binaries with pinned checksums.
- Add a safe uninstall command for the privileged TUN helper and sudoers entry.
- Reduce CSP inline-style requirements.
- Improve release signing and update verification.
