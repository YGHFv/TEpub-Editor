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

## 桌面端文件访问范围（Tauri scope）

桌面端前端的文件访问范围已从早期的全盘 `**` 收敛，`src-tauri/capabilities/default.json` 与 `tauri.conf.json` 现在只静态放行 `$APPDATA`：

- **plugin-fs 静态 scope**：仅 `$APPDATA`（递归读写）。前端直接的 plugin-fs 调用只有三处——`platform.readFile/writeFile`、mobile 导入缓存写入、批处理日志导出，目标都在 `$APPDATA` 内或来自对话框返回值。
- **对话框选中路径自动授权**：`plugin-dialog` 的 `open`/`save` 在用户确认后会调用 `allow_file`/`allow_directory` 把该路径加入运行时 scope。因此「另存为任意位置」照常可用，且授权精确到用户刚选的那个文件。
- **运行时补授权（两处）**：`extract_epub` 把 EPUB 解压目录授权给 asset 协议——书库根可被用户指向任意目录、便携版指向 exe 同级 `data/`、失败时还会回退系统临时目录，静态 scope 无法覆盖；`mobile_export_epub` 把导出到公共下载目录（Android `/storage/emulated/0/Download` 等）的文件授权给 fs scope，供前端读回交给系统分享面板。
- **assetProtocol.scope**：静态 `$APPDATA/**` 加上述运行时授权。reader 通过 `asset://` 直接 fetch 解压后的章节/CSS/字体/图片（比 IPC 快一个数量级），这条链路依赖运行时授权。
- **其余文件操作走后端 invoke**：83 个 Tauri 命令中绝大多数文件读写在 Rust 侧完成，不受插件 scope 限制——这也是收敛能做得这么彻底的前提。

新增前端直接 fs 调用前，请先确认目标路径落在 `$APPDATA` 内或来自对话框返回值；否则应改为后端命令或在命令内运行时授权，而不是放宽静态 scope。

## 安全注意事项

自托管公网部署前，请了解当前服务端（`server/web-server.mjs`）的安全边界：

- **代理端点需登录**：`/api/commands/*`（AI 校对、AI 生图、远程图片下载）要求携带有效会话 token，且所有服务器外发请求都会做内网/环回地址过滤，防止被当作开放代理或 SSRF 跳板。
- **凭据明文落盘**：会话 token 和用户设置（可能包含 AI API Key）以明文存储在 `/data/users.json`。请确保数据目录权限受控，不要把它放在可被静态服务读到的位置；磁盘加密或卷加密可以进一步降低风险。
- **会话管理**：登录时每个账号仅保留最近 5 个会话 token；登出会撤销当前 token。没有过期时间机制，长期不用的 token 需要靠重新登录轮换挤出。
- **频率限制**：登录/注册按来源 IP 限制为 5 分钟 10 次。若部署在反向代理之后，`req.socket.remoteAddress` 是代理地址，限速会退化为全局共享——需要限速生效时应在代理层（Nginx/Caddy）配置。
- **传输加密**：服务本身只提供 HTTP。公网部署必须在前面加 TLS 终结（反向代理或负载均衡器），否则 token 和密码明文过网。
- **请求体上限**：认证/设置接口 1 MB，命令接口 128 MB（图片处理需要较大请求体）。如有更严格需求可在反向代理层再收紧。

小规模自用部署（内网、朋友间共享）以上默认值即可；面向公众的部署建议额外加反向代理层的 WAF/限速，并定期备份 `/data`。
