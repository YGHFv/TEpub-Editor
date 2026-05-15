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

### 2026-05-15 19:49 +08:00

Request: fix Android launcher icon cropping, make the mobile make-EPUB TOC preview fold volumes reliably, bump the app to v0.5.6, and push a release update that also builds an Android APK in GitHub Actions.

Changes:

- Regenerated Android launcher icons with safe padding from the desktop icon so adaptive icon masks no longer crop the artwork oddly.
- Changed the mobile make-EPUB TOC preview folding model to bind chapters directly to their volume row, matching the working reorder-preview folding behavior.
- Bumped app metadata to `0.5.6` in npm, Cargo, Tauri config, and lockfile.
- Added a GitHub Actions Android release job that builds an arm64 APK, signs it, uploads it as an artifact, and attaches it to tagged draft releases.

Verification:

- `pnpm build` passed with existing warnings.
- `cargo check` passed.
- `pnpm tauri android build --target aarch64 --apk true --aab false` produced a release APK.
- Signed `TEpub-Editor-android-arm64-release-signed.apk`; `apksigner verify` passed with v2/v3 signatures.
- Installed the signed APK on the connected Android device and launched `com.tepubeditor.app/.MainActivity`.

### 2026-05-15 19:35 +08:00

Request: change the Android app icon to match the desktop app icon and rebuild the Android package.

Changes:

- Synced Android launcher icon resources from `src-tauri/icons/android` into the generated Android project resource folders.
- Confirmed the generated Android `mipmap-xxxhdpi/ic_launcher.png` now matches the source Android icon hash.

Verification:

- `pnpm tauri android build --target aarch64 --apk true --aab false` produced a release APK.
- Signed the release APK with the local Android debug keystore for install testing; `apksigner verify` passed with v2/v3 signatures.
- Installed `TEpub-Editor-android-arm64-release-signed.apk` on the connected Android device and launched `com.tepubeditor.app/.MainActivity`.

### 2026-05-15 19:27 +08:00

Request: fix remaining Android make EPUB TOC folding and WebP preview failures, then package a normal installable APK.

Changes:

- Simplified mobile make EPUB TOC classification so non-meta level-1 entries are treated as volume nodes, allowing chapter rows below them to fold reliably.
- Kept intro/preface/postface entries protected as meta rows so they do not become fold parents or reorder targets.
- Fixed EPUB editor extraction file classification to recognize image extensions case-insensitively, including WebP/GIF/SVG/BMP.
- Added frontend image-extension fallback so image files classified as `other` by unusual EPUB structures still open through the binary preview path instead of UTF-8 text reading.

Verification:

- `pnpm build` passed.
- `cargo check` passed.
- `pnpm tauri android build --target aarch64 --apk true --aab false` produced a release APK.
- Signed the release APK with the local Android debug keystore for install testing; `apksigner verify` passed with v2/v3 signatures.
- Installed `TEpub-Editor-android-arm64-release-signed.apk` on the connected Android device and launched `com.tepubeditor.app/.MainActivity`.

### 2026-05-15 19:12 +08:00

Request: refine Android make EPUB and EPUB image preview issues: make TOC volume folding work, prevent intros from being reordered, add 1234/一二三四 numbering choices, add cover and UUID inputs, and fix WebP preview.

Changes:

- Added mobile make EPUB cover selection and preview next to title/author, passing the cached cover path into EPUB export.
- Added a UUID field below the make EPUB metadata area; it auto-generates by default and becomes editable when focused.
- Added volume/chapter numbering style choices for TOC reorder; defaults are Chinese volume numbers and Arabic chapter numbers.
- Tightened mobile TOC item classification on both frontend and backend so intros/prefaces/postfaces are meta entries even when matched by level-3 rules, keeping them visible but excluded from reorder.
- Strengthened volume detection to require a real volume title shape, which keeps meta rows from becoming collapsible parents.
- Made EPUB editor image preview detect MIME from image bytes, including RIFF/WEBP, and fixed typed-array byte-offset handling.

Verification:

- `pnpm build` passed.
- `cargo check` passed.
- `pnpm tauri android build --debug --target aarch64 --apk true --aab false` passed.
- Reinstalled the debug APK on the connected Android device and launched `com.tepubeditor.app/.MainActivity`.

### 2026-05-15 19:01 +08:00

Request: keep refining Android mobile EPUB editor and make-EPUB flows: make the edit EPUB secondary page match the other secondary pages, fix volume folding in make EPUB, and bring mobile TOC reorder preview closer to the desktop behavior with ranges, regex, and per-volume numbering control.

Changes:

- Unified the mobile edit EPUB empty state with the other secondary pages by using the same white panel pattern and removing its extra header divider.
- Fixed mobile make EPUB TOC hierarchy so meta entries such as intros/prefaces remain visible but no longer become volume parents; real volumes now own chapter folding.
- Added mobile TOC reorder controls for scope selection, custom regex selection, and optional per-volume chapter numbering.
- Updated reorder preview to keep non-reordered entries visible, highlight broken sequence chapters, and fold chapter rows under each volume.
- Checked Android build size: the debug APK is large because the debug native library is about 193MB and APK signing/alignment adds similar overhead; the existing release output is about 14.2MB APK / 7.2MB AAB.

Verification:

- `pnpm build` passed.
- `cargo check` passed.
- `pnpm tauri android build --debug --target aarch64 --apk true --aab false` passed.
- Reinstalled the debug APK on the connected Android device and launched `com.tepubeditor.app/.MainActivity`.

### 2026-05-15 18:46 +08:00

Request: fix remaining Android mobile page polish issues: secondary-page back arrows not aligned with titles, edit EPUB header being the only white title bar, WebP preview failure, confusing separated preview/edit tabs, dark code preview background, EPUB folder group rows still too tall, and expand/collapse alignment.

Changes:

- Unified mobile secondary-page topbars to a fixed grid with a 34px centered back button and transparent background.
- Removed the EPUB edit page's white header bar so it matches the other mobile secondary pages.
- Reduced EPUB edit folder group row height and tightened the add/expand button columns.
- Replaced the split preview/edit code-file flow with a single editable CodeMirror-based `EpubCodeEditor` surface, giving syntax highlighting while editing.
- Switched the mobile code editor surface back to a light background.
- Made image binary handling more tolerant of different Tauri bridge return shapes before creating Blob URLs, which should help WebP preview on Android.

Verification:

- `pnpm build` passed.
- `cargo check` passed.
- `pnpm tauri android build --debug --target aarch64 --apk true --aab false` passed.
- Reinstalled the debug APK on the connected Android device and launched `com.tepubeditor.app/.MainActivity`.

### 2026-05-15 18:35 +08:00

Request: refine Android mobile TOC checking/reordering and EPUB editing UX: remove word-count checks from mobile, make TOC preview/check panels collapsible, show reorder preview with desktop-like highlighting, highlight broken sequence items in TOC preview, fix EPUB edit folder action alignment, support image preview, and add syntax-colored preview for HTML/CSS/XML-like files.

Changes:

- Removed mobile-only word-count issue checks from the make EPUB page while keeping sequence and empty-title checks.
- Added a collapsible TOC preview header matching the TOC check panel behavior.
- Added a desktop-like two-column reorder preview (`原标题` / `修改后`) and changed the reorder action to apply the previewed replacements.
- Added shared broken-sequence highlighting in both TOC preview rows and reorder preview rows.
- Reworked mobile EPUB edit group headers so folder title, add button, and expand/collapse button occupy stable aligned columns.
- Wired the group add button to add a selected file into the corresponding EPUB internal folder and refresh the file list.
- Added image file preview using existing EPUB binary read commands.
- Added syntax-colored preview for HTML/CSS/XML/other editable text files, with an edit tab for direct content changes.

Verification:

- `pnpm build` passed.
- `cargo check` passed.
- `pnpm tauri android build --debug --target aarch64 --apk true --aab false` passed.
- Reinstalled the debug APK on the connected Android device and launched `com.tepubeditor.app/.MainActivity`.

Notes:

- The code coloring is a lightweight mobile preview highlighter, not the full desktop CodeMirror editor surface.

### 2026-05-15 18:21 +08:00

Request: reduce oversized Android secondary-page typography; make the EPUB edit file list denser and closer to the provided Yuewei reference; allow tapping editable EPUB files; fix group expand/collapse alignment; investigate the 1.22GB Android package; add TOC checking and reorder tools for mobile EPUB creation.

Changes:

- Tightened mobile secondary-page spacing, header sizes, button heights, and form/list typography across make/decrypt/edit/metadata pages.
- Updated the mobile EPUB edit file list to use smaller grouped rows, fixed action/expand columns, and compact file icons/subtitles.
- Added tap-to-edit for editable EPUB internal files (`html`, `css`, `xml`, `other`) using the existing EPUB read/save backend commands and a bottom mobile editor panel.
- Added mobile make-page TOC checking for sequence jumps, empty titles, and abnormal chapter word counts.
- Added one-tap TOC reorder for TXT sources, preserving title bodies while rewriting headings into ordered `第N卷` / `第N章` labels.
- Cleaned stale Android generated `libepub_editor_lib.so` artifacts from the local generated Android project.
- Removed local generated debug JNI keep-symbol packaging configuration and rebuilt a single-ABI arm64 debug APK for phone testing.

Verification:

- `pnpm build` passed.
- `cargo check` passed.
- `pnpm tauri android build --debug` passed.
- `pnpm tauri android build --debug --target aarch64 --apk true --aab false` passed.
- Installed the resulting debug APK to the connected Android device and launched `com.tepubeditor.app/.MainActivity`.

Notes:

- The original universal debug APK was oversized because it bundled four unstripped Rust debug libraries plus stale old-name JNI libraries. After cleanup, universal debug dropped to about 777MB; the single-ABI arm64 debug APK is about 200MB. The release APK remains about 14MB.
- `src-tauri/gen/android` is generated/ignored, so local generated-project cleanup may need repeating if the Android project is regenerated.

### 2026-05-15 15:08 +08:00

Request: optimize Android page logic so the home page only keeps four feature entries; use separate pages for each workflow; add TXT TOC preview/regex adjustment for EPUB creation; make EPUB edit page resemble the provided mobile file-list design, with text file subtitles showing corresponding titles.

Changes:

- Replaced `/mobile` with a four-entry home page only.
- Added separate mobile workflow pages:
  - `/mobile/make`
  - `/mobile/decrypt`
  - `/mobile/edit`
  - `/mobile/metadata`
- Added shared mobile file/export helpers in `src/lib/mobileFlow.ts`.
- Added TXT/MD/HTML import for the make page, plus TOC preview, editable regex rules, foldable TOC rows, title/author fields, EPUB generation, and export.
- Aligned the make page default TOC regex with the TXT editor defaults, including the stricter volume rule for `第X卷 标题` / `卷X 标题` style headings.
- Added `mobile_scan_chapters` and updated `mobile_make_epub` so mobile EPUB generation uses the currently edited regex rules.
- Added HTML/XHTML heading extraction during `extract_epub`, so mobile EPUB edit rows can show the chapter title under `chapter0`, `chapter1`, etc. instead of generic HTML text.
- Added a mobile EPUB edit file-list UI modeled after the provided screenshots, with grouped/collapsible folders and type-specific row subtitles.

Touched files:

- `src/routes/mobile/+page.svelte`
- `src/routes/mobile/make/+page.svelte`
- `src/routes/mobile/decrypt/+page.svelte`
- `src/routes/mobile/edit/+page.svelte`
- `src/routes/mobile/metadata/+page.svelte`
- `src/lib/mobileFlow.ts`
- `src-tauri/src/lib.rs`
- `PROJECT_LOG.md`

Verification:

- `pnpm build` passed.
- `cargo check` passed.
- `pnpm tauri android build --debug` passed.
- Reinstalled the debug APK on the connected Android device and launched `com.tepubeditor.app/.MainActivity`.

Caveats:

- The EPUB edit mobile page currently focuses on structure browsing and export; actual chapter content editing remains a later step.
- Build still emits existing Svelte accessibility warnings and the existing large chunk warning from unrelated desktop pages.

### 2026-05-15 14:43 +08:00

Request: Android export currently lands under `Android/data`, startup flashes the desktop library UI, and the remaining mobile function blocks need backend wiring.

Changes:

- Added an early Android-only redirect in `src/app.html` so the Android WebView goes to `/mobile` before the desktop library route renders.
- Removed the previous post-mount Android redirect from `src/routes/+layout.svelte`, keeping desktop theme initialization intact.
- Added `mobile_make_epub` to generate an EPUB from imported TXT/MD/HTML text using the existing chapter scanner and EPUB exporter.
- Extended the mobile UI so 制作 EPUB, 解密 EPUB, and 编辑 EPUB each have a backend-backed export action instead of being status-only placeholders.
- Enhanced mobile export to try public `/storage/emulated/0/Download/TEpub-Editor` and `/sdcard/Download/TEpub-Editor` before falling back to app data.
- Added a system share/download handoff after export, so Android can save/share the generated EPUB from a visible system flow even when direct public-folder writes are denied.
- Added `fs:allow-appdata-read-recursive` so the mobile UI can read exported app-data EPUBs for system share/download.

Touched files:

- `src/app.html`
- `src/routes/+layout.svelte`
- `src/routes/mobile/+page.svelte`
- `src-tauri/src/lib.rs`
- `src-tauri/capabilities/default.json`
- `PROJECT_LOG.md`

Verification:

- `pnpm build` passed.
- `cargo check` passed.
- `pnpm tauri android build --debug` passed.
- Reinstalled the debug APK on the connected Android device and launched `com.tepubeditor.app/.MainActivity`.

Caveats:

- Android scoped storage can still deny direct public Downloads writes on some devices. The new share/download handoff is the practical visible export path until a native SAF folder picker/save-as bridge is added.
- Build still emits existing Svelte accessibility warnings and the existing large chunk warning from unrelated desktop pages.

### 2026-05-15 14:22 +08:00

