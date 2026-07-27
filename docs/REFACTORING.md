# 重构路线图

本文档记录代码审查（2026-07）识别出的大型技术债，以及分期消化的建议顺序。每期都应该独立可合并、可回滚，避免长期分支。

## 现状概览

| 问题 | 规模 | 影响 |
|------|------|------|
| 巨型组件 | `editor/+page.svelte` ~9000 行、`epub-editor/+page.svelte` ~6300 行、`mobile/make` ~3900 行 | 任何修改都要在数千行里定位，状态互相纠缠 |
| 平行实现分叉 | 查找替换有 3-4 套实现；桌面/Web 的 epub-editor 有约 20 个同名函数各自演化；reader 手写约 400 行 OPF 解析而 `webEpub.ts` 已有一套 | 修 bug 要修多处，行为不一致 |
| `lib.rs` 单文件 | ~13500 行、83 个 `#[tauri::command]` | Rust 侧同样难以定位与复用 |
| Svelte 4/5 混用 | 全部路由页面是 legacy 写法，仅 `SettingsShell.svelte` 用 runes | 新维护者困惑，两套心智模型 |
| 桌面页面绕过平台抽象 | editor / epub-editor / library / reader 直接 import `@tauri-apps/api` | Web 版无法复用桌面页面，被迫平行实现 |

## 分期计划

### 第 1 期：Rust 模块拆分 ✅ 已完成（2026-07-28）

`lib.rs` 按功能域拆成模块，**只移动代码不改逻辑**。实际落地结构（lib.rs 从 13501 行减到 203 行）：

```
src-tauri/src/
  lib.rs        # 203 行：mod 声明、tray/menu、run()、generate_handler 注册
  toolbox.rs    # 4814 行：toolbox_* 命令、远程图片、AI 生图、批处理（含 8 个测试）
  epub.rs       # 3315 行：EPUB 读写、OPF 元数据、scan_chapters（含 5 个测试）
  library.rs    # 1739 行：书库 CRUD、封面、历史、md5、LibraryConfig
  fs_utils.rs   #  939 行：EpubCache、zip 条目清洗、路径帮助
  font.rs       #  729 行：字体库命令、sfnt/cmap、fonttools 子集化
  mobile.rs     #  557 行：mobile_* 命令
  templates.rs  #  517 行：模板仓库 + 样式模板
  app.rs        #  378 行：系统/文件/启动参数命令
  ai.rs         #  285 行：AI 校对与日志
  search.rs     #  139 行：advanced_search / advanced_replace / search_in_files
```

验收结果：`cargo build` 0 警告、`cargo test` 13/13、83 个命令名与拆分前逐一 diff 一致。

### 第 2 期：前端共享逻辑下沉到 `src/lib` ✅ 已完成（2026-07-28，findReplace 消费者切换遗留见第 1 条）

从巨型组件里**先抽纯逻辑，不动 UI**，每块配套 vitest：

1. ✅ 查找替换核心 → `src/lib/findReplace.ts`（21 个测试）。已切换 Web 版 `toolbox/epub-editor`（净 -39 行）。**剩余**：桌面 epub-editor、editor（后端 Rust 正则，语义不同需谨慎）、search-replace 窗口（有独占的 extended 转义模式）逐个切换；四套实现的行为差异清单见 findReplace.ts 注释与 2026-07 审查记录。
2. ✅ reader 的 container.xml/OPF/nav/NCX 手写解析（约 180 行）已删除，改用 `webEpub.ts` 导出的 `parseContainerRootfilePath` / `parseOpfPackage` / `parseNavDocument` / `parseNcxDocument`；`loadWebEpub` 同步改为复用同一实现。
3. ✅ localStorage key 已集中到 `src/lib/storageKeys.ts`（21 个 key/前缀，动态 key 配生成函数），16 个使用文件全部改为常量导入；共享 key（`app-settings`、`epub-find/replace-history`、`tepub-epub-style-library-v1`、`tepub-editor-launch-files`、`tepub-editor-batch-task:` 等）在模块内注明共享方。webAccount.ts 私有账号 key 原地保留（storageKeys.ts 登记备查）；app.html 内联脚本无法 import，保留 `tepub-web-view-mode` 硬编码副本并两侧注释互指。
4. ✅ 字体逻辑 → `src/lib/fontTools.ts`（17 个导出、26 个测试，epub-editor 净 -125 行）；DOM 遍历与 FontFace 注册等状态耦合部分按计划留在页面。

### 第 3 期：桌面页面接入平台抽象

把 editor / epub-editor / library / reader 中的 `@tauri-apps/api` 直接调用替换为 `$lib/platform`（迁移规则见 `docs/web-deploy.md`）。一次一个工作流（如"打开文件"、"保存"），桌面行为保持不变。完成后桌面与 Web 可逐步共享页面组件。

### 第 4 期：拆分巨型组件

前两期完成后组件已明显变薄，再按 UI 区域拆：

- `epub-editor`：文件树、标签栏、预览面板、查找替换面板、字体预览各自成组件。
- `editor`：工具栏、设置面板、批处理队列。

拆出的新组件**直接用 Svelte 5 runes 写**；存量 legacy 代码不主动改写，随拆分自然消化。这就是 Svelte 5 迁移策略：新代码 runes、旧代码不动、不做一次性大迁移。

### 第 5 期：错误处理与通知统一 ✅ 已完成（2026-07-28）

`src/lib/toast.ts` + `Toast.svelte` 已接入 epub-editor（替换全部 14 处 `alert()`）、library（9 处用户可感知失败接 `toastError`）和 editor（2 处）。其余静默 catch 经逐一审查确认为合理静默（已有原生对话框/页面内状态栏反馈、后台 best-effort、缓存回退），保持原样。Tauri 原生 confirm 对话框保留（阻塞式确认是合理场景）。

## 不做 / 需要决策

- **LICENSE**：README 声明保留所有权利，但公开推 Docker 镜像并教用户自部署，默认授权矛盾。需要作者决定：补一个写明允许范围的自定义 LICENSE，或换 MIT/Apache-2.0。
- **Tauri `fs:scope "**"` 收敛**：需要先梳理哪些前端直接 fs 调用是必要的（多数走后端 invoke），收敛范围要配合第 3 期做，单独收会破坏现有功能。
- **全量 cargo fmt / prettier --write**：工具链已配好（`.prettierrc`、`eslint.config.js`、`rustfmt.toml`），全量格式化会产生巨大 diff，应在某个功能间歇期单独提交，不与功能改动混合。
