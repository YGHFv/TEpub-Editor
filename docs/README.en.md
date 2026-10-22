# TEpub Editor

[简体中文](../README.md) | [繁體中文](README.zh-TW.md) | **English**

![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux%20%7C%20Android-blue)
![Version](https://img.shields.io/badge/Version-1.4.0-orange)
![Stack](https://img.shields.io/badge/Stack-Tauri%202%20%2B%20Svelte%205%20%2B%20Rust-00a7d0)
![Languages](https://img.shields.io/badge/Languages-Simplified%20Chinese%20%7C%20Traditional%20Chinese%20%7C%20English-green)

TEpub Editor is a workbench built around TXT and EPUB. It combines library management, long-form text editing, EPUB creation, in-EPUB editing, reading, and a collection of EPUB processing tools in one application, while offering both a desktop client and a browser-based Web version. The interface supports Simplified Chinese, Traditional Chinese, and English.

## Features

The application uses the Toolbox as its entry point and includes the following tools:

**Common entries**

- **Library**: Manage imported books, covers, and metadata
- **TXT to EPUB**: Generate an EPUB from TXT and enter the contents preview and creation workflow
- **EPUB Style Library**: Preview header-image and title styles
- **TXT Editor**: Import, proofread, find, and replace text
- **EPUB Editor**: Edit files inside an EPUB
- **EPUB Reader**: Read EPUB books

The desktop version uses a single-instance, multi-window model. Launching the app again brings the existing window to the front. Opening a TXT or EPUB by double-clicking or through the context menu hands the file to the existing process, and each file can open in its own editor or reader window without closing or replacing other books.

**EPUB processing** (creates new files; supports individual files or batches)

- **Font encryption / font decryption**
- **File encryption / file decryption**
- **EPUB restructuring**: Clean up paths and references
- **Image conversion**: Convert WebP images inside an EPUB
- **Image processing**: Create full-screen covers and Yuewei banners
- **EPUB diagnostics**: Check the OPF, manifest, and internal references
- **EPUB to TXT and EPUB 2.0 / 3.0 conversion**
- **Whole-book Simplified/Traditional conversion, ad removal, pinyin annotation, and note/footnote conversion**
- **Image compression, invisible image watermarks, and bidirectional PNG/JPEG/WebP conversion**
- **EPUB merging, chapter-based splitting, and font subsetting**
- **Send to Kindle**: Open Amazon's official Send to Kindle page

The conversion and organization tools share a browser engine, so the desktop and Web versions perform the same processing. See [`epub-toolkit-feature-audit.md`](epub-toolkit-feature-audit.md) for the complete comparison.

### Editing and proofreading

- A text editor based on CodeMirror 6
- Find, replace, and regular-expression search
- Simplified/Traditional Chinese conversion based on opencc-js
- Configurable contents-detection regular expressions (volume and chapter levels)
- Optional AI proofreading and image generation: configure a text model or image model in Settings (Base URL / API Key / model name)

### Reader

- Paged and scrolling reading modes
- Adjustable font size, line spacing, paragraph spacing, and page margins
- Theme, background, and font settings
- Contents navigation, bookmarks, and progress saving

## Multilingual Interface

- Supports Simplified Chinese, Traditional Chinese, and English, with Simplified Chinese as the source language
- Follows the device or browser language on first launch; the language can also be changed manually in Toolbox or mobile settings
- Persists the language preference and keeps it synchronized across desktop windows, the Web version, and mobile
- Converts the Traditional Chinese interface automatically with OpenCC, covering major pages, tools, settings, and runtime messages

## Technology Stack

- **Frontend**: Svelte 5 + SvelteKit + Vite
- **Desktop shell**: Tauri 2
- **Backend**: Rust
- **Editor**: CodeMirror 6
- **Chinese conversion**: opencc-js
- **UI internationalization**: Svelte 5 runtime translation + OpenCC

## Download and Installation

Download builds from [Releases](https://github.com/YGHFv/TEpub-Editor/releases):

- **Windows**: `.msi` / `.exe`
- **macOS**: `.dmg`
- **Linux**: `.deb` / `.AppImage`
- **Android**: `.apk`

## Web Version

In addition to the desktop client, the project can be built as a browser application. The Web version consists of a static frontend and an optional lightweight Node service. The service provides account registration, sign-in, account-level settings storage, and an AI request proxy; account data is written to `/data`. For security reasons, server-proxied features such as AI proofreading, AI image generation, and remote image downloads require a signed-in account.

The Web Toolbox provides all 14 entries: Library, TXT to EPUB, EPUB Style Library, TXT Editor, EPUB Editor / Reader, font encryption / decryption, file encryption / decryption, EPUB restructuring, image conversion, image processing, and EPUB diagnostics. The Web library uses IndexedDB to store books and version history and can request persistent browser storage. TXT and EPUB books can be opened from the library for reading or editing and saved back.

> File associations, overwriting arbitrary local paths, native multi-window support, and system file-manager actions depend on desktop operating-system capabilities. In the browser, the app uses native Web workflows such as file selection, downloads, and page navigation and never requests unsafe local path permissions.

### Online Demo

Online demos:

- Cloudflare Pages: <https://tepub.ygvlive.com/>
- GitHub Pages: <https://yghfv.github.io/TEpub-Editor/>

> The demo sites are static-only deployments without the Node service, so account registration / sign-in, settings synchronization, and other features that depend on `/api` are unavailable. Use Docker or the Node setup below for the complete feature set.

### Run with Docker

The repository includes `docker-compose.yml`. Run the following from the repository root:

```bash
docker compose up -d
```

Then visit `http://<server-ip>:5233`. Accounts and synchronized settings are stored in the `tepub-web-data` volume and survive container recreation. Book files remain in the visiting device's browser IndexedDB and are not uploaded to the server.

To change the port:

```bash
TEPUB_WEB_PORT=8090 docker compose up -d
```

### Without Docker

Node.js is required:

```bash
pnpm install
pnpm build:web    # Build the static frontend
pnpm serve:web    # Start the built-in Node service at http://127.0.0.1:5233
```

## Local Development

Requirements: Node.js 18+, stable Rust, and pnpm.

```bash
pnpm install
pnpm tauri dev      # Desktop development
pnpm tauri build    # Build desktop installers
pnpm dev:web        # Web development server
```

## License

This project does not declare an open-source license. The source code is published for reference only. All rights reserved.
