# TEpub-Editor Project Log

## Maintenance Rule

- Before making code or documentation changes in this repository, read this file first.
- After each completed change, append a concise entry under "Change History".
- Keep entries factual: date/time, request, touched areas, verification, and known caveats.
- Do not use this file for secrets, credentials, or machine-specific private data.

## Project Outline

TEpub-Editor is a Tauri + SvelteKit desktop editor for TXT/EPUB workflows.

Main areas:

- `src/routes/+page.svelte`: library/home page, book list/grid, library settings, metadata editing.
- `src/routes/editor/+page.svelte`: TXT editor page, table of contents, EPUB export, settings, content checks, proofreading panel.
- `src/routes/reader/+page.svelte`: reader page.
- `src/lib/Editor.svelte`: CodeMirror wrapper used by TXT editing flows.
- `src/lib/textProofing.ts`: TXT proofreading and text-cleanup logic.
- `src-tauri/src/lib.rs`: Tauri backend commands and EPUB/TXT processing.
- `src-tauri/tauri.conf.json`: Tauri app configuration.

## Current Known Validation Baseline

- `pnpm exec tsc --noEmit --pretty false` passes after the proofreading panel changes.
- `pnpm build` passes.
- `pnpm check` still fails because of existing issues outside the proofreading work:
  - `src/lib/ReaderTocNode.svelte:12`: `export interface` placement.
  - `src/routes/+page.svelte:1311-1312`: nullable `selectedBook` handler typing.
  - Several existing Svelte accessibility warnings in library/reader pages.

## Change History

### 2026-05-12 07:34 +08:00

Request: bump the app to version 0.5.0, build locally, push updates to GitHub, and trigger GitHub Actions.

Changes:

- Updated app version from `0.4.9` to `0.5.0` in:
  - `package.json`
  - `src-tauri/tauri.conf.json`
  - `src-tauri/Cargo.toml`
  - `src-tauri/Cargo.lock`
- Prepared release commit and `v0.5.0` tag for GitHub Actions.

Verification:

- `pnpm tauri build` completed successfully.
- Local release artifacts were produced:
  - `src-tauri/target/release/bundle/msi/TEpub-Editor_0.5.0_x64_zh-CN.msi`
  - `src-tauri/target/release/bundle/nsis/TEpub-Editor_0.5.0_x64-setup.exe`

Caveats:

- Build still emits existing Svelte accessibility warnings.
- Tauri still warns that bundle identifier `com.tepubeditor.app` ends with `.app`.
- `README.md`, `.claude/`, and `AGENTS.md` were left out of the release commit because they were pre-existing unrelated working-tree changes.

### 2026-05-12 07:30 +08:00

Request: create a root file to store the project outline and all change records, and use it as a pre/post modification log.

Changes:

- Added `PROJECT_LOG.md`.
- Documented the repository maintenance rule: read this file before modifying, append an entry after modifying.
- Captured the current project outline and validation baseline.

Verification:

- Confirmed `PROJECT_LOG.md` did not exist before creation.

### 2026-05-12 07:26 +08:00

Request: refine the TXT editor proofreading panel.

Changes:

- Added a proofreading button to the TXT editor toolbar using an icon instead of visible text.
- Added a proofreading panel with tabs for directory title rewrite, built-in regex cleanup, and simplified/traditional conversion.
- Implemented directory title rewrite preview with normalized chapter/volume titles and scrollable two-column preview.
- Removed line-number display from title rewrite preview.
- Merged the previous quick cleanup and regex pages into the built-in regex tab.
- Changed built-in cleanup actions to preview matches first, with selectable rows, "replace selected", and "replace all".
- Removed line-number prefixes from built-in regex match display.
- Added click-to-locate behavior for built-in regex match content.
- Kept the checkbox column for selecting replacement targets.
- Added full-text simplified/traditional conversion via `opencc-js`, loaded lazily.
- Added `replaceAllContent` to `src/lib/Editor.svelte` so proofing changes go through a CodeMirror transaction and remain undoable.
- Adjusted the proofing header height to match the left TOC header height.

Touched files:

- `package.json`
- `pnpm-lock.yaml`
- `src/lib/Editor.svelte`
- `src/lib/textProofing.ts`
- `src/routes/editor/+page.svelte`

Verification:

- `pnpm exec tsc --noEmit --pretty false` passed.
- `pnpm build` passed.
- `pnpm tauri dev` was started successfully; `/editor` returned `200`.

Caveats:

- Existing `pnpm check` failures remain outside this feature area.
- Existing unrelated working-tree items were present and not modified: `README.md`, `.claude/`, `AGENTS.md`.
