# Contributing to FrieRay

Thank you for your interest in FrieRay. This project is a macOS desktop V2Ray/Xray client built with Tauri 2, React, and Rust.

## Before you start

- Check existing issues and pull requests to avoid duplicate work.
- Keep pull requests focused on one bug fix or feature.
- Do not include real subscription URLs, server UUIDs, passwords, access tokens, generated Xray configs, or other private connection data in issues, tests, screenshots, or logs.

## Development setup

Requirements:

- macOS
- Node.js 20+
- npm
- Rust toolchain
- Xcode Command Line Tools

Install dependencies:

```bash
npm install
```

Run the app in development mode:

```bash
npm run tauri dev
```

Build the frontend:

```bash
npm run build
```

Run Rust checks:

```bash
cd src-tauri
cargo test
cargo check
```

## Project structure

```text
src/                         React frontend
src/api/tauri.js             Frontend wrappers for Tauri commands
src/pages/                   App pages and tray popup UI
src/components/              Shared React components
src-tauri/src/commands/      Tauri command handlers
src-tauri/src/core/          Xray, proxy, TUN, config, and tray logic
src-tauri/src/models/        Rust data models
src-tauri/src/utils/         Storage, parsing, logging utilities
```

## Tauri command changes

When adding or changing a Tauri command, update the complete bridge:

1. Rust implementation in `src-tauri/src/commands/` or `src-tauri/src/core/`.
2. Command registration in `src-tauri/src/lib.rs`.
3. Frontend wrapper in `src/api/tauri.js`.
4. React caller in `src/pages/` or `src/components/`.

## Security-sensitive areas

Be extra careful when touching:

- subscription fetching and parsing
- generated Xray configs
- local storage files
- system proxy settings
- TUN mode and privileged helper code
- bundled third-party binaries

Expected baseline:

- Do not log full subscription URLs or credentials.
- Preserve HTTPS-only remote subscription behavior unless there is a clear reason to change it.
- Keep generated configs and local runtime data private where supported.
- Avoid widening Tauri frontend permissions.
- Preserve or tighten the Content Security Policy.

## Pull request checklist

Before opening a pull request, run:

```bash
npm run build
cd src-tauri
cargo test
cargo check
```

In the PR description, include:

- what changed
- why it changed
- how it was tested
- screenshots or screen recordings for UI changes, if useful

## Code style

- Match the existing style of nearby files.
- Prefer small, explicit changes over broad refactors.
- Keep user-facing text clear and practical.
- Add or update tests when changing parsers, config generation, connection validation, or security-sensitive behavior.
