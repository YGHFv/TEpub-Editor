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

Some older Svelte pages contain historically mojibake text. Avoid whole-file PowerShell rewrites on those files; use targeted patches or UTF-8-preserving editor operations so string literals are not damaged during migration.

## Web Limitations

Browser builds cannot directly use local paths, native save/open dialogs, file associations, system tray, or Explorer reveal. Web workflows must use browser upload/download and server-side temporary files.

The recommended backend migration is to expose the existing Rust command behavior as HTTP endpoints, then route web `platform.invoke` calls to those endpoints.

## 安全注意事项

自托管公网部署前，请了解当前服务端（`server/web-server.mjs`）的安全边界：

- **代理端点需登录**：`/api/commands/*`（AI 校对、AI 生图、远程图片下载）要求携带有效会话 token，且所有服务器外发请求都会做内网/环回地址过滤，防止被当作开放代理或 SSRF 跳板。
- **凭据明文落盘**：会话 token 和用户设置（可能包含 AI API Key）以明文存储在 `/data/users.json`。请确保数据目录权限受控，不要把它放在可被静态服务读到的位置；磁盘加密或卷加密可以进一步降低风险。
- **会话管理**：登录时每个账号仅保留最近 5 个会话 token；登出会撤销当前 token。没有过期时间机制，长期不用的 token 需要靠重新登录轮换挤出。
- **频率限制**：登录/注册按来源 IP 限制为 5 分钟 10 次。若部署在反向代理之后，`req.socket.remoteAddress` 是代理地址，限速会退化为全局共享——需要限速生效时应在代理层（Nginx/Caddy）配置。
- **传输加密**：服务本身只提供 HTTP。公网部署必须在前面加 TLS 终结（反向代理或负载均衡器），否则 token 和密码明文过网。
- **请求体上限**：认证/设置接口 1 MB，命令接口 128 MB（图片处理需要较大请求体）。如有更严格需求可在反向代理层再收紧。

小规模自用部署（内网、朋友间共享）以上默认值即可；面向公众的部署建议额外加反向代理层的 WAF/限速，并定期备份 `/data`。
