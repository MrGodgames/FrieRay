# Roadmap

This roadmap describes the near-term direction for FrieRay. Items may change as the project evolves.

## Security hardening

- Store server secrets in macOS Keychain instead of plaintext JSON.
- Verify downloaded `tun2socks` binaries with pinned checksums.
- Improve privileged TUN helper lifecycle and add a safe uninstall command.
- Reduce CSP inline-style requirements.
- Improve release signing and update verification.

## Reliability

- Expand parser tests for provider-specific subscription edge cases.
- Add more connection validation scenarios.
- Improve recovery when system proxy or TUN setup fails.
- Add more diagnostics for Xray startup and outbound failures.

## Protocol and import support

- Improve Xray/V2Ray JSON compatibility.
- Evaluate Clash YAML import support.
- Evaluate TUIC, Hysteria, and WireGuard support.

## User experience

- Continue improving the macOS menu bar workflow.
- Improve routing and split tunnel UX.
- Add clearer empty states and troubleshooting hints.
- Improve accessibility and keyboard navigation.

## Release engineering

- Keep CI green for frontend build, Rust tests/checks, and dependency audit.
- Document bundled binary versions and checksums for each release.
- Improve reproducibility of release builds.
