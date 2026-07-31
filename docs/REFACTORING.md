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

1. ✅ 查找替换核心 → `src/lib/findReplace.ts`（31 个测试）。已切换：Web 版 toolbox/epub-editor、search-replace 窗口（extended 转义下沉、顺带修复 wholeWord 连字符正则非法的潜在 bug）、桌面 epub-editor（findNextMatchWrapped 保留历史语义；getFilesInScope 因桌面 open/selected/html 范围语义与共享版实质不同，有意保留待统一）。editor 走后端 Rust 正则（advanced_search），语义不同，按设计不切换。
2. ✅ reader 的 container.xml/OPF/nav/NCX 手写解析（约 180 行）已删除，改用 `webEpub.ts` 导出的 `parseContainerRootfilePath` / `parseOpfPackage` / `parseNavDocument` / `parseNcxDocument`；`loadWebEpub` 同步改为复用同一实现。
3. ✅ localStorage key 已集中到 `src/lib/storageKeys.ts`（21 个 key/前缀，动态 key 配生成函数），16 个使用文件全部改为常量导入；共享 key（`app-settings`、`epub-find/replace-history`、`tepub-epub-style-library-v1`、`tepub-editor-launch-files`、`tepub-editor-batch-task:` 等）在模块内注明共享方。webAccount.ts 私有账号 key 原地保留（storageKeys.ts 登记备查）；app.html 内联脚本无法 import，保留 `tepub-web-view-mode` 硬编码副本并两侧注释互指。
4. ✅ 字体逻辑 → `src/lib/fontTools.ts`（17 个导出、26 个测试，epub-editor 净 -125 行）；DOM 遍历与 FontFace 注册等状态耦合部分按计划留在页面。

### 第 3 期：桌面页面接入平台抽象 ✅ 已完成（2026-07-28）

四个桌面主页面的 `@tauri-apps/api` 直接调用已全部替换为 `$lib/platform`（迁移规则见 `docs/web-deploy.md`），静态导入清零，桌面行为不变，web 端各能力均有论证过的降级：

- ✅ reader：8 处替换；PlatformAdapter 新增 `listen`、`getFileSrcConverter`（异步取同步转换器，适配 CSS url() 同步替换）。
- ✅ library：60 处替换；新增 `onCurrentWindowFileDrop`、`setCurrentWindowTitle`。
- ✅ editor：62 处替换；新增 `emit`、`setCurrentWindowPosition/Size`、`onCurrentWindowMoved`、窗口句柄 `listen`（窗口作用域，可收 emitTo 定向事件，与全局 listen 不可互换）。
- ✅ epub-editor：59 处替换；新增 `confirm`（OK/Cancel 双钮，与 ask/message 语义区分）。

### 第 4 期：拆分巨型组件 ✅ 路线图刀次已完成（2026-07-28）

全部新组件为 Svelte 5 runes 写法（$props/$bindable/$derived/$effect + 回调 props），业务 IO 一律留页面：

- `epub-editor`（6764 → 4974 行，累计净 -1790）：FileTreePanel(254)、TabsBar(218)、FindReplacePanel(579，顺带切换共享 findReplace)、PreviewPane(278)、FontPreviewPanel(236)，另有 fileIcons.ts 纯函数模块。
- `editor`（9032 → 7426 行，累计净 -1606）：ChapterSidebar(417)、EditorToolbar(216)、设置面板 8 个 tab 全部抽出——SettingsDisplayTab(70)、SettingsFontsTab(233)、SettingsTocTab(282)、SettingsApiTab(385)、SettingsAiTab(172)、SettingsProofLogsTab(217)、SettingsStylesTab(649)、SettingsHistoryTab(119)。批处理队列经核实该页不存在（原审查记录有误）。
- 落地经验（拆组件必读）：
  1. legacy `set = set` / `arr = [...arr]` 自赋值穿 runes prop 会被引用相等短路——Set 用 `new Set()`、数组项字段原地改用 `map + 新对象`；
  2. keyed each 下仅浅拷贝数组不够，item 引用不变则 `{#if item.x}` 不重算；
  3. 页面裸元素选择器（button、::-webkit-scrollbar、共享 .placeholder）经 Svelte hash 作用域后不穿组件边界，需组件内复制并注释；
  4. 全局 app.css 的样式不受组件边界影响，无需迁移。
- 可选后续（不在路线图承诺内）：editor 校对面板（~540 行，与检查/AI 状态纠缠最深）；ChapterSidebar 加 scrollToChapter 实例方法收编页面的 getElementById 滚动定位。

### 第 5 期：错误处理与通知统一 ✅ 已完成（2026-07-28）

`src/lib/toast.ts` + `Toast.svelte` 已接入 epub-editor（替换全部 14 处 `alert()`）、library（9 处用户可感知失败接 `toastError`）和 editor（2 处）。其余静默 catch 经逐一审查确认为合理静默（已有原生对话框/页面内状态栏反馈、后台 best-effort、缓存回退），保持原样。Tauri 原生 confirm 对话框保留（阻塞式确认是合理场景）。

## 不做 / 需要决策

- **LICENSE**：README 声明保留所有权利，但公开推 Docker 镜像并教用户自部署，默认授权矛盾。需要作者决定：补一个写明允许范围的自定义 LICENSE，或换 MIT/Apache-2.0。
- **Tauri `fs:scope "**"` 收敛** ✅ 已完成（2026-07-31）：`fs:scope` 的 `**` 条目已删除，`assetProtocol.scope` 收为 `$APPDATA/**`。前端直接 fs 调用只剩三处且都在 `$APPDATA` 或对话框返回路径内；对话框选中路径由 plugin-dialog 自动授权；`extract_epub`（解压目录 → asset 协议）与 `mobile_export_epub`（公共下载目录导出文件 → fs scope）在运行时按实际路径补授权。细节见 `docs/web-deploy.md` 的「桌面端文件访问范围」章节。
- **全量 cargo fmt / prettier --write**：工具链已配好（`.prettierrc`、`eslint.config.js`、`rustfmt.toml`），全量格式化会产生巨大 diff，应在某个功能间歇期单独提交，不与功能改动混合。
