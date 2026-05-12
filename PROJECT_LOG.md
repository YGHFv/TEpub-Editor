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

### 2026-05-12 19:20 +08:00

Request: fix TXT editor right-click title actions so they do not add or remove numbering, prevent the custom title menu from appearing on EPUB creation form fields, then release version 0.5.3.

Changes:

- Changed TXT editor right-click title actions to store manual TOC overrides instead of rewriting the clicked line text.
- `Set chapter title` and `Set volume title` now keep the original line content unchanged.
- `Cancel title` now only ignores that line in the TOC; it does not remove existing numbering or title text.
- EPUB export now uses the same merged manual TOC overrides as the editor sidebar.
- Prevented the custom editor context menu from opening on normal input and textarea controls.
- Bumped app version from 0.5.2 to 0.5.3.

Touched files:

- `src/routes/editor/+page.svelte`
- `src/lib/ContextMenu.svelte`
- `package.json`
- `src-tauri/tauri.conf.json`
- `src-tauri/Cargo.toml`
- `src-tauri/Cargo.lock`
- `PROJECT_LOG.md`

Verification:

- `pnpm build` passed before the version bump.
- `cargo check` finished successfully before the version bump.
- `pnpm build` passed after the version bump.
- `cargo check` finished successfully after the version bump.
- `pnpm tauri build` completed and produced MSI/NSIS release bundles for 0.5.3.
- Version 0.5.3 changes are prepared for GitHub push and release workflow trigger.

### 2026-05-12 19:08 +08:00

Request: fix the EPUB creation modal becoming mostly blank after the previous footer/search-result layout change.

Changes:

- Removed the fixed height from the EPUB creation modal shell.
- Restored modal body auto-sizing when no cover search results are visible.
- Kept cover search results internally scrollable by limiting only the results panel height.

Touched files:

- `src/routes/editor/+page.svelte`
- `PROJECT_LOG.md`

Verification:

- `pnpm build` passed.
- Running dev server received the Svelte style HMR update.

### 2026-05-12 19:06 +08:00

Request: fix cover-search thumbnails being compressed again and prevent old search results from appearing when starting EPUB creation.

Changes:

- Cleared cover search results when the EPUB creation modal is opened.
- Cleared cover search results when EPUB generation starts.
- Changed cover result grid rows to natural content height.
- Replaced result thumbnail aspect-ratio sizing with a fixed image area height so internal scrolling cannot squash images.

Touched files:

- `src/routes/editor/+page.svelte`
- `PROJECT_LOG.md`

Verification:

- `pnpm build` passed.
- Running dev server received Svelte HMR updates.

### 2026-05-12 19:01 +08:00

Request: adjust the EPUB creation cover preview/search layout so the Advanced Options and Start buttons remain visible when cover search results are shown.

Changes:

- Set the EPUB creation modal to a bounded height and made its body use an internal flex layout.
- Reduced the cover preview height and switched its image fitting to contained display.
- Made the cover-search results panel consume the remaining body space and scroll internally.
- Kept the footer action row fixed within the modal body so Advanced Options and Start Creation remain visible.

Touched files:

- `src/routes/editor/+page.svelte`
- `PROJECT_LOG.md`

Verification:

- `pnpm build` passed.
- Running dev server received the Svelte style HMR update.

### 2026-05-12 18:30 +08:00

Request: automatically collapse cover search results after the user clicks a result and applies it.

Changes:

- Cleared `coverSearchResults` after a manually selected remote cover is downloaded and applied.
- Kept the success status message visible so the user still gets feedback after the result panel closes.

Touched files:

- `src/routes/editor/+page.svelte`
- `PROJECT_LOG.md`

Verification:

- `pnpm build` passed.

### 2026-05-12 18:24 +08:00

Request: address six TXT/EPUB workflow issues, fix the cover-search thumbnail aspect ratio after visual review, and restart the development environment.

Changes:

- Changed EPUB cover search so the search button only refreshes candidates; it no longer auto-applies a preferred source.
- Removed the preferred-source badge from cover search results.
- Fixed cover result cards so global button styles no longer compress thumbnails into horizontal strips; thumbnails now keep a book-cover ratio and use contained image fitting.
- Restricted title rewrite/reorder selection to explicit numbered titles:
  - volumes: explicit numbered volume headings only
  - chapters: explicit numbered chapter headings or numeric-only headings only