Request: metadata editing now works, but modified EPUBs cannot be exported; add a place to store exported files and continue adding functionality.

Changes:

- Added `mobile_export_epub` Tauri command to export the current mobile cached EPUB.
- Export first tries the public download directory under `Downloads/TEpub-Editor`, using a sanitized and de-duplicated EPUB filename.
- If Android denies public Downloads writes, export falls back to the app data `mobile-exports` directory and returns a message explaining the fallback.
- Added a mobile metadata "导出 EPUB" button. If metadata is dirty, it saves metadata first, then exports the updated EPUB.
- The mobile UI now displays the export path in the status panel.

Touched files:

- `src-tauri/src/lib.rs`
- `src/routes/mobile/+page.svelte`
- `PROJECT_LOG.md`

Verification:

- `pnpm build` passed.
- `cargo check` passed with no warnings.
- `pnpm tauri android build --debug` passed.
- Reinstalled the debug APK on the connected Android device and launched `com.tepubeditor.app/.MainActivity`.

Caveats:

- This is a pragmatic first export path. A true Android SAF folder picker/save-as flow is still needed to let users select arbitrary folders reliably across file providers.
- Build still emits existing Svelte accessibility warnings and the existing large chunk warning from unrelated desktop pages.

### 2026-05-15 14:08 +08:00

Request: Android metadata file selection fails with `fs.write_file not allowed`.

Changes:

- Added `fs:allow-appdata-write-recursive` to the default Tauri capability so the mobile route can write selected EPUB files into `$APPDATA/mobile-imports`.

Touched files:

- `src-tauri/capabilities/default.json`
- `PROJECT_LOG.md`

Verification:

- `pnpm build` passed.
- `cargo check` passed.
- `pnpm tauri android build --debug` passed.
- Reinstalled the debug APK on the connected Android device and launched `com.tepubeditor.app/.MainActivity`.

Caveats:

- The mobile metadata flow still edits the cached EPUB copy until the next export/save-as step is implemented.
- Build still emits existing Svelte accessibility warnings and the existing large chunk warning from unrelated desktop pages.

### 2026-05-15 14:02 +08:00

Request: Android metadata file selection fails with `TypeError: Must be a file URL`.

Changes:

- Replaced the mobile route's Tauri dialog selection path with a hidden native browser `<input type="file">`, avoiding Android `content://` URLs entirely.
- Cached the selected browser `File` into the app data `mobile-imports` directory through `File.arrayBuffer()` + `@tauri-apps/plugin-fs.writeFile`.
- Kept a timeout around file import so failed providers recover the UI instead of leaving the button busy.
- Metadata save now clearly reports that the edited EPUB is saved to the app cache copy until a later export/overwrite flow is added.

Touched files:

- `src/routes/mobile/+page.svelte`
- `PROJECT_LOG.md`

Verification:

- `pnpm build` passed.
- `cargo check` passed.
- `pnpm tauri android build --debug` passed.
- Reinstalled the debug APK on the connected Android device and launched `com.tepubeditor.app/.MainActivity`.

Caveats:

- The mobile metadata flow currently edits the cached EPUB copy. A dedicated export/save-as action is needed next so users can write the edited file back to a chosen location.
- Build still emits existing Svelte accessibility warnings and the existing large chunk warning from unrelated desktop pages.

### 2026-05-15 13:55 +08:00

Request: Android mobile file selection gets stuck on "处理中" after choosing a file.

Changes:

- Changed mobile `content://` handling to copy the selected file directly into the app data `mobile-imports` directory via `@tauri-apps/plugin-fs.copyFile`.
- Removed the hot path that converted the whole EPUB to `Array.from(bytes)` and sent it through a Tauri invoke JSON payload, which could stall the WebView bridge on larger books.
- Added a 45-second timeout around Android file import so the UI recovers with a clear error instead of staying busy forever.
- Metadata sync-back now uses `copyFile` from the cached EPUB to the original `content://` URI when Android grants write access.

Touched files:

- `src/routes/mobile/+page.svelte`
- `PROJECT_LOG.md`

Verification:

- `pnpm build` passed.
- `cargo check` passed.
- `pnpm tauri android build --debug` passed.
- Reinstalled the debug APK on the connected Android device and launched `com.tepubeditor.app/.MainActivity`.

Caveats:

- Some third-party Android file providers may not support direct copy from their `content://` URI; the code falls back to read/write and times out with guidance if the provider stalls.
- Build still emits existing Svelte accessibility warnings and the existing large chunk warning from unrelated desktop pages.

### 2026-05-15 13:49 +08:00

Request: fix Android mobile file selection reporting "EPUB 文件不存在".

Changes:

