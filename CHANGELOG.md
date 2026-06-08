# Changelog

## v0.2.2 - Scan Accuracy and Security Patch

Release type: patch

This release improves server scan accuracy, speed-test compatibility, and baseline application security.

### Added

- Protocol-aware Xray outbound generation for VMess, Trojan, and Shadowsocks during connection and speed testing.
- VMess `alterId` parsing from subscription links and JSON configs.
- Tests for protocol-specific outbound generation.
- HTTPS validation for remote subscription URLs.
- Tests for subscription URL validation.

### Improved

- Ping checks now measure multiple resolved IP addresses in parallel.
- Speed tests now wait for temporary Xray SOCKS readiness instead of sleeping for a fixed startup delay.
- Speed tests can finish earlier when the first successful samples are stable.
- Temporary speed-test Xray configs are written to a private app temp directory.
- Local runtime JSON files are written with private Unix permissions where supported.
- Main Xray config files are written with private Unix permissions where supported.
- Subscription URLs are redacted in logs to avoid leaking provider tokens.
- Direct `ss://` links are now handled by the direct-link import path.
- README was rewritten with clearer GitHub release, security, development, and status sections.

### Security

- Removed unused Tauri shell plugin registration.
- Removed broad frontend shell capabilities from Tauri permissions.
- Enabled a Content Security Policy for the Tauri webview.
- Removed disabled TLS certificate validation from subscription fetching.
- Remote subscription URLs now require HTTPS, with local HTTP still allowed for development.
- Removed fallback loading of an arbitrary `xray` binary from the system `PATH`.
- Removed third-party proxy mirrors from the `tun2socks` download flow.
- TUN helper install now uses unique temporary filenames instead of predictable `/tmp` paths.

### Known Follow-Ups

- Move server secrets from plaintext JSON to macOS Keychain.
- Add pinned checksum verification for downloaded `tun2socks` binaries.
- Add a safe uninstall command for the privileged TUN helper and sudoers entry.
- Reduce CSP inline-style requirements.
- Add automated dependency audit checks in CI.

### Verification

- `cargo test`
- `cargo check`
- `npm run build`