- Added TXT editor right-click actions for setting a line as a chapter title, setting it as a volume title, or removing title numbering.
- Added library settings for whether opening TXT/EPUB editors hides the library window, and whether closing the TXT editor returns to the library or exits the app.
- Changed TXT editor close handling so closing a child editor window no longer exits the whole app by default.
- Added EPUB creation status feedback and post-build actions to open the generated EPUB in the EPUB editor or reveal it in Explorer.
- Updated title-bracket cleanup to preserve trivial bracket contents such as up/down markers, Arabic digits, or the first four Chinese numerals.
- Restarted the Tauri dev environment after clearing the stale processes using port `1420`.

Touched files:

- `src/routes/editor/+page.svelte`
- `src/routes/+page.svelte`
- `src/lib/Editor.svelte`
- `src/lib/ContextMenu.svelte`
- `src/lib/textProofing.ts`
- `src-tauri/src/lib.rs`
- `PROJECT_LOG.md`

Verification:

- `pnpm build` passed.
- `cargo check` passed.
- `pnpm tauri dev` was restarted successfully; Vite is serving `http://localhost:1420/`.

Caveats:

- `pnpm check` still reports existing unrelated Svelte/type issues and accessibility warnings.
- `.codex-dev.log` was created as a local dev-server log and is not part of the source changes.
- Existing untracked `.claude/` and `AGENTS.md` remain untouched.

### 2026-05-12 13:31 +08:00

Request: fix the EPUB creation modal layout regression, bump the app to version `0.5.2`, build locally, and push the update to GitHub Actions.

Changes:

- Fixed the EPUB creation modal shell width so the wider cover-search layout is applied to the outer modal instead of being clipped by the default 520px shell.
- Added modal overflow constraints so long metadata/search-result content stays inside the dialog.
- Tightened EPUB textarea and row sizing to prevent form fields from pushing the cover column out of view.
- Updated app version from `0.5.1` to `0.5.2` in:
  - `package.json`
  - `src-tauri/tauri.conf.json`
  - `src-tauri/Cargo.toml`
  - `src-tauri/Cargo.lock`

Touched files:

- `package.json`
- `src/routes/editor/+page.svelte`
- `src-tauri/Cargo.toml`
- `src-tauri/Cargo.lock`
- `src-tauri/tauri.conf.json`

Verification:

- `pnpm exec tsc --noEmit --pretty false` passed.
- `cargo check` passed.
- `pnpm build` passed.
- `pnpm tauri build` completed successfully.
- Local release artifacts were produced:
  - `src-tauri/target/release/bundle/msi/TEpub-Editor_0.5.2_x64_zh-CN.msi`
  - `src-tauri/target/release/bundle/nsis/TEpub-Editor_0.5.2_x64-setup.exe`

Caveats:

- Build still emits existing Svelte accessibility warnings outside this feature area.
- Tauri still warns that bundle identifier `com.tepubeditor.app` ends with `.app`.

### 2026-05-12 13:21 +08:00

Request: optimize the EPUB cover-search result display after the result cards appeared cramped in the cover column.

Changes:

- Moved cover search results out of the narrow cover column into a full-width result panel below the EPUB metadata form.
- Increased the EPUB creation modal width and cover column width for a more balanced layout.
- Redesigned result cards as a responsive cover grid with larger thumbnails, short titles, source labels, and an `优先` badge.
- Shortened remote-cover application failure messages so errors no longer stretch the right-side cover controls.

Touched files:

- `src/routes/editor/+page.svelte`

Verification:

- `pnpm exec tsc --noEmit --pretty false` passed.
- `cargo check` passed.
- `pnpm build` passed.

Caveats:

- `pnpm build` still emits existing Svelte accessibility warnings outside this feature area.

### 2026-05-12 13:17 +08:00

Request: change EPUB cover search from direct Qidian/Fanqie site search to general image-search style results, only using source domains for auto-priority.

Changes:

- Reworked the backend cover search to query general image results for `book title + author + 小说 封面`.
- Parse image result metadata for original image URL, source page URL, title, and host.
- Removed the old Qidian mobile page result parser from the active search path.
- Kept auto-priority based on trusted image/source domains such as Yuewen, Qidian, Fanqie, and Byteimg.

Touched files:

- `src-tauri/src/lib.rs`

Verification:

- `cargo check` passed.
- `pnpm exec tsc --noEmit --pretty false` passed.
- `pnpm build` passed.

Caveats:

- The current general image-search parser depends on Bing Images result metadata markup.
- `pnpm build` still emits existing Svelte accessibility warnings outside this feature area.

### 2026-05-12 13:14 +08:00

Request: also auto-prioritize cover search results from `https://fanqienovel.com` and `https://p9-novel-sign.byteimg.com`.

Changes:

- Extracted cover-source priority detection into a shared backend helper.
- Added `fanqienovel.com` and `p9-novel-sign.byteimg.com` to the preferred cover source rules.
- Switched remote cover download Referer based on Qidian/Yuewen versus Fanqie/Byteimg image domains.

Touched files:

- `src-tauri/src/lib.rs`

Verification:

- `cargo check` passed.
- `pnpm exec tsc --noEmit --pretty false` passed.

Caveats:

- Fanqie search endpoint still needs a separate reliable integration; this change prioritizes matching Fanqie/Byteimg results when they are present in cover candidates.

### 2026-05-12 13:09 +08:00

Request: add cover search below the EPUB creation cover area, auto-searching by book title and author, auto-applying Qidian/Yuewen covers, while keeping manual result and local image selection.

Changes:

- Added cover search controls under the EPUB creation cover preview.
- Search results are shown as selectable cover cards, with preferred Qidian/Yuewen results marked and auto-applied.
- Kept local image selection as a separate button and as the cover preview click action.
- Added Tauri backend commands for searching `m.qidian.com` cover candidates and downloading a selected remote cover to a local temp file.
- Allowed remote HTTPS/data images in the Tauri image CSP so search result thumbnails can render.
- Added `reqwest` for backend cover search/download.

Touched files:

- `src/routes/editor/+page.svelte`
- `src-tauri/src/lib.rs`
- `src-tauri/Cargo.toml`
- `src-tauri/Cargo.lock`
- `src-tauri/tauri.conf.json`

Verification:

- `pnpm exec tsc --noEmit --pretty false` passed.
- `cargo check` passed.
- `pnpm build` passed.

Caveats:

- Cover search currently targets Qidian mobile search results and may need adjustment if their page structure changes.
- `pnpm build` still emits existing Svelte accessibility warnings outside this feature area.

### 2026-05-12 12:52 +08:00

Request: change the library's default double-click action to edit files.

Changes:

- Updated the default shelf setting so double-clicking a book opens the editor by default for new or unset shelf settings.

Touched files:

- `src/routes/+page.svelte`

Verification:

- `pnpm exec tsc --noEmit --pretty false` passed.

### 2026-05-12 12:51 +08:00

Request: refine default TOC regex levels and title patterns for volumes, extras, final chapters, and metadata sections.

Changes:

- Split the old first metadata regex into two defaults:
  - `内容简介 / 本书相关 / 完本感言` at volume level.
  - `简介 / 序章 / 序言 / 前言 / 楔子 / 后记 / 尾声` at body/chapter level.
- Tightened default volume matching to only `第X卷 标题` or `卷X 标题` with a separator before title text.
- Added `终章 标题` matching to the default chapter regex.
- Kept `番外 ...`, `新增番外 ...`, and `【番外】...` in chapter matching.
- Updated local settings migration so legacy broad metadata, volume, and chapter defaults are rewritten to the new split defaults.
- Updated backend meta detection so `本书相关` is treated as a volume-level meta section.

Touched files:

- `src/routes/editor/+page.svelte`
- `src-tauri/src/lib.rs`

Verification:

- `pnpm exec tsc --noEmit --pretty false` passed.
- `cargo check` passed.
- `pnpm build` passed.

Caveats:

- `pnpm build` still emits existing Svelte accessibility warnings outside this change area.

### 2026-05-12 12:44 +08:00

Request: reduce false positive TOC title detection for prose mentioning volumes, and add extra-title matching for `番外 ...` and `【番外】...`.

Changes:

- Added `番外 ...`, `新增番外 ...`, and `【番外】...` to the default chapter title regex.
- Migrated legacy default chapter regex rules to the new default so existing local settings pick up the extra-title support.
- Added runtime filtering for `第X卷/部` lines immediately followed by prose connector characters such as `的`, `和`, `与`, `想`, `写`, `看`, `说`, and `讲`.
- Kept the backend regex patterns Rust-compatible by avoiding unsupported lookaround syntax.

Touched files:

- `src/routes/editor/+page.svelte`

Verification:

- `pnpm exec tsc --noEmit --pretty false` passed.
- `pnpm build` passed.

Caveats:

- `pnpm build` still emits existing Svelte accessibility warnings outside this change area.

### 2026-05-12 12:36 +08:00

Request: make history version time and size easier to distinguish.

Changes:

- History version rows now use a two-column grid.
- Snapshot time stays in the left column with ellipsis when needed.
- Snapshot size is right-aligned in a fixed-width monospace column.

Touched files:

- `src/routes/editor/+page.svelte`

Verification:

- `pnpm exec tsc --noEmit --pretty false` passed.

### 2026-05-12 12:32 +08:00

Request: simplify word-count check results, move editor settings save/apply to the bottom, make history versions a settings card, and move theme settings to library settings.