- Added `mobile_cache_input_file` Tauri command to save Android-selected file bytes into the app data `mobile-imports` directory as a real filesystem path.
- Updated the mobile route to detect Android `content://` selection URIs, read them through `@tauri-apps/plugin-fs`, and pass the cached path to existing EPUB backend commands.
- Metadata save now attempts to sync the modified cached EPUB back to the original Android `content://` URI, and reports when Android only permits saving the app cache copy.

Touched files:

- `src-tauri/src/lib.rs`
- `src/routes/mobile/+page.svelte`
- `PROJECT_LOG.md`

Verification:

- `pnpm build` passed.
- `cargo check` passed.
- `pnpm tauri android build --debug` passed.
- Reinstalled the debug APK on the connected Android device and launched `com.tepubeditor.app/.MainActivity`.

Caveats:

- Large EPUBs selected from Android providers are copied into app cache before processing; later mobile flows should add a visible export/save-copy action for cached results.
- Build still emits existing Svelte accessibility warnings and the existing large chunk warning from unrelated desktop pages.

### 2026-05-15 13:39 +08:00

Request: start adding functional Android features after the mobile workbench shell and Android test.

Changes:

- Added mobile-only EPUB metadata DTO and Tauri commands for reading and writing metadata directly by selected EPUB path.
- Reused the existing OPF parser, OPF metadata writer, and EPUB rewrite pipeline instead of duplicating EPUB zip handling.
- Wired the Android `/mobile` metadata module to read title, author, subtitle, publisher, maker, series, tags, UUID, and description.
- Added a mobile metadata edit form with dirty-state save behavior and tag splitting.

Touched files:

- `src-tauri/src/lib.rs`
- `src/routes/mobile/+page.svelte`
- `PROJECT_LOG.md`

Verification:

- `pnpm build` passed.
- `cargo check` passed.
- `pnpm tauri android build --debug` passed and produced:
  - `src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk`
  - `src-tauri/gen/android/app/build/outputs/bundle/universalDebug/app-universal-debug.aab`

Caveats:

- The metadata save currently writes back to the selected EPUB in place; cover editing and "save as copy" are still future mobile workflow steps.
- Build still emits existing Svelte accessibility warnings and the existing large chunk warning from unrelated desktop pages.

### 2026-05-15 13:29 +08:00

Request: run an Android test for the new mobile UI.

Changes:

- Temporarily configured the local shell with the installed Android SDK/NDK paths.
- Fixed the local generated Android package namespace mismatch from `com.epubeditor.app` to `com.tepubeditor.app` so it matches `tauri.conf.json`.
- Removed stale generated Kotlin under the old package path in the ignored `src-tauri/gen/android` tree.
- Built and installed the Android debug APK on a connected device.

Verification:

- `pnpm tauri android build --debug` passed.
- Built artifacts:
  - `src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk`
  - `src-tauri/gen/android/app/build/outputs/bundle/universalDebug/app-universal-debug.aab`
- `adb install -r .../app-universal-debug.apk` succeeded.
- `adb shell am start -n com.tepubeditor.app/.MainActivity` succeeded.
- Device foreground check showed `com.tepubeditor.app/.MainActivity` focused.
- Screenshot confirmed the Android mobile workbench UI is visible.

Caveats:

- `src-tauri/gen/` is gitignored, so the namespace cleanup is local generated-project maintenance. If Android is regenerated, run `tauri android init` from the current `com.tepubeditor.app` identifier or keep the generated namespace aligned.
- Build still emits existing Svelte accessibility/CSS warnings, chunk-size warning, and Android/JDK deprecation warnings.

### 2026-05-15 13:13 +08:00

Request: start implementing a separate Android UI for EPUB creation/decryption/editing/metadata workflows without affecting existing desktop pages.

Changes:

- Added a new `/mobile` Svelte route as the Android EPUB workbench entry.
- Built a mobile-first home UI with separate functional modules for making EPUBs, decrypting EPUBs, editing EPUBs, and editing EPUB metadata.
- Added Android-only root redirection in the shared layout so Android WebView opens `/mobile`, while desktop platforms keep the existing library home page.
- Added first-step file picking affordances per mobile module.
- Wired the Android decrypt module to `prepare_epub_for_open` and the edit module to `extract_epub` for first-step EPUB processing.

Touched files:

- `src/routes/+layout.svelte`
- `src/routes/mobile/+page.svelte`
- `PROJECT_LOG.md`

Verification:

- `pnpm build` passed.

Caveats:

- This is still an early mobile UI shell. The next step should add a dedicated mobile metadata read/save command and then a mobile-friendly EPUB creation wizard.
- Build still emits the existing chunk-size warning.

### 2026-05-13 15:50 +08:00

Request: bump TEpub-Editor to 0.5.5, build locally, then push updates to GitHub to trigger Actions.

Changes:

