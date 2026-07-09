# Release Process

This document describes the intended release checklist for FrieRay maintainers.

## Before release

1. Confirm the version is updated consistently:
   - `package.json`
   - `package-lock.json`
   - `src-tauri/Cargo.toml`
   - `src-tauri/tauri.conf.json`
   - README release section and visible version text, if changed
2. Run verification:

   ```bash
   npm audit --audit-level=moderate
   npm run build
   cd src-tauri
   cargo test
   cargo check
   ```

3. If bundled binaries changed, record upstream source, version, and checksum in the release notes.
4. Review `SECURITY.md` and `THIRD_PARTY_NOTICES.md` for stale information.
5. Update `CHANGELOG.md`.

## Build artifacts

Build the desktop app:

```bash
npm run tauri build
```

Expected macOS artifacts are written under:

```text
src-tauri/target/release/bundle/macos/
src-tauri/target/release/bundle/dmg/
```

## GitHub release

1. Create a tag such as `v0.2.5`.
2. Publish a GitHub release with:
   - summary of changes
   - security notes, if any
   - verification commands run
   - DMG artifact
   - artifact checksum
3. Mark prereleases clearly when the build is experimental.