Changes:

- Word-count check result chips now show the chapter title and final word count only, without `低于/高于` threshold text.
- TXT editor settings now use `显示 / 目录 / 历史版本` tabs, with history snapshots shown as a dedicated settings page instead of a separate button.
- TXT editor `保存并应用` is now in a fixed bottom footer of the settings modal.
- Removed theme selection from TXT editor settings.
- Added theme selection to the library settings panel and persisted it through `app-settings.uiTheme`.

Touched files:

- `src/routes/+page.svelte`
- `src/routes/editor/+page.svelte`

Verification:

- `pnpm exec tsc --noEmit --pretty false` passed.
- `pnpm build` passed.

Caveats:

- `pnpm build` still emits existing Svelte accessibility warnings outside this change area.

### 2026-05-12 12:28 +08:00

Request: change simplified/traditional proofreading search to find occasional opposite-script characters instead of whole sentences, and prevent unsafe reverse scans in mostly simplified/traditional books.

Changes:

- Simplified/traditional preview now checks the book's dominant script before scanning.
- Mostly simplified books skip `简体转繁体` preview searches, and mostly traditional books skip `繁体转简体` preview searches, avoiding accidental full-book scans.
- Preview results are now per suspicious character/short run, with a small surrounding context instead of the entire line or sentence.
- Applying selected preview results now replaces only the matched character/run, not the full line.
- Clicking a conversion preview result now selects the exact matched position in the editor.

Touched files:

- `src/lib/textProofing.ts`
- `src/routes/editor/+page.svelte`

Verification:

- `pnpm exec tsc --noEmit --pretty false` passed.
- `pnpm build` passed.

Caveats:

- `pnpm build` still emits existing Svelte accessibility warnings outside this change area.

### 2026-05-12 12:25 +08:00

Request: refine library ingest feedback and fix proofreading title-number conversion, built-in replacement safety, and simplified/traditional conversion preview/layout.

Changes:

- Added library toolbar ingest status next to the book count: TXT/non-slow imports show `入库中`, while EPUB imports that keep processing past a short delay show `解密入库中`.
- Added a directory rewrite scope for converting title numbers without reordering chapter/volume sequence.
- Built-in proofreading replacement previews now carry source offsets and apply replacements by exact text ranges instead of coarse line splices, preserving adjacent line breaks more reliably.
- Built-in text-check preview messaging now correctly says matches default to unselected.
- Simplified/traditional preview now filters by direction-specific characters before offering line replacements, so `繁体转简体` no longer proposes broad simplified-to-traditional replacements in mostly simplified books.
- Simplified/traditional preview keeps original indentation in replacements and its action buttons use a compact grid to avoid overlap in the proofreading panel.
- Freed the stale Node dev server that was occupying port `1420`.

Touched files:

- `src/lib/textProofing.ts`
- `src/routes/+page.svelte`
- `src/routes/editor/+page.svelte`

Verification:

- `pnpm exec tsc --noEmit --pretty false` passed.
- `pnpm build` passed.
- Confirmed port `1420` no longer has a listening process after stopping the stale Node server.

Caveats:

- `pnpm build` still emits existing Svelte accessibility warnings outside this change area.

### 2026-05-12 08:36 +08:00

Request: fix proofreading word-count layout overflow, make text-check replacements default unselected, add preview replacement for simplified/traditional conversion, move TXT settings next to search, and refresh library/TXT settings UI.

Changes:

- Fixed the proofreading word-count threshold controls so the right-side input no longer overflows the panel.
- Built-in text check now defaults to no selected matches after preview generation.
- Simplified/traditional conversion now supports finding convertible lines, previewing original/replacement text, click-to-locate, selected replacement, and replace all.
- TXT editor toolbar now places Settings to the right of Search, matching the library toolbar order.
- Library settings panel now uses a cleaner card-grid layout for storage, file associations, and shelf display.
- TXT editor settings panel now uses a wider, cleaner two-column shell with side tabs.
- Long check-result labels now truncate within the proofreading panel instead of spilling horizontally.

Touched files:

- `src/lib/textProofing.ts`
- `src/routes/+page.svelte`
- `src/routes/editor/+page.svelte`

Verification:

- `pnpm exec tsc --noEmit --pretty false` passed.
- `pnpm build` passed.
- Started local Vite dev server at `http://127.0.0.1:1420` and verified toolbar order, proofreading tabs, conversion preview controls, text-check default disabled replacement, and library settings rendering via browser DOM snapshots.

Caveats:

- Browser verification of the TXT editor still logs expected non-Tauri API errors when opened outside the Tauri desktop shell.
- `pnpm build` still emits existing Svelte accessibility warnings outside this feature area.

### 2026-05-12 08:14 +08:00

Request: bump the app to version 0.5.1, build locally, push updates to GitHub, and trigger GitHub Actions.

Changes:

- Updated app version from `0.5.0` to `0.5.1` in:
  - `package.json`
  - `src-tauri/tauri.conf.json`
  - `src-tauri/Cargo.toml`
  - `src-tauri/Cargo.lock`
- Prepared the current proofreading panel improvements for release.

Verification:

- `pnpm exec tsc --noEmit --pretty false` passed.
- `pnpm tauri build` completed successfully.
- Local release artifacts were produced:
  - `src-tauri/target/release/bundle/msi/TEpub-Editor_0.5.1_x64_zh-CN.msi`
  - `src-tauri/target/release/bundle/nsis/TEpub-Editor_0.5.1_x64-setup.exe`

Caveats:

- Build still emits existing Svelte accessibility warnings.
- Tauri still warns that bundle identifier `com.tepubeditor.app` ends with `.app`.

### 2026-05-12 08:09 +08:00

Request: refine the proofreading panel tabs, TOC check button feedback, word count checks, and directory rewrite sequence highlighting.

Changes:

- Reordered proofreading tabs to: title check, directory rewrite, text check, simplified/traditional conversion.
- Made the TOC header Check button open the title-check panel on pointer down so the visual response appears before mouse release.
- Moved visible word-count check controls from settings into the proofreading title-check panel.
- Added separate low/high word-count thresholds, defaulting to 2000 and 6000.
- Word-count checks now report chapters below the low threshold or above the high threshold.
- Directory rewrite defaults now use Chinese numbers for volumes and Arabic numbers for chapters.
- Directory rewrite preview now marks only the actual sequence break point instead of highlighting every following chapter.
- Directory rewrite original-title cells no longer get normal change highlighting; they only turn red for broken sequence rows.

Touched files:

- `src/lib/textProofing.ts`
- `src/routes/editor/+page.svelte`

Verification:

- `pnpm exec tsc --noEmit --pretty false` passed.
- `pnpm build` passed.
- `tauri dev` was already running and `/editor` remained reachable, but generic browser verification still hits expected non-Tauri API errors outside the desktop shell.

Caveats:

- `pnpm build` still emits existing Svelte accessibility warnings outside this feature area.

### 2026-05-12 07:56 +08:00

Request: refine the TXT proofreading directory preview and merge the old top TOC check popup into the proofreading panel.

Changes:

- Added a "Check" tab inside the proofreading panel and routed the TOC header Check button to that tab.
- Moved broken sequence, title content, and word count checks into the proofreading panel.
- Kept the existing check result click behavior so each item still jumps to the matching editor position.
- Made directory preview highlighting column-scoped: the original-title and replacement-title cells highlight independently.
- Kept broken sequence emphasis on the original-title cell only, using red instead of row-wide highlighting.
- Made original-title cells clickable so preview rows can jump to their source position.
- Adjusted volume preview rows so the collapse control and volume title align cleanly.
- Disabled the old standalone check popup path.
- Removed unused row-wide preview highlight styles.

Touched files:

- `src/routes/editor/+page.svelte`

Verification:

- `pnpm exec tsc --noEmit --pretty false` passed.
- `pnpm build` passed.

Caveats:

- `pnpm build` still emits existing Svelte accessibility warnings outside this feature area.

### 2026-05-12 07:45 +08:00

Request: improve proofreading directory rewrite preview with broken sequence highlighting, sticky/collapsible volumes, and separate volume/chapter number styles.

Changes:

- Directory rewrite preview now parses original title numbers.
- Chapter rows whose original number does not match the expected current order are marked as broken sequence.
- Broken sequence chapter rows render in red in the preview.
- Volume rows in the directory preview are sticky while scrolling and remain visible until the next volume reaches the same position.
- Volume rows can be collapsed/expanded, hiding or showing their chapter rows.
- Directory rewrite options now expose separate number styles for volumes and chapters.
- Title rewrite generation now uses the separate volume/chapter number styles.

Touched files:

- `src/lib/textProofing.ts`
- `src/routes/editor/+page.svelte`

Verification:

- `pnpm exec tsc --noEmit --pretty false` passed.
- `pnpm build` passed.
- Offline sample confirmed that `第3章` in the second chapter position is flagged as broken sequence, while volume/chapter number styles can differ.

Caveats:

- Build still emits existing Svelte accessibility warnings outside this feature area.
- `.claude/` and `AGENTS.md` remain untracked and untouched.

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