- Updated app/package versions from 0.5.4 to 0.5.5.
- Included the latest reader/theme/proofreading/library fixes already present in the working tree for the 0.5.5 release.
- Ran a local Tauri release package build.

Touched files:

- `package.json`
- `src-tauri/Cargo.toml`
- `src-tauri/Cargo.lock`
- `src-tauri/tauri.conf.json`
- `PROJECT_LOG.md`

Verification:

- `pnpm tauri build` passed.
- Built local artifacts:
  - `src-tauri/target/release/bundle/msi/TEpub-Editor_0.5.5_x64_zh-CN.msi`
  - `src-tauri/target/release/bundle/nsis/TEpub-Editor_0.5.5_x64-setup.exe`

Caveats:

- Build still emits existing Svelte accessibility/CSS warnings and the existing chunk-size warning.

### 2026-05-13 15:43 +08:00

Request: fix occasional blank pages in EPUB reader pagination, reduce page-turn stutter, and add darker/heavier body text controls.

Changes:

- Neutralized EPUB-internal `page-break-before/after` hints inside chapter content during paginated reading, while keeping reader-managed chapter section boundaries.
- Reduced background hydrate batch size and paused background hydration briefly after user page turns so lazy rendering is less likely to compete with flipping.
- Added body text tone and weight settings to the reader typography panel, with darker/thicker defaults for body readability.
- Applied body text color/weight through CSS variables so these controls do not trigger a full pagination recompute.

Touched files:

- `src/routes/reader/+page.svelte`

Verification:

- `pnpm build` passed.

Caveats:

- Build still emits the existing chunk-size warning.

### 2026-05-13 15:26 +08:00

Request: keep `山河稷` chapter header artwork from occupying a whole first page, and let transparent artwork areas follow the reader theme background without changing other colors.

Changes:

- Added a narrow reader CSS override for EPUB `.header_image` blocks: neutralize forced page breaks and cap header artwork height to leave room for chapter title/body text.
- Made `.epub-body` and chapter-header decoration containers transparent only at the `background-color` layer, preserving text colors, image colors, and background images.
- Kept the earlier no-op header normalization stance, avoiding generic DOM rewriting or forced full-bleed image scaling.

Touched files:

- `src/routes/reader/+page.svelte`

Verification:

- `pnpm build` passed.

Caveats:

- Build still emits the existing chunk-size warning.

### 2026-05-13 15:52 +08:00

Request: `山河稷` is still broken in both scroll and paginated reading modes after header-image normalization.

Changes:

- Disabled the reader's opening-header detection from adding internal header classes.
- Disabled header-image normalization so EPUB chapter artwork/title/logo blocks render in their original EPUB document flow.
- Removed the CSS rules that resized or constrained `.rd-opening-header-*` elements.
- Kept a small cleanup pass that removes stale header-only/spacer artifacts from previously normalized DOM.

Touched files:

- `src/routes/reader/+page.svelte`

Verification:

- `pnpm build` passed.

Caveats:

- This intentionally favors EPUB-native rendering over forced full-bleed headers; true paginated full-bleed needs a later page-container rewrite rather than CSS-column overflow tricks.
- Build still emits the existing chunk-size warning.

### 2026-05-13 15:45 +08:00

Request: fix direct page turns where the next page shows the previous chapter header image/title over the text.

Changes:

- Stopped allowing opening header images to horizontally overflow CSS columns in paginated mode.
- Kept edge bleed only in scroll mode, where there are no adjacent columns for the image to paint into.
- Changed the fixed-header image CSS to fill its normalized wrapper instead of independently expanding beyond the page column.

Touched files:

- `src/routes/reader/+page.svelte`

Verification:

- `pnpm build` passed.

Caveats:

- Paginated mode now prioritizes no overlap/no right overflow over full horizontal bleed. A full rewrite should implement clipping per page before re-enabling true edge-to-edge banners in paginated mode.
- Build still emits the existing chunk-size warning.

### 2026-05-13 15:36 +08:00

Request: fix remaining EPUB reader rendering issues where a banner still did not touch the edge, and `山河稷_deobf.epub` showed chapter art/text overlap and horizontal overflow on later pages.

Changes:

- Inspected `山河稷_deobf.epub`; chapters use `.wrap.article > .header_image > img.width100`, and `.header_image` has `page-break-before: always`.
- Relaxed opening-banner detection so wide chapter art is normalized even when EPUB image dimensions are smaller than the previous threshold.
- Overrode EPUB header-image forced page breaks in the reader, preventing `.header_image { page-break-before: always; }` from fighting CSS-column pagination.
- Stopped copying EPUB body backgrounds into the reader's fullscreen `.rd-chapter-bg` layer and stopped clearing `.epub-body` backgrounds, so decorative art stays in the document flow like the EPUB editor preview.
- Preserved inline body background styles instead of stripping them during reader serialization.

