# Third-Party Notices

FrieRay bundles and/or uses third-party software. This file documents the most important runtime components that are shipped with the app or are central to its operation.

## Xray-core

FrieRay uses Xray as the local proxy engine.

Tracked bundled files:

```text
src-tauri/binaries/xray
src-tauri/binaries/xray-aarch64-apple-darwin
```

Upstream project:

- https://github.com/XTLS/Xray-core

Before updating bundled Xray binaries:

1. Download the binary from the official upstream release source.
2. Record the upstream version and checksum in the release notes or pull request.
3. Test at least VLESS, VMess, Trojan, and Shadowsocks config generation.
4. Run `npm run build`, `cargo test`, and `cargo check`.

## tun2socks

FrieRay uses `tun2socks` for macOS TUN mode.

Tracked bundled file:

```text
src-tauri/binaries/tun2socks.gz
```

Before updating bundled `tun2socks` binaries:

1. Download from a trusted upstream release source.
2. Verify and record the checksum.
3. Test TUN helper install/start/stop behavior on macOS.
4. Confirm cleanup restores routes and proxy settings.

## JavaScript and Rust dependencies

JavaScript dependencies are declared in `package.json` and locked in `package-lock.json`.

Rust dependencies are declared in `src-tauri/Cargo.toml` and locked in `src-tauri/Cargo.lock`.

Recommended checks before release:

```bash
npm audit --audit-level=moderate
cd src-tauri
cargo test
cargo check
```

If `cargo-audit` is installed, also run:

```bash
cd src-tauri
cargo audit
```
