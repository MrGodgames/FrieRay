# Roadmap

FrieRay is moving from a macOS-only personal proxy client toward a cross-platform open-source V2Ray/Xray desktop app with practical per-application VPN control.

The roadmap is intentionally public: it shows where contributors can help and what the project needs before wider adoption.

## Now: open-source foundation

- Keep the repository ready for contributors: CI, issue templates, security policy, release notes, and clear documentation.
- Maintain a green baseline for `npm run build`, `cargo test`, `cargo check`, and `npm audit --audit-level=moderate`.
- Document bundled binary sources, versions, and checksums for Xray and `tun2socks`.
- Add English UI support and keep Russian/English strings in sync.
- Add screenshots and short demos for the main workflow.

## Cross-platform desktop support

### Linux port

- Audit all macOS-specific code paths and isolate platform adapters.
- Add Linux system proxy support for common desktop environments where possible.
- Evaluate Linux TUN setup flow and required privileges.
- Package Linux builds as AppImage and/or `.deb` once the runtime path is stable.
- Verify Xray lifecycle, config paths, autostart behavior, and tray integration on Linux.

### Windows port

- Audit process management, config paths, and cleanup for Windows.
- Add Windows system proxy support.
- Evaluate Wintun/TUN integration and privilege requirements.
- Package Windows builds as `.msi` or `.exe` installer.
- Verify tray workflow, autostart, proxy cleanup, and Xray lifecycle on Windows.

## Per-application VPN filtering

Goal: let users decide which applications should use the VPN/proxy instead of forcing all traffic through one mode.

Planned modes:

- **Allow list**: only selected apps use FrieRay.
- **Block list**: every app uses FrieRay except selected apps.
- **Direct rules**: selected domains/IP ranges bypass the proxy.
- **Proxy rules**: selected domains/IP ranges always use the proxy.

Research and implementation tasks:

- Detect installed applications per platform.
- Store user app selections in settings.
- Map app identity to traffic rules reliably on macOS, Linux, and Windows.
- Integrate app filtering with existing system proxy and TUN modes.
- Show clear warnings when a platform cannot guarantee per-app routing.
- Add tests for rule generation and settings persistence.

## Security hardening

- Store server secrets in macOS Keychain first, then Linux Secret Service/KWallet and Windows Credential Manager.
- Verify downloaded `tun2socks` binaries with pinned checksums.
- Improve privileged TUN helper lifecycle and add a safe uninstall command.
- Reduce CSP inline-style requirements.
- Improve release signing and update verification.
- Avoid logging subscription URLs, credentials, UUIDs, generated configs, or app-routing rules that expose private usage.

## Reliability

- Expand parser tests for provider-specific subscription edge cases.
- Add more connection validation scenarios.
- Improve recovery when system proxy or TUN setup fails.
- Add more diagnostics for Xray startup and outbound failures.
- Add cleanup tests for proxy/TUN/Xray shutdown paths.

## Protocol and import support

- Improve Xray/V2Ray JSON compatibility.
- Add Clash YAML import support after parser coverage is strong enough.
- Evaluate TUIC, Hysteria, and WireGuard support.
- Keep protocol-specific config generation covered by Rust tests.

## User experience

- Continue improving the macOS menu bar workflow.
- Make the app-filtering workflow understandable for non-technical users.
- Add clearer empty states and troubleshooting hints.
- Improve accessibility and keyboard navigation.
- Add onboarding for first subscription import and first connection.
- Add English screenshots and documentation for the public README.

## Release engineering

- Keep CI green for frontend build, Rust tests/checks, and dependency audit.
- Add cross-platform CI jobs when Linux and Windows support lands.
- Document bundled binary versions and checksums for each release.
- Improve reproducibility of release builds.
- Add signed release artifacts when the signing workflow is ready.