Touched files:

- `src/routes/reader/+page.svelte`

Verification:

- `pnpm build` passed.

Caveats:

- Build still emits the existing chunk-size warning.

### 2026-05-13 15:16 +08:00

Request: stop patching header images piecemeal and make the reader follow the editor preview's normal-flow rendering before pagination.

Changes:

- Removed the reader's absolute-position plus spacer treatment for opening header images.
- Changed wide opening headers to render in normal document flow with edge-to-edge sizing and a capped height, so text can continue below the image on the same page.
- Detected pure header-image spine sections and allowed the following section to continue without a forced page break, matching editor-preview behavior more closely when EPUBs split banners and text into adjacent XHTML files.

Touched files:

- `src/routes/reader/+page.svelte`

Verification:

- `pnpm build` passed.

Caveats:

- This is a targeted first step toward a fuller reader pagination rewrite; the current reader still uses CSS columns for page slicing.
- Build still emits the existing chunk-size warning.

### 2026-05-13 14:58 +08:00

Request: fix the reader regression where non-header opening images were enlarged, while real header banners still left a page with only the image.

Changes:

- Narrowed opening-header treatment to wide banner-like images only, based on rendered/natural aspect and size.
- Removed the old CSS-only first-image enlargement path so portrait avatars and ordinary opening illustrations keep their EPUB-defined size.
- Added cleanup for previously normalized non-banner images by removing injected spacers and inline full-bleed styles when the image fails the wide-header check.

Touched files:

- `src/routes/reader/+page.svelte`

Verification:

- `pnpm build` passed.

Caveats:

- Build still emits the existing chunk-size warning.

### 2026-05-13 14:44 +08:00

Request: make the EPUB reader use the EPUB editor preview's working header-image behavior instead of continuing CSS-only guesses.

Changes:

- Ported the editor preview's core header-image strategy into the reader: opening header wrappers are absolutely positioned to the reader frame and followed by a spacer that preserves document flow height.
- Recomputed the spacer from the rendered image size and rebound image load events so late-loaded EPUB images can correct pagination after loading.
- Kept the reader's pagination/toolbar/progress system intact instead of embedding the editor preview iframe wholesale.

Touched files:

- `src/routes/reader/+page.svelte`

Verification:

- `pnpm build` passed.

Caveats:

- Build still emits the existing chunk-size warning.

### 2026-05-13 14:27 +08:00

Request: fix EPUB reader opening header images that still did not touch the left/top edges, including nested header image wrappers.

Changes:

- Added DOM-time detection for each chapter's opening image and marked it with internal reader classes.
- Marked nested opening image wrappers so EPUB structures such as `wrap > header_image > img` and `div > img` receive header-image layout treatment.
- Applied edge-to-edge header image styling to the marked image instead of only direct first-child images.

Touched files:

- `src/routes/reader/+page.svelte`

Verification:

- `pnpm build` passed.

Caveats:

- Build still emits existing Svelte accessibility warnings outside this change area.

### 2026-05-13 14:20 +08:00

Request: continue fixing EPUB reader header-image bleed after screenshots still showed a left-edge remnant on later pages.

Changes:

- Increased the single-page reader's internal CSS column gap to provide a wider off-page buffer for edge-to-edge header images.
- Stabilized first-image wrapper sizing so header wrappers keep the page content width while the image itself can extend to the reader frame edge.

Touched files:

- `src/routes/reader/+page.svelte`

Verification:

- `pnpm build` passed.

Caveats:

- Build still emits existing Svelte accessibility warnings outside this change area.

### 2026-05-13 14:14 +08:00

Request: fix EPUB reader rendering where chapter header images do not touch the page edge and overflow into later pages.

Changes:

- Separated the visible spread gap from the CSS column gap in the EPUB reader layout.
- Added a single-page column buffer so full-width header images can extend into the page margin without bleeding into adjacent pages.
- Cleared left/right spacing on first-image wrappers so chapter header images can align to the reader frame edge more reliably.

Touched files:

- `src/routes/reader/+page.svelte`

Verification:

- `pnpm build` passed.

Caveats:

- Build still emits existing Svelte accessibility warnings outside this change area.

### 2026-05-13 13:58 +08:00

Request: fix multiple dark-theme display problems across the library, TXT editor, and EPUB editor.

Changes:

- Added global dark-theme compatibility overrides for older component-local light surfaces.
- Normalized dark backgrounds, borders, text colors, inputs, selects, textareas, buttons, tabs, sidebars, settings panels, proofreading panels, EPUB file tree, preview pane, and find/replace controls.
- Preserved accent, active, primary, and danger states under the dark theme.

Touched files:

- `src/app.css`

Verification:

- `pnpm build` passed.

Caveats:

