# Web Deployment Track

TEpub-Editor is currently a Tauri desktop application with a SvelteKit static frontend. The web deployment track keeps the desktop and mobile builds intact by introducing a platform adapter boundary instead of replacing Tauri calls directly inside pages.

## Build Modes

- Desktop/Tauri frontend: `pnpm build`
- Web frontend: `pnpm build:web`
- Web dev server: `pnpm dev:web`
- Web preview: `pnpm preview:web`

`web` mode changes the Vite dev server to port `5173` and defines `__TEPUB_TARGET__` as `web`. The default mode remains `tauri` for existing desktop release flows.

## Platform Boundary

New code should import platform capabilities from:

```ts
import { platform } from "$lib/platform";
```

The adapter currently exposes:

- `platform.invoke(command, args)` for backend commands
- dialog helpers: `openDialog`, `saveDialog`, `message`, `ask`
- filesystem/window helpers where available
- `platform.kind`, `platform.isTauri`, `platform.isWeb`

Desktop mode loads Tauri APIs lazily from `$lib/platform/tauri.ts`. Web mode routes backend commands to HTTP:

```text
POST /api/commands/:command
```

The request body is the command args as JSON. The response body should be JSON.

## Migration Rule

Do not add new direct imports from `@tauri-apps/*` in Svelte pages. New or touched features should use `$lib/platform` first. Existing pages can be migrated gradually, one workflow at a time, so desktop behavior stays stable.

## Web Limitations

Browser builds cannot directly use local paths, native save/open dialogs, file associations, system tray, or Explorer reveal. Web workflows must use browser upload/download and server-side temporary files.

The recommended backend migration is to expose the existing Rust command behavior as HTTP endpoints, then route web `platform.invoke` calls to those endpoints.