- Build still emits existing Svelte accessibility warnings outside this change area.

### 2026-05-13 13:42 +08:00

Request: fix incorrect matches in the TXT proofreading pinyin check.

Changes:

- Tightened pinyin detection so it validates actual pinyin syllables instead of generic Latin/digit fragments.
- Rejected formula/code-like fragments containing symbols such as `=`, `_`, `*`, `☉`, and `≈`.
- Updated pinyin preview and replacement paths to use the stricter cleanup function, so ordinary chapter titles, Chinese text containing numbers, and formula lines are no longer listed as pinyin matches.

Touched files:

- `src/lib/textProofing.ts`

Verification:

- `pnpm build` passed.

Caveats:

- Build still emits existing Svelte accessibility warnings outside this change area.

### 2026-05-12 22:54 +08:00

Request: bump TEpub-Editor to 0.5.4, build locally, then push updates to GitHub and trigger Actions.

Changes:

- Bumped app version from 0.5.3 to 0.5.4.
- Included the pending cover-search, cover-layout, TXT/EPUB close behavior, and context-menu fixes in the release candidate.

Touched files:

- `package.json`
- `src-tauri/tauri.conf.json`
- `src-tauri/Cargo.toml`
- `src-tauri/Cargo.lock`
- `PROJECT_LOG.md`

Verification:

- `pnpm build` passed.
- `cargo check` finished successfully.
- `pnpm tauri build` completed and produced MSI/NSIS release bundles for 0.5.4.
- GitHub push is pending in this workflow.

### 2026-05-12 22:50 +08:00

Request: prevent the TXT-only title context menu from appearing in the EPUB editor text area, and reduce cases where closing TXT/EPUB editors closes the whole app.

Changes:

- Added an `enableTitleActions` switch to the shared context menu component.
- Enabled title actions only on the TXT editor page.
- Left the EPUB editor CodeMirror area with normal edit actions only.
- Added an EPUB editor close helper that returns to the library when the current window is the main window, and only destroys child editor windows.
- Used that helper for EPUB close-confirm save/discard flows and for clean main-window close requests.

Touched files:

- `src/lib/ContextMenu.svelte`
- `src/routes/editor/+page.svelte`
- `src/routes/epub-editor/+page.svelte`
- `PROJECT_LOG.md`

Verification:

- `pnpm build` passed.
- `cargo check` finished successfully.

### 2026-05-12 22:38 +08:00

Request: restore the TXT editor right-click menu after the input-field context-menu guard made it stop appearing.

Changes:

- Fixed the context-menu input guard so CodeMirror's contenteditable editor surface is still treated as the editor.
- Plain input, textarea, and non-editor contenteditable fields still keep their native right-click behavior.

Touched files:

- `src/lib/ContextMenu.svelte`
- `PROJECT_LOG.md`

Verification:

- `pnpm build` passed.

### 2026-05-12 22:36 +08:00

Request: fix the first row of cover search results still not showing fully, and fix dev-mode TXT editor close actions not returning to the library or exiting the app correctly.

Changes:

- Increased the cover search results panel height so a full first row can fit after switching thumbnails to `3:4`.
- Narrowed cover-result cards to keep one row shorter and more stable inside the modal.
- Removed long image alt text from result thumbnails so broken remote images do not render huge text inside the image frame.
- Changed TXT editor close handling so the configured `exit` action wins even when the editor was opened from the library.
- Changed return-to-library for library-opened TXT editor windows to use `destroy()` instead of `close()` to bypass recursive close interception in dev mode.

Touched files:

- `src/routes/editor/+page.svelte`
- `PROJECT_LOG.md`

Verification:

- `cargo check` finished successfully.
- `pnpm build` passed.

### 2026-05-12 19:42 +08:00

Request: fix EPUB cover preview still hiding part of the cover, make cover search results closer to expected Bing image results, treat `icode.qq.com` as a preferred source, and use the normal web-novel cover ratio.

Changes:

- Changed the EPUB creation cover preview to a `3:4` web-novel cover ratio.
- Changed cover-search result thumbnails to the same `3:4` ratio and kept `object-fit: contain`.
- Moved the hover hint into a small pill so it no longer spans and blocks the whole bottom edge of the cover.
- Switched cover search to `cn.bing.com` and made the first query use the book title directly.
- Added merged fallback queries for title + author and title + novel cover while keeping title matches weighted highest.
- Added `icode.qq.com` to preferred cover sources and boosted portrait-like cover dimensions.
- Updated the cover-results helper text to describe ranking instead of implying an automatic preferred-cover apply.

Touched files:

- `src/routes/editor/+page.svelte`
- `src-tauri/src/lib.rs`
- `PROJECT_LOG.md`

Verification:

- `cargo check` finished successfully.
- `pnpm build` passed.

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
