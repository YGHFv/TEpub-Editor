<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { base } from "$app/paths";
  import { getWebClientViewOverride, setWebClientViewOverride } from "$lib/clientProfile";
  import { platform, type PlatformWindowHandle } from "$lib/platform";
  import CustomSelect from "$lib/CustomSelect.svelte";
  import SettingsShell from "$lib/SettingsShell.svelte";
  import { isSharedWebTool, isWebRouteToolId, webToolRoute } from "$lib/webToolRoutes";
  import {
    applyTheme,
    DEFAULT_TOC_REGEX_RULES,
    loadAppSettings,
    newAiProvider,
    providerToProofingConfig,
    saveAppSettings,
    type AiProviderConfig,
    type GlobalAppSettings,
    type TocRegexRule,
  } from "$lib/appSettings";
  import {
    getWebAccountSession,
    loginWebAccount,
    logoutWebAccount,
    registerWebAccount,
    syncRemoteSettings,
    usesWebScopedSettings,
    type WebAccountSession,
  } from "$lib/webAccount";

  type ToolId =
    | "library"
    | "txt-edit"
    | "txt-epub"
    | "epub-style-library"
    | "epub-edit"
    | "epub-read"
    | "font-encrypt"
    | "font-decrypt"
    | "file-encrypt"
    | "file-decrypt"
    | "epub-reformat"
    | "image-convert"
    | "image-tools"
    | "epub-diagnose"
    | "epub-to-txt"
    | "epub-version"
    | "epub-chinese"
    | "epub-ad-clean"
    | "epub-phonetic"
    | "epub-footnote"
    | "image-compress"
    | "image-watermark"
    | "epub-merge"
    | "epub-split"
    | "font-subset"
    | "send-to-kindle";

  type Tool = {
    id: ToolId;
    icon: string;
    title: string;
    detail: string;
  };

  type ToolGroupFilter = "all" | "edit" | "convert" | "security" | "optimize";

  type ToolGroup = {
    id: Exclude<ToolGroupFilter, "all">;
    title: string;
    tools: Tool[];
  };

  type ToolboxResult = {
    sourcePath: string;
    outputPath: string;
    changed: boolean;
    action: string;
    message: string;
  };

  type ToolboxDiagnosticIssue = {
    level: string;
    kind: string;
    path?: string | null;
    message: string;
  };

  type ToolboxDiagnosticResult = {
    sourcePath: string;
    opfPath?: string | null;
    totalEntries: number;
    manifestItems: number;
    errorCount: number;
    warningCount: number;
    issues: ToolboxDiagnosticIssue[];
  };

  type LaunchInfo = {
    filePath?: string | null;
    filePaths?: string[];
    action?: string | null;
  };

  type LegacyLibraryData = {
    config?: Record<string, any>;
  };

  const LAUNCH_SESSION_KEY = "tepub-editor-launch-files";
  const BATCH_TASK_PREFIX = "tepub-editor-batch-task:";
  const TOOLBOX_WINDOW_WIDTH = 1200;
  const TOOLBOX_WINDOW_HEIGHT = 740;
  const THEME_OPTIONS = [
    { value: "modern", label: "现代" },
    { value: "classic", label: "经典" },
    { value: "dark", label: "深色" },
  ];
  const REGEX_LEVEL_OPTIONS = [
    { value: "1", label: "卷" },
    { value: "3", label: "章" },
  ];

  const tools: Tool[] = [
    {
      id: "library",
      icon: "LIB",
      title: "书库",
      detail: "管理图书与元数据",
    },
    {
      id: "txt-epub",
      icon: "TXT",
      title: "TXT 转 EPUB",
      detail: "将 TXT 制作为标准 EPUB",
    },
    {
      id: "epub-style-library",
      icon: "CSS",
      title: "EPUB 样式预览",
      detail: "预览头图与标题样式",
    },
    {
      id: "txt-edit",
      icon: "EDIT",
      title: "TXT 编辑器",
      detail: "导入、校对、查找替换",
    },
    {
      id: "epub-edit",
      icon: "EPUB",
      title: "EPUB 编辑器",
      detail: "编辑结构、资源与元数据",
    },
    {
      id: "epub-read",
      icon: "READ",
      title: "EPUB 阅读器",
      detail: "打开 EPUB 进行阅读预览",
    },
    {
      id: "font-encrypt",
      icon: "FONT+",
      title: "EPUB 字体加密",
      detail: "加密 EPUB 内嵌字体",
    },
    {
      id: "font-decrypt",
      icon: "FONT-",
      title: "EPUB 字体解密",
      detail: "使用对应 TXT 还原字体",
    },
    {
      id: "file-encrypt",
      icon: "LOCK",
      title: "EPUB 文件加密",
      detail: "加密 EPUB 正文文件",
    },
    {
      id: "file-decrypt",
      icon: "OPEN",
      title: "EPUB 文件解密",
      detail: "还原 EPUB 正文文件",
    },
    {
      id: "epub-reformat",
      icon: "REFIT",
      title: "EPUB 结构重构",
      detail: "整理目录与引用",
    },
    {
      id: "image-convert",
      icon: "IMG",
      title: "图片格式转换",
      detail: "WebP 与 PNG/JPEG 互转",
    },
    {
      id: "image-tools",
      icon: "COVER",
      title: "封面与横幅",
      detail: "制作全屏封面与阅微横幅",
    },
    {
      id: "epub-diagnose",
      icon: "CHK",
      title: "EPUB 结构诊断",
      detail: "检查 OPF、manifest 和内部引用",
    },
    { id: "epub-to-txt", icon: "TXT", title: "EPUB 转 TXT", detail: "按阅读顺序提取正文" },
    { id: "epub-version", icon: "2/3", title: "EPUB 版本互转", detail: "EPUB 2.0 与 3.0 互转" },
    { id: "epub-chinese", icon: "繁简", title: "EPUB 简繁互转", detail: "转换正文、目录与元数据" },
    { id: "epub-ad-clean", icon: "CLEAN", title: "EPUB 广告清理", detail: "按规则清理广告段落" },
    { id: "epub-phonetic", icon: "PIN", title: "EPUB 拼音标注", detail: "生成标准 ruby 注音" },
    { id: "epub-footnote", icon: "NOTE", title: "EPUB 批注与脚注", detail: "批注与 EPUB3 脚注互转" },
    { id: "image-compress", icon: "ZIP", title: "EPUB 图片压缩", detail: "压缩 EPUB 内嵌图片" },
    { id: "image-watermark", icon: "WM", title: "EPUB 图片水印", detail: "写入或读取隐形文本水印" },
    { id: "epub-merge", icon: "MERGE", title: "EPUB 合并", detail: "保留资源与阅读顺序" },
    { id: "epub-split", icon: "SPLIT", title: "EPUB 拆分", detail: "按章节数拆分为多本 EPUB" },
    { id: "font-subset", icon: "FONT S", title: "EPUB 字体精简", detail: "移除正文未使用字形" },
    { id: "send-to-kindle", icon: "KINDLE", title: "发送到 Kindle", detail: "打开 Amazon 官方传书页面" },
  ];

  const toolById = new Map(tools.map((tool) => [tool.id, tool]));
  const selectTools = (...ids: ToolId[]) => ids.map((id) => toolById.get(id)).filter((tool): tool is Tool => Boolean(tool));
  const temporarilyHiddenToolIds = new Set<ToolId>(["send-to-kindle"]);

  const toolGroups: ToolGroup[] = [
    {
      id: "edit",
      title: "编辑阅读",
      tools: selectTools("library", "txt-edit", "epub-edit", "epub-read", "epub-style-library", "image-tools", "send-to-kindle"),
    },
    {
      id: "convert",
      title: "格式转换",
      tools: selectTools("txt-epub", "epub-to-txt", "epub-version", "epub-chinese", "image-convert", "epub-footnote"),
    },
    {
      id: "security",
      title: "加密解密",
      tools: selectTools("font-encrypt", "font-decrypt", "file-encrypt", "file-decrypt"),
    },
    {
      id: "optimize",
      title: "整理优化",
      tools: selectTools("epub-diagnose", "epub-reformat", "epub-ad-clean", "epub-phonetic", "image-compress", "image-watermark", "font-subset", "epub-merge", "epub-split"),
    },
  ];

  let activeToolGroup: ToolGroupFilter = "all";
  let toolSearch = "";
  $: normalizedToolSearch = toolSearch.trim().toLocaleLowerCase("zh-CN");
  $: visibleToolGroups = toolGroups
    .map((group) => ({
      ...group,
      tools: group.tools.filter((tool) => (
        !temporarilyHiddenToolIds.has(tool.id)
        && (!platform.isWeb || tool.id !== "library")
        && (!normalizedToolSearch || `${tool.title} ${tool.detail}`.toLocaleLowerCase("zh-CN").includes(normalizedToolSearch))
      )),
    }))
    .filter((group) => activeToolGroup === "all" || group.id === activeToolGroup)
    .filter((group) => group.tools.length > 0);

  let busyTool: ToolId | "" = "";
  let statusText = "";
  let showSettings = false;
  let webDesktopViewEnabled = false;
  let toolboxSettingsActiveTab: "account" | "general" | "assoc" | "regex" | "api" = "general";
  $: toolboxSettingsTabs = [
    ...(usesWebScopedSettings() ? [{ id: "account", label: "账号" }] : []),
    { id: "general", label: "通用" },
    ...(usesWebScopedSettings() ? [] : [{ id: "assoc", label: "文件关联" }]),
    { id: "regex", label: "目录正则" },
    { id: "api", label: "API 配置" },
  ];
  let appSettings: GlobalAppSettings = loadAppSettings();
  let accountSession: WebAccountSession | null = getWebAccountSession();
  let accountMode: "login" | "register" = "login";
  let accountUsername = "";
  let accountPassword = "";
  let accountMessage = "";
  let apiEditorOpen = false;
  let apiEditorMode: "new" | "edit" = "new";
  let apiEditorId = "";
  let apiDraft: AiProviderConfig = newAiProvider({ kind: "text" });
  let apiSettingsMessage = "";

  function appPath(path: string) {
    if (!path || path === "#" || /^https?:\/\//i.test(path)) return path;
    return `${base}${path.startsWith("/") ? path : `/${path}`}`;
  }

  function windowLabel(prefix: string) {
    return `${prefix}-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;
  }

  async function createToolWindow(label: string, options: Record<string, any> & { url: string }) {
    const { url, ...windowOptions } = options;
    return platform.createWebviewWindow(label, url, windowOptions);
  }

  function collectLaunchPaths(launchInfo: LaunchInfo | null | undefined) {
    const candidates = [...(launchInfo?.filePaths ?? []), launchInfo?.filePath ?? ""];
    const seen = new Set<string>();
    return candidates
      .filter((path): path is string => typeof path === "string" && path.trim().length > 0)
      .filter((path) => {
        const key = path.toLowerCase();
        if (seen.has(key)) return false;
        seen.add(key);
        return true;
      });
  }

  function launchSessionKey(paths: string[], action: string) {
    return JSON.stringify({ action, paths });
  }

  function supportedLaunchPath(path: string) {
    const ext = path.split(".").pop()?.toLowerCase();
    return ext === "epub" || ext === "txt";
  }

  async function openLaunchPath(filePath: string, action: string) {
    const ext = filePath.split(".").pop()?.toLowerCase();
    const encoded = encodeURIComponent(filePath);

    if (ext === "txt") {
      await createToolWindow(windowLabel("editor"), {
        url: appPath(`/editor?file=${encoded}&fromLibrary=1`),
        title: "TEpub-Editor-TXT",
        width: TOOLBOX_WINDOW_WIDTH,
        height: TOOLBOX_WINDOW_HEIGHT,
        minWidth: TOOLBOX_WINDOW_WIDTH,
        minHeight: TOOLBOX_WINDOW_HEIGHT,
        dragDropEnabled: true,
        center: true,
      });
      return;
    }

    if (ext !== "epub") return;

    if (action === "reader") {
      await createToolWindow(windowLabel("reader"), {
        url: appPath(`/reader?file=${encoded}`),
        title: "TEpub-Editor-Reader",
        width: TOOLBOX_WINDOW_WIDTH,
        height: TOOLBOX_WINDOW_HEIGHT,
        minWidth: TOOLBOX_WINDOW_WIDTH,
        minHeight: TOOLBOX_WINDOW_HEIGHT,
        dragDropEnabled: false,
        center: true,
      });
      return;
    }

    await createToolWindow(windowLabel("epub-editor"), {
      url: appPath(`/epub-editor?file=${encoded}`),
      title: "TEpub-Editor-EPUB",
      width: TOOLBOX_WINDOW_WIDTH,
      height: TOOLBOX_WINDOW_HEIGHT,
      minWidth: TOOLBOX_WINDOW_WIDTH,
      minHeight: TOOLBOX_WINDOW_HEIGHT,
      dragDropEnabled: true,
      center: true,
    });
  }

  async function openLaunchFiles() {
    if (!isRootToolbox()) return;
    if (platform.isWeb) return;

    try {
      const launchInfo = await platform.invoke<LaunchInfo>("get_launch_info");
      const action = launchInfo?.action ?? "";
      const paths = collectLaunchPaths(launchInfo).filter(supportedLaunchPath);
      if (paths.length === 0) return;

      const key = launchSessionKey(paths, action);
      if (sessionStorage.getItem(LAUNCH_SESSION_KEY) === key) return;
      sessionStorage.setItem(LAUNCH_SESSION_KEY, key);

      for (const path of paths) {
        await openLaunchPath(path, action);
      }
      statusText = paths.length > 1 ? `已打开 ${paths.length} 个文件` : "文件已打开";
    } catch (e) {
      console.warn("处理启动文件失败:", e);
    }
  }

  async function pickFile(tool: Tool) {
    busyTool = tool.id;
    statusText = "";
    try {
      if (tool.id === "library") {
        await openLibrary();
        return;
      }
      if (tool.id === "image-tools") {
        await openImageTools();
        statusText = "封面与横幅工具已打开";
        return;
      }

      if (isBatchTool(tool.id)) {
        await openBatchWindow(tool);
        return;
      }

      const selected = await platform.openDialog<string | string[] | null>({
        multiple: false,
        filters: [
          tool.id === "txt-epub" || tool.id === "txt-edit"
            ? { name: "TXT 文件", extensions: ["txt"] }
            : { name: "EPUB 文件", extensions: ["epub"] },
        ],
      });

      if (!selected || Array.isArray(selected)) return;
      if (isBatchTool(tool.id)) {
        await openBatchWindow(tool, [selected]);
        statusText = `${tool.title} 鎵归噺绐楀彛宸叉墦寮€`;
      } else if ((tool.id as ToolId) === "epub-diagnose") {
        const result = await runEpubDiagnose(selected);
        statusText = result.issues.length === 0
          ? "EPUB 诊断未发现明显问题"
          : `EPUB 诊断完成：${result.errorCount} 错误 / ${result.warningCount} 警告`;
        await platform.message(formatDiagnosticReport(result), {
          title: tool.title,
          kind: result.errorCount > 0 ? "error" : result.warningCount > 0 ? "warning" : "info",
        });
      } else if (isProcessingTool(tool.id)) {
        let txtPath: string | undefined;
        if (tool.id === "font-decrypt") {
          const selectedTxt = await platform.openDialog<string | string[] | null>({
            multiple: false,
            filters: [{ name: "TXT 文件", extensions: ["txt"] }],
            title: "选择与 EPUB 对应的明文 TXT",
          });
          if (!selectedTxt || Array.isArray(selectedTxt)) return;
          txtPath = selectedTxt;
        }
        const result = await runProcessingTool(tool, selected, txtPath);
        statusText = result.changed ? `${tool.title} 已完成` : result.message;
        await platform.message(`${result.message}\n\n${result.outputPath}`, {
          title: tool.title,
          kind: result.changed ? "info" : "warning",
        });
      } else {
        await openToolForPath(tool, selected);
        statusText = `${tool.title} 已打开`;
      }
    } catch (e: any) {
      console.error("工具箱打开失败:", e);
      statusText = "打开失败";
      await platform.message(`打开失败: ${e}`, { title: "错误", kind: "error" });
    } finally {
      busyTool = "";
    }
  }

  function handleToolMainClick(event: MouseEvent, tool: Tool) {
    if (busyTool !== "") {
      event.preventDefault();
      return;
    }
    if (isSharedBrowserTool(tool.id) || (platform.isWeb && isWebRouteTool(tool.id))) {
      event.preventDefault();
      void goto(webToolHref(tool.id));
      return;
    }
    if (tool.id === "epub-style-library") {
      event.preventDefault();
      void openStyleLibrary();
      return;
    }
    if (tool.id === "send-to-kindle") {
      event.preventDefault();
      statusText = "正在打开 Send to Kindle";
      void platform.openExternal("https://www.amazon.com/sendtokindle");
      return;
    }
    event.preventDefault();
    pickFile(tool);
  }

  function isWebRouteTool(id: ToolId) {
    return isWebRouteToolId(id);
  }

  function isSharedBrowserTool(id: ToolId) {
    return isSharedWebTool(id);
  }

  function webToolHref(id: ToolId) {
    const route = webToolRoute(id);
    return /^https?:\/\//.test(route) || route === "#" ? route : appPath(route);
  }

  function isRootToolbox() {
    const rootPath = `${base || ""}/`;
    return typeof window !== "undefined" && window.location.pathname === rootPath;
  }

  async function openLibrary() {
    if (isRootToolbox()) {
      await goto(appPath("/library"));
      return;
    }

    try {
      const mainWin = await platform.getWindowByLabel("main");
      if (mainWin) {
        await mainWin.show();
        await mainWin.setFocus();
        await platform.closeCurrentWindow();
        return;
      }
    } catch (e) {
      console.warn("唤起主窗口失败，改为在当前窗口打开书库:", e);
    }

    await goto(appPath("/library"));
  }

  async function hideToolboxHomeWhileOpen(childWindow: PlatformWindowHandle) {
    if (appSettings.closeToolboxOnToolOpen === false) return;
    const toolboxWindow = platform.getCurrentWindow();
    try {
      childWindow.once("tauri://destroyed", async () => {
        try {
          await toolboxWindow.show();
          await toolboxWindow.setFocus();
        } catch (e) {
          console.warn("恢复工具箱主页失败:", e);
        }
      });
      await toolboxWindow.hide();
    } catch (e) {
      console.warn("隐藏工具箱主页失败:", e);
    }
  }

  async function openImageTools() {
    if (platform.isWeb && typeof window !== "undefined") {
      await goto(appPath("/toolbox/image-tools"));
      return;
    }

    const win = await createToolWindow(windowLabel("image-tools"), {
      url: appPath("/toolbox/image-tools"),
      title: "TEpub-Editor-封面与横幅",
      width: TOOLBOX_WINDOW_WIDTH,
      height: TOOLBOX_WINDOW_HEIGHT,
      minWidth: TOOLBOX_WINDOW_WIDTH,
      minHeight: TOOLBOX_WINDOW_HEIGHT,
      dragDropEnabled: true,
      center: true,
    });
    await hideToolboxHomeWhileOpen(win);
  }

  async function openStyleLibrary() {
    busyTool = "epub-style-library";
    statusText = "正在打开 EPUB 样式预览...";
    try {
      const win = await createToolWindow(windowLabel("epub-style-library"), {
        url: appPath("/toolbox/epub-style-library"),
        title: "TEpub-Editor-EPUB-Style-Library",
        width: TOOLBOX_WINDOW_WIDTH,
        height: TOOLBOX_WINDOW_HEIGHT,
        minWidth: 900,
        minHeight: 620,
        dragDropEnabled: false,
        center: true,
      });
      await hideToolboxHomeWhileOpen(win);
      statusText = "EPUB 样式预览已打开";
    } catch (e: any) {
      console.error("打开 EPUB 样式预览失败:", e);
      statusText = "打开 EPUB 样式预览失败";
      await platform.message(`打开 EPUB 样式预览失败: ${e}`, { title: "错误", kind: "error" });
    } finally {
      busyTool = "";
    }
  }

  function isProcessingTool(id: ToolId) {
    return id === "font-encrypt" || id === "font-decrypt" || id === "file-encrypt" || id === "file-decrypt" || id === "epub-reformat" || id === "image-convert";
  }

  function isBatchTool(id: ToolId) {
    return isProcessingTool(id) || id === "epub-diagnose";
  }

  function commandForTool(id: ToolId) {
    switch (id) {
      case "font-encrypt":
        return "toolbox_font_encrypt";
      case "font-decrypt":
        return "toolbox_font_decrypt";
      case "file-encrypt":
        return "toolbox_file_encrypt";
      case "file-decrypt":
        return "toolbox_file_decrypt";
      case "epub-reformat":
        return "toolbox_epub_reformat";
      case "image-convert":
        return "toolbox_image_convert";
      default:
        throw new Error(`不支持的工具: ${id}`);
    }
  }

  async function runProcessingTool(tool: Tool, filePath: string, txtPath?: string) {
    return await platform.invoke<ToolboxResult>(commandForTool(tool.id), {
      epubPath: filePath,
      txtPath,
      imageFormat: tool.id === "image-convert" ? "auto" : undefined,
    });
  }

  async function runEpubDiagnose(filePath: string) {
    return await platform.invoke<ToolboxDiagnosticResult>("toolbox_epub_diagnose", {
      epubPath: filePath,
    });
  }

  function formatDiagnosticReport(result: ToolboxDiagnosticResult) {
    const lines = [
      `文件：${result.sourcePath}`,
      `OPF：${result.opfPath || "未发现"}`,
      `ZIP 条目：${result.totalEntries}`,
      `manifest 条目：${result.manifestItems}`,
      `结果：${result.errorCount} 错误 / ${result.warningCount} 警告`,
    ];

    if (result.issues.length === 0) {
      lines.push("", "未发现明显结构问题。");
      return lines.join("\n");
    }

    lines.push("", "问题列表：");
    const visibleIssues = result.issues.slice(0, 16);
    for (const issue of visibleIssues) {
      const path = issue.path ? ` [${issue.path}]` : "";
      lines.push(`- ${issue.level}/${issue.kind}${path}: ${issue.message}`);
    }
    if (result.issues.length > visibleIssues.length) {
      lines.push(`... 还有 ${result.issues.length - visibleIssues.length} 条未显示`);
    }
    return lines.join("\n");
  }

  async function openBatchWindow(tool: Tool, inputPaths: string[] = []) {
    if (!isBatchTool(tool.id)) return;
    busyTool = tool.id;
    statusText = "";
    try {
      const taskId = `batch-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
      localStorage.setItem(
        `${BATCH_TASK_PREFIX}${taskId}`,
        JSON.stringify({
          taskId,
          tool: tool.id,
          toolTitle: tool.title,
          inputPaths,
          imageFormat: tool.id === "image-convert" ? "auto" : undefined,
        }),
      );
      const win = await createToolWindow(windowLabel("batch-progress"), {
        url: appPath(`/batch-progress?taskId=${encodeURIComponent(taskId)}&tool=${encodeURIComponent(tool.title)}`),
        title: `${tool.title} 批量处理`,
        width: TOOLBOX_WINDOW_WIDTH,
        height: TOOLBOX_WINDOW_HEIGHT,
        minWidth: TOOLBOX_WINDOW_WIDTH,
        minHeight: TOOLBOX_WINDOW_HEIGHT,
        dragDropEnabled: false,
        center: true,
      });
      await hideToolboxHomeWhileOpen(win);
      statusText = `${tool.title} 批量窗口已打开`;
    } catch (e: any) {
      console.error("批量处理失败:", e);
      statusText = "批量处理失败";
      await platform.message(`批量处理失败: ${e}`, { title: "错误", kind: "error" });
    } finally {
      busyTool = "";
    }
  }

  async function runBatchForFolder(tool: Tool) {
    await openBatchWindow(tool);
  }

  async function openToolForPath(tool: Tool, filePath: string) {
    const encoded = encodeURIComponent(filePath);
    if (tool.id === "txt-epub" || tool.id === "txt-edit") {
      const win = await createToolWindow(windowLabel("editor"), {
        url: appPath(`/editor?file=${encoded}&fromLibrary=1`),
        title: "TEpub-Editor-TXT",
        width: TOOLBOX_WINDOW_WIDTH,
        height: TOOLBOX_WINDOW_HEIGHT,
        minWidth: TOOLBOX_WINDOW_WIDTH,
        minHeight: TOOLBOX_WINDOW_HEIGHT,
        dragDropEnabled: true,
        center: true,
      });
      await hideToolboxHomeWhileOpen(win);
      return;
    }

    if (tool.id === "epub-edit") {
      const win = await createToolWindow(windowLabel("epub-editor"), {
        url: appPath(`/epub-editor?file=${encoded}`),
        title: "TEpub-Editor-EPUB",
        width: TOOLBOX_WINDOW_WIDTH,
        height: TOOLBOX_WINDOW_HEIGHT,
        minWidth: TOOLBOX_WINDOW_WIDTH,
        minHeight: TOOLBOX_WINDOW_HEIGHT,
        dragDropEnabled: true,
        center: true,
      });
      await hideToolboxHomeWhileOpen(win);
      return;
    }

    const win = await createToolWindow(windowLabel("reader"), {
      url: appPath(`/reader?file=${encoded}`),
      title: "TEpub-Editor-Reader",
      width: TOOLBOX_WINDOW_WIDTH,
      height: TOOLBOX_WINDOW_HEIGHT,
      minWidth: TOOLBOX_WINDOW_WIDTH,
      minHeight: TOOLBOX_WINDOW_HEIGHT,
      dragDropEnabled: false,
      center: true,
    });
    await hideToolboxHomeWhileOpen(win);
  }

  function closeWindow() {
    platform.closeCurrentWindow();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && !isRootToolbox()) {
      closeWindow();
    }
  }

  async function loadGlobalSettingsWithLegacy() {
    if (platform.isWeb) {
      try {
        await syncRemoteSettings();
      } catch (error) {
        console.warn("同步 Web 账号设置失败:", error);
      }
      accountSession = getWebAccountSession();
      appSettings = loadAppSettings();
      applyTheme(appSettings.uiTheme);
      return;
    }
    try {
      const data = await platform.invoke<LegacyLibraryData>("load_library");
      appSettings = saveAppSettings(loadAppSettings(data?.config || {}));
    } catch {
      appSettings = loadAppSettings();
      applyTheme(appSettings.uiTheme);
    }
  }

  function saveGlobalSettings() {
    const selectedTextProvider =
      appSettings.aiProviders.find((provider) => provider.id === appSettings.txtAiProofing.providerId && provider.kind !== "image")
      || appSettings.aiProviders.find((provider) => provider.kind !== "image");
    if (selectedTextProvider) {
      if (!appSettings.txtAiProofing.providerId) appSettings.txtAiProofing.providerId = selectedTextProvider.id;
      if (!appSettings.txtAiProofing.approvalProviderId) appSettings.txtAiProofing.approvalProviderId = selectedTextProvider.id;
      appSettings.aiProofing = providerToProofingConfig(selectedTextProvider, appSettings.aiProofing);
    }
    appSettings = saveAppSettings(appSettings);
  }

  function setUiTheme(value: string) {
    if (value !== "modern" && value !== "classic" && value !== "dark") return;
    appSettings.uiTheme = value;
    saveGlobalSettings();
  }

  function updateTocRegexRule(index: number, patch: Partial<TocRegexRule>) {
    appSettings.customRegexRules = appSettings.customRegexRules.map((rule, i) => (
      i === index ? { ...rule, ...patch } : rule
    ));
    saveGlobalSettings();
  }

  function addTocRegexRule(level: number) {
    appSettings.customRegexRules = [
      ...appSettings.customRegexRules,
      { enabled: true, level: level <= 1 ? 1 : 3, pattern: "" },
    ];
    saveGlobalSettings();
  }

  function removeTocRegexRule(index: number) {
    appSettings.customRegexRules = appSettings.customRegexRules.filter((_, i) => i !== index);
    saveGlobalSettings();
  }

  function resetTocRegexRules() {
    appSettings.customRegexRules = DEFAULT_TOC_REGEX_RULES.map((rule) => ({ ...rule }));
    saveGlobalSettings();
  }

  function providerKindLabel(kind: AiProviderConfig["kind"]) {
    return kind === "image" ? "生图模型" : "文字模型";
  }

  function openNewApiEditor() {
    apiEditorMode = "new";
    apiEditorId = "";
    apiDraft = newAiProvider({ kind: "text", name: "" });
    apiSettingsMessage = "";
    apiEditorOpen = true;
  }

  function openEditApiEditor(provider: AiProviderConfig) {
    apiEditorMode = "edit";
    apiEditorId = provider.id;
    apiDraft = { ...provider };
    apiSettingsMessage = "";
    apiEditorOpen = true;
  }

  function cancelApiEditor() {
    apiEditorOpen = false;
    apiSettingsMessage = "";
  }

  function setApiDraftKind(kind: AiProviderConfig["kind"]) {
    const currentDefault = apiDraft.kind === "image" ? "gpt-image-1" : "gpt-4o-mini";
    const nextDefault = kind === "image" ? "gpt-image-1" : "gpt-4o-mini";
    apiDraft = {
      ...apiDraft,
      kind,
      model: !apiDraft.model.trim() || apiDraft.model === currentDefault ? nextDefault : apiDraft.model,
      temperature: kind === "image" ? 0.1 : apiDraft.temperature,
    };
  }

  function normalizedApiDraft() {
    return newAiProvider({
      ...apiDraft,
      id: apiEditorMode === "edit" ? apiEditorId : apiDraft.id,
      name: apiDraft.name.trim() || (apiDraft.kind === "image" ? "生图 API" : "文字 API"),
      baseUrl: apiDraft.baseUrl.trim(),
      apiKey: apiDraft.apiKey,
      model: apiDraft.model.trim(),
      temperature: apiDraft.kind === "image" ? 0.1 : apiDraft.temperature,
    });
  }

  async function saveApiEditor() {
    const provider = normalizedApiDraft();
    if (!provider.name.trim() || !provider.baseUrl.trim() || !provider.model.trim()) {
      apiSettingsMessage = "请填写名称、API 地址和模型。";
      return;
    }
    if (apiEditorMode === "edit") {
      appSettings.aiProviders = appSettings.aiProviders.map((item) => (item.id === apiEditorId ? provider : item));
    } else {
      appSettings.aiProviders = [...appSettings.aiProviders, provider];
    }
    if (provider.kind !== "image") {
      if (!appSettings.txtAiProofing.providerId) appSettings.txtAiProofing.providerId = provider.id;
      if (!appSettings.txtAiProofing.approvalProviderId) appSettings.txtAiProofing.approvalProviderId = provider.id;
    }
    saveGlobalSettings();
    apiEditorOpen = false;
    apiSettingsMessage = "API 配置已保存。";
  }

  async function removeAiProvider(providerId: string) {
    appSettings.aiProviders = appSettings.aiProviders.filter((provider) => provider.id !== providerId);
    const fallbackId = appSettings.aiProviders.find((provider) => provider.kind !== "image")?.id || "";
    if (appSettings.txtAiProofing.providerId === providerId) appSettings.txtAiProofing.providerId = fallbackId;
    if (appSettings.txtAiProofing.approvalProviderId === providerId) appSettings.txtAiProofing.approvalProviderId = appSettings.txtAiProofing.providerId || fallbackId;
    saveGlobalSettings();
    apiSettingsMessage = "API 配置已删除。";
  }

  async function toggleFileAssoc(verb: "epub-read" | "epub-edit" | "txt-make-epub", enabled: boolean) {
    try {
      await platform.invoke("set_file_assoc", { verb, enabled });
      saveGlobalSettings();
    } catch (e: any) {
      if (verb === "epub-read") appSettings.assocEpubRead = !enabled;
      else if (verb === "epub-edit") appSettings.assocEpubEdit = !enabled;
      else appSettings.assocTxtMakeEpub = !enabled;
      await platform.message(`设置失败: ${e}`, { title: "错误", kind: "error" });
    }
  }

  async function openSettings() {
    await loadGlobalSettingsWithLegacy();
    accountSession = getWebAccountSession();
    apiEditorOpen = false;
    apiSettingsMessage = "";
    if (!toolboxSettingsTabs.some((tab) => tab.id === toolboxSettingsActiveTab)) {
      toolboxSettingsActiveTab = usesWebScopedSettings() ? "account" : "general";
    }
    showSettings = true;
  }

  function refreshWebClientViewState() {
    webDesktopViewEnabled = getWebClientViewOverride() === "desktop";
  }

  function toggleWebDesktopView() {
    if (!platform.isWeb) return;
    const nextDesktop = !webDesktopViewEnabled;
    setWebClientViewOverride(nextDesktop ? "desktop" : "");
    webDesktopViewEnabled = nextDesktop;
    statusText = nextDesktop ? "已切换为电脑页面" : "已恢复自动页面";
  }

  function setToolboxSettingsTab(tabId: string) {
    if (tabId === "account" || tabId === "general" || tabId === "assoc" || tabId === "regex" || tabId === "api") {
      if (tabId === "assoc" && usesWebScopedSettings()) return;
      toolboxSettingsActiveTab = tabId;
    }
  }

  async function submitAccountForm() {
    accountMessage = "";
    const username = accountUsername.trim();
    if (!username || !accountPassword) {
      accountMessage = "请填写账号和密码。";
      return;
    }
    try {
      if (accountMode === "register") {
        await registerWebAccount(username, accountPassword);
        accountMessage = "注册成功，已登录。";
      } else {
        await loginWebAccount(username, accountPassword);
        accountMessage = "登录成功。";
      }
      accountPassword = "";
      accountSession = getWebAccountSession();
      appSettings = loadAppSettings();
      applyTheme(appSettings.uiTheme);
    } catch (error) {
      accountMessage = String((error as Error)?.message || error);
    }
  }

  function logoutAccount() {
    logoutWebAccount();
    accountSession = null;
    appSettings = loadAppSettings();
    applyTheme(appSettings.uiTheme);
    accountMessage = "已退出登录，当前修改只保存在本次浏览器会话。";
  }

  onMount(() => {
    loadGlobalSettingsWithLegacy();
    refreshWebClientViewState();
    openLaunchFiles();
    const onSettingsUpdated = () => {
      accountSession = getWebAccountSession();
      appSettings = loadAppSettings();
    };
    window.addEventListener("tepub-settings-updated", onSettingsUpdated);
    return () => window.removeEventListener("tepub-settings-updated", onSettingsUpdated);
  });
</script>

<svelte:window on:keydown={handleKeydown} />

<main class="toolbox-app" class:desktop-view={webDesktopViewEnabled}>
  <section class="toolbox-content" aria-label="工具列表">
    <div class="toolbox-commandbar">
      <div class="toolbox-filter-tabs" role="tablist" aria-label="工具分类">
        <button type="button" class:active={activeToolGroup === "all"} on:click={() => (activeToolGroup = "all")}>全部</button>
        {#each toolGroups as group}
          <button type="button" class:active={activeToolGroup === group.id} on:click={() => (activeToolGroup = group.id as ToolGroupFilter)}>{group.title}</button>
        {/each}
      </div>
      <label class="toolbox-search">
        <span aria-hidden="true">⌕</span>
        <input type="search" bind:value={toolSearch} placeholder="搜索工具" aria-label="搜索工具" />
      </label>
      <div class="toolbox-home-actions">
        {#if platform.isWeb}
          <button
            class="toolbox-settings-btn"
            class:active={webDesktopViewEnabled}
            type="button"
            on:click={toggleWebDesktopView}
            aria-label={webDesktopViewEnabled ? "恢复自动页面" : "切换电脑页面"}
            aria-pressed={webDesktopViewEnabled}
          ><span class="desktop-view-icon" aria-hidden="true"></span></button>
        {/if}
        <button class="toolbox-settings-btn" type="button" on:click={openSettings} title="设置" aria-label="设置">⚙</button>
      </div>
    </div>
    {#if statusText}<span class="toolbox-status-announcer" aria-live="polite">{statusText}</span>{/if}

    {#if visibleToolGroups.length}
      <div class="tool-directory">
        {#each visibleToolGroups as group}
          {#each group.tools as tool}
            <div
              class="tool-card tool-card-{group.id}"
              class:tool-card-disabled={busyTool !== ""}
              aria-label={tool.title}
            >
              <a
                class="tool-main"
                href={webToolHref(tool.id)}
                aria-disabled={busyTool !== ""}
                on:click={(event) => handleToolMainClick(event, tool)}
              >
              <span class="tool-icon">{tool.icon}</span>
              <span class="tool-copy">
                <span class="tool-title">{tool.title}</span>
                <span class="tool-detail">{busyTool === tool.id ? "处理中..." : tool.detail}</span>
              </span>
              </a>
              {#if isBatchTool(tool.id) && !platform.isWeb}
                <button
                  class="tool-batch"
                  type="button"
                  on:click={() => platform.isWeb || isSharedBrowserTool(tool.id) ? goto(webToolHref(tool.id)) : runBatchForFolder(tool)}
                  disabled={busyTool !== ""}
                  title={platform.isWeb ? "选择多个文件批量处理" : "选择文件夹批量处理"}
                >
                  批量
                </button>
              {/if}
            </div>
          {/each}
        {/each}
      </div>
    {:else}
      <div class="toolbox-empty">没有匹配的工具</div>
    {/if}

    {#if platform.isWeb}
      <footer class="web-toolbox-footer" aria-label="版权信息">
        <div class="web-toolbox-footer-card">
          <div class="web-footer-line">
            <span>© 2026</span>
            <a href="https://blog.ygvlive.com" target="_blank" rel="noreferrer">源谷绘</a>
            <span>. All Rights Reserved.</span>
          </div>
          <div class="web-footer-line web-footer-powered">
            <span>Powered by</span>
            <a href="https://github.com/YGHFv/TEpub-Editor" target="_blank" rel="noreferrer">TEpub-Editor</a>
          </div>
        </div>
      </footer>
    {/if}
  </section>
</main>

{#if showSettings}
  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <!-- svelte-ignore a11y-no-static-element-interactions -->
  <div class="toolbox-settings-overlay" on:click={(e) => { if (e.target === e.currentTarget) showSettings = false; }}>
    <SettingsShell
      title="工具箱设置"
      tabs={toolboxSettingsTabs}
      activeTab={toolboxSettingsActiveTab}
      onTabChange={setToolboxSettingsTab}
      onClose={() => (showSettings = false)}
      actionLabel="完成"
      onAction={() => (showSettings = false)}
      shellClass="toolbox-settings-panel"
      contentClass="toolbox-settings-content"
    >
      {#if toolboxSettingsActiveTab === "account"}
        <section class="settings-section account-settings">
          <div class="section-title">账号</div>
          {#if accountSession}
            <p class="section-hint">
              当前账号：<strong>{accountSession.username}</strong>{accountSession.localOnly ? "（本机开发模式）" : ""}。设置会按账号独立保存。
            </p>
            <button class="toolbox-mini-btn" type="button" on:click={logoutAccount}>退出登录</button>
          {:else}
            <p class="section-hint">未登录时，Web 端设置只保存在本次浏览器会话；关闭后重新进入会恢复默认。</p>
            <div class="api-kind-switch account-mode-switch">
              <button type="button" class:active={accountMode === "login"} on:click={() => (accountMode = "login")}>登录</button>
              <button type="button" class:active={accountMode === "register"} on:click={() => (accountMode = "register")}>注册</button>
            </div>
            <div class="set-row">
              <span class="set-label">账号</span>
              <input class="set-control" type="text" bind:value={accountUsername} autocomplete="username" placeholder="3-32 位字母、数字或下划线" />
            </div>
            <div class="set-row">
              <span class="set-label">密码</span>
              <input class="set-control" type="password" bind:value={accountPassword} autocomplete={accountMode === "login" ? "current-password" : "new-password"} placeholder="至少 6 位" />
            </div>
            <button class="toolbox-mini-btn account-submit" type="button" on:click={submitAccountForm}>
              {accountMode === "login" ? "登录" : "注册并登录"}
            </button>
          {/if}
          {#if accountMessage}
            <div class="api-status">{accountMessage}</div>
          {/if}
        </section>
      {:else if toolboxSettingsActiveTab === "general"}
        <section class="settings-section">
          <div class="section-title">通用</div>
          <div class="set-row">
            <span class="set-label">界面主题</span>
            <CustomSelect className="set-control" value={appSettings.uiTheme} options={THEME_OPTIONS} on:change={(e) => setUiTheme(e.detail)} />
          </div>
          <label class="set-row toggle-row">
            <span class="set-label">打开工具时隐藏工具箱主页</span>
            <input type="checkbox" bind:checked={appSettings.closeToolboxOnToolOpen} on:change={saveGlobalSettings} />
          </label>
        </section>
      {:else if toolboxSettingsActiveTab === "assoc"}
        <section class="settings-section">
          <div class="section-title">文件关联</div>
          <p class="section-hint">注册到系统右键菜单和默认打开方式，修改时会写入注册表。</p>
          <label class="set-row toggle-row">
            <span class="set-label">EPUB 阅读 <small>(.epub)</small></span>
            <input type="checkbox" bind:checked={appSettings.assocEpubRead} on:change={(e) => toggleFileAssoc("epub-read", (e.currentTarget as HTMLInputElement).checked)} />
          </label>
          <label class="set-row toggle-row">
            <span class="set-label">EPUB 编辑 <small>(.epub)</small></span>
            <input type="checkbox" bind:checked={appSettings.assocEpubEdit} on:change={(e) => toggleFileAssoc("epub-edit", (e.currentTarget as HTMLInputElement).checked)} />
          </label>
          <label class="set-row toggle-row">
            <span class="set-label">制作 EPUB <small>(.txt)</small></span>
            <input type="checkbox" bind:checked={appSettings.assocTxtMakeEpub} on:change={(e) => toggleFileAssoc("txt-make-epub", (e.currentTarget as HTMLInputElement).checked)} />
          </label>
        </section>
      {:else if toolboxSettingsActiveTab === "regex"}
        <section class="settings-section">
          <div class="api-section-head">
            <div>
              <div class="section-title">目录正则</div>
              <p class="section-hint">用于 TXT 编辑器目录扫描和网页 EPUB 制作页；取消勾选的规则不会参与扫描。</p>
            </div>
            <button class="toolbox-mini-btn" type="button" on:click={resetTocRegexRules}>重置</button>
          </div>
          <div class="toolbox-regex-list">
            {#each appSettings.customRegexRules as rule, index}
              <div class="toolbox-regex-row">
                <label class="toolbox-regex-enabled" title="是否应用该正则">
                  <input
                    type="checkbox"
                    checked={rule.enabled !== false}
                    on:change={(e) => updateTocRegexRule(index, { enabled: (e.currentTarget as HTMLInputElement).checked })}
                  />
                </label>
                <CustomSelect
                  className="set-control toolbox-regex-level"
                  value={String(rule.level <= 1 ? 1 : 3)}
                  options={REGEX_LEVEL_OPTIONS}
                  on:change={(e) => updateTocRegexRule(index, { level: Number(e.detail) })}
                />
                <input
                  class="set-control"
                  type="text"
                  value={rule.pattern}
                  on:input={(e) => updateTocRegexRule(index, { pattern: (e.currentTarget as HTMLInputElement).value })}
                  placeholder="输入目录匹配正则"
                />
                <button class="toolbox-mini-btn danger" type="button" on:click={() => removeTocRegexRule(index)}>删除</button>
              </div>
            {/each}
          </div>
          <div class="toolbox-regex-actions">
            <button class="toolbox-mini-btn" type="button" on:click={() => addTocRegexRule(1)}>添加卷规则</button>
            <button class="toolbox-mini-btn" type="button" on:click={() => addTocRegexRule(3)}>添加章规则</button>
          </div>
        </section>
      {:else if toolboxSettingsActiveTab === "api"}
        <section class="settings-section">
          <div class="api-section-head">
            <div>
              <div class="section-title">API 配置</div>
              <p class="section-hint">保存后的文字模型用于校对和书库智能匹配，生图模型用于图片处理。</p>
            </div>
            <button class="toolbox-mini-btn" type="button" on:click={openNewApiEditor}>新增</button>
          </div>
          {#if apiSettingsMessage}
            <div class="api-status">{apiSettingsMessage}</div>
          {/if}
          {#if appSettings.aiProviders.length === 0 && !apiEditorOpen}
            <div class="api-empty">暂无 API 配置，点击“新增”添加文字模型或生图模型。</div>
          {:else}
            <div class="api-list">
              {#each appSettings.aiProviders as provider}
                <div class="api-item">
                  <div class="api-item-main">
                    <div class="api-item-title">
                      <strong>{provider.name || provider.model}</strong>
                      <span class:api-kind-image={provider.kind === "image"}>{providerKindLabel(provider.kind)}</span>
                    </div>
                    <div class="api-item-meta">
                      <span>{provider.model || "未填写模型"}</span>
                      <span>{provider.baseUrl || "未填写 API 地址"}</span>
                      <span>{provider.apiKey ? "已保存 Key" : "未填写 Key"}</span>
                    </div>
                  </div>
                  <div class="api-item-actions">
                    <button class="toolbox-mini-btn" type="button" on:click={() => openEditApiEditor(provider)}>编辑</button>
                    <button class="toolbox-mini-btn danger" type="button" on:click={() => removeAiProvider(provider.id)}>删除</button>
                  </div>
                </div>
              {/each}
            </div>
          {/if}

          {#if apiEditorOpen}
            <div class="api-editor">
              <div class="api-editor-head">
                <strong>{apiEditorMode === "new" ? "新增 API" : "编辑 API"}</strong>
                <div class="api-editor-actions">
                  <button class="toolbox-mini-btn" type="button" on:click={saveApiEditor}>保存</button>
                  <button class="toolbox-mini-btn" type="button" on:click={cancelApiEditor}>取消</button>
                </div>
              </div>
              <div class="set-row">
                <span class="set-label">模型类型</span>
                <div class="api-kind-switch">
                  <button type="button" class:active={apiDraft.kind === "text"} on:click={() => setApiDraftKind("text")}>文字模型</button>
                  <button type="button" class:active={apiDraft.kind === "image"} on:click={() => setApiDraftKind("image")}>生图模型</button>
                </div>
              </div>
              <div class="set-row">
                <span class="set-label">名称</span>
                <input class="set-control" type="text" bind:value={apiDraft.name} placeholder="例如 DeepSeek / OpenAI 生图" />
              </div>
              <div class="set-row">
                <span class="set-label">API 地址</span>
                <input class="set-control" type="text" bind:value={apiDraft.baseUrl} placeholder="https://api.openai.com/v1" />
              </div>
              <div class="set-row">
                <span class="set-label">API Key</span>
                <input class="set-control" type="password" bind:value={apiDraft.apiKey} placeholder="sk-..." />
              </div>
              <div class="set-row">
                <span class="set-label">模型</span>
                <input class="set-control" type="text" bind:value={apiDraft.model} placeholder={apiDraft.kind === "image" ? "gpt-image-1 / seedream / flux" : "gpt-4o-mini / deepseek-chat"} />
              </div>
              {#if apiDraft.kind !== "image"}
                <div class="set-row">
                  <span class="set-label">温度</span>
                  <input class="set-control" type="number" min="0" max="1" step="0.1" bind:value={apiDraft.temperature} />
                </div>
              {/if}
            </div>
          {/if}
        </section>
      {/if}
    </SettingsShell>
  </div>
{/if}

<style>
  :global(body) {
    margin: 0;
    overflow: visible;
    background: var(--color-canvas);
  }

  .toolbox-app {
    box-sizing: border-box;
    min-height: 100vh;
    min-height: 100dvh;
    display: flex;
    flex-direction: column;
    overflow: visible;
    background: var(--color-canvas);
    color: var(--color-text);
    font-family: var(--font-ui);
  }

  .toolbox-home-actions {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
    min-width: 0;
    min-height: 36px;
    margin: 0;
  }

  .toolbox-settings-btn,
  .toolbox-mini-btn {
    min-height: 32px;
    box-sizing: border-box;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
    color: var(--color-text-soft);
    cursor: pointer;
    font: inherit;
    font-size: 13px;
    line-height: 1;
  }

  .toolbox-settings-btn {
    width: 36px;
    height: 36px;
    padding: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-color: var(--color-border);
    border-radius: 8px;
    background: var(--color-surface);
    font-size: 17px;
    transition: color var(--transition-fast), border-color var(--transition-fast), background var(--transition-fast);
  }

  .toolbox-settings-btn:hover,
  .toolbox-settings-btn.active {
    border-color: var(--color-border);
    background: var(--color-hover);
    color: var(--color-accent-deep);
  }

  .desktop-view-icon {
    position: relative;
    width: 16px;
    height: 11px;
    box-sizing: border-box;
    border: 1.7px solid currentColor;
    border-radius: 2px;
  }

  .desktop-view-icon::before {
    content: "";
    position: absolute;
    left: 50%;
    bottom: -4px;
    width: 7px;
    height: 3px;
    border-bottom: 1.7px solid currentColor;
    transform: translateX(-50%);
  }

  .toolbox-mini-btn {
    flex: 0 0 auto;
    padding: 0 12px;
    font-weight: 700;
  }

  .toolbox-mini-btn.danger {
    color: var(--color-danger);
    background: var(--color-danger-soft);
  }

  .toolbox-settings-btn:hover,
  .toolbox-mini-btn:hover {
    border-color: var(--color-border-strong);
    background: var(--color-hover);
    color: var(--color-text);
  }

  .toolbox-settings-btn.active {
    border-color: var(--color-accent);
    background: var(--color-accent);
    color: #ffffff;
  }

  .toolbox-status-announcer {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }

  .toolbox-settings-overlay {
    position: fixed;
    inset: 0;
    z-index: 30;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    box-sizing: border-box;
    background: rgba(15, 23, 42, 0.28);
  }

  :global(.settings-shell.toolbox-settings-panel) {
    width: min(92vw, 860px);
    min-width: min(760px, calc(100vw - 48px));
    max-width: 860px;
    min-height: 480px;
    max-height: 84vh;
    display: grid;
    grid-template-columns: 150px minmax(0, 1fr);
    grid-template-rows: 58px minmax(0, 1fr) 64px;
  }

  :global(.settings-shell.toolbox-settings-panel .settings-shell-header) {
    grid-column: 1 / -1;
  }

  :global(.settings-shell.toolbox-settings-panel .settings-shell-tabs) {
    grid-column: 1;
    grid-row: 2 / 4;
  }

  :global(.settings-shell.toolbox-settings-panel .settings-shell-body) {
    grid-column: 2;
    grid-row: 2;
  }

  :global(.settings-shell.toolbox-settings-panel .settings-shell-footer) {
    grid-column: 2;
    grid-row: 3;
  }

  :global(.settings-shell.toolbox-settings-panel .set-row) {
    align-items: center;
  }

  .api-section-head,
  .api-editor-head,
  .api-item,
  .api-item-title,
  .api-item-actions,
  .api-editor-actions {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .api-section-head,
  .api-editor-head,
  .api-item {
    justify-content: space-between;
  }

  .api-section-head {
    margin-bottom: 12px;
  }

  .api-section-head .section-hint {
    margin-bottom: 0;
  }

  .api-status,
  .api-empty {
    box-sizing: border-box;
    margin-bottom: 12px;
    padding: 10px 12px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-surface-soft);
    color: var(--color-muted);
    font-size: 13px;
    line-height: 1.5;
  }

  .api-list {
    display: grid;
    gap: 10px;
  }

  .account-mode-switch {
    margin-bottom: 12px;
  }

  .account-submit {
    margin-top: 4px;
  }

  .toolbox-regex-list {
    display: grid;
    gap: 8px;
  }

  .toolbox-regex-row {
    display: grid;
    grid-template-columns: 28px 72px minmax(0, 1fr) auto;
    gap: 8px;
    align-items: center;
  }

  .toolbox-regex-enabled {
    min-height: 34px;
    display: grid;
    place-items: center;
  }

  .toolbox-regex-enabled input {
    width: 16px;
    height: 16px;
    accent-color: var(--color-accent);
  }

  :global(.toolbox-regex-level .custom-select-trigger) {
    min-height: 36px;
  }

  .toolbox-regex-actions {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
    margin-top: 10px;
  }

  .api-item {
    min-width: 0;
    padding: 12px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
  }

  .api-item-main {
    min-width: 0;
    display: grid;
    gap: 6px;
  }

  .api-item-title {
    justify-content: flex-start;
    min-width: 0;
  }

  .api-item-title strong {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 14px;
  }

  .api-item-title span {
    flex: 0 0 auto;
    padding: 2px 7px;
    border-radius: 999px;
    background: var(--color-accent-quiet);
    color: var(--color-accent-deep);
    font-size: 11px;
    font-weight: 800;
  }

  .api-item-title span.api-kind-image {
    background: var(--color-warning-soft);
    color: var(--color-warning);
  }

  .api-item-meta {
    min-width: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 6px 12px;
    color: var(--color-muted);
    font-size: 12px;
    line-height: 1.4;
  }

  .api-item-meta span {
    max-width: 100%;
    overflow-wrap: anywhere;
  }

  .api-editor {
    margin-top: 14px;
    padding: 12px;
    border: 1px solid var(--color-border-strong);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
  }

  .api-editor-head {
    margin-bottom: 12px;
  }

  .api-kind-switch {
    display: inline-grid;
    grid-template-columns: repeat(2, minmax(90px, 1fr));
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }

  .api-kind-switch button {
    min-height: 32px;
    padding: 0 12px;
    border: 0;
    border-radius: 0;
    background: var(--color-surface);
    color: var(--color-muted);
    font: inherit;
    font-size: 13px;
    font-weight: 800;
    cursor: pointer;
  }

  .api-kind-switch button + button {
    border-left: 1px solid var(--color-border);
  }

  .api-kind-switch button.active {
    background: var(--color-accent);
    color: white;
  }

  .toolbox-content {
    box-sizing: border-box;
    width: min(1240px, calc(100% - 48px));
    flex: 1 0 auto;
    min-height: auto;
    margin: 0 auto;
    padding: 18px 0 30px;
    display: flex;
    flex-direction: column;
    overflow: visible;
  }

  .toolbox-commandbar {
    position: sticky;
    top: 0;
    z-index: 20;
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(180px, 280px) auto;
    align-items: center;
    gap: 12px;
    min-width: 0;
    padding: 8px 0 10px;
    margin-bottom: 6px;
    background: color-mix(in srgb, var(--color-canvas) 92%, transparent);
    backdrop-filter: blur(12px);
  }

  .toolbox-filter-tabs {
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 2px;
    overflow-x: auto;
    scrollbar-width: none;
  }

  .toolbox-filter-tabs::-webkit-scrollbar {
    display: none;
  }

  .toolbox-filter-tabs button {
    flex: 0 0 auto;
    min-height: 34px;
    padding: 0 11px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--color-muted);
    cursor: pointer;
    font: inherit;
    font-size: 12px;
    font-weight: 700;
    transition: color var(--transition-fast), background var(--transition-fast);
  }

  .toolbox-filter-tabs button:hover {
    background: var(--color-hover);
    color: var(--color-text);
  }

  .toolbox-filter-tabs button.active {
    background: var(--color-surface);
    color: var(--color-accent-deep);
    box-shadow: inset 0 0 0 1px var(--color-border), var(--shadow-xs);
  }

  .toolbox-filter-tabs button:focus-visible,
  .toolbox-search:focus-within,
  .toolbox-settings-btn:focus-visible {
    outline: none;
    box-shadow: var(--focus-ring);
  }

  .toolbox-search {
    min-width: 0;
    min-height: 36px;
    box-sizing: border-box;
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 0 10px;
    border: 1px solid var(--color-border);
    border-radius: 7px;
    background: var(--color-surface);
    color: var(--color-muted);
  }

  .toolbox-search input {
    width: 100%;
    min-width: 0;
    padding: 0;
    border: 0;
    outline: 0;
    background: transparent;
    color: var(--color-text);
    font: inherit;
    font-size: 13px;
  }

  .toolbox-search input::placeholder {
    color: var(--color-muted);
  }

  .tool-directory {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 5px 14px;
    padding: 8px 0;
  }

  .tool-card {
    position: relative;
    min-height: 0;
    box-sizing: border-box;
    border: 0;
    border-radius: 7px;
    background: transparent;
    color: var(--color-text);
    text-align: left;
    box-shadow: none;
    overflow: visible;
    transition: background var(--transition-fast), box-shadow var(--transition-fast);
  }

  .tool-card:hover:not(.tool-card-disabled) {
    background: color-mix(in srgb, var(--color-surface) 78%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--color-border) 72%, transparent);
  }

  .tool-card-disabled {
    opacity: 0.68;
  }

  .tool-main {
    width: 100%;
    min-height: 62px;
    box-sizing: border-box;
    display: grid;
    grid-template-columns: 36px minmax(0, 1fr);
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border: 0;
    background: transparent;
    color: inherit;
    cursor: pointer;
    text-align: left;
    text-decoration: none;
    font: inherit;
  }

  .tool-main:focus-visible,
  .tool-batch:focus-visible {
    outline: none;
    box-shadow: var(--focus-ring);
  }

  .tool-main:disabled,
  .tool-main[aria-disabled="true"],
  .tool-batch:disabled {
    cursor: wait;
  }

  .tool-icon {
    width: 36px;
    height: 36px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-sm);
    background: var(--color-accent-quiet);
    color: var(--color-accent-deep);
    font-size: 10px;
    font-weight: 800;
    letter-spacing: 0;
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--color-accent) 10%, transparent);
  }

  .tool-card-convert .tool-icon {
    background: color-mix(in srgb, #2f8f65 12%, var(--color-surface));
    color: #24724f;
    box-shadow: inset 0 0 0 1px color-mix(in srgb, #2f8f65 18%, transparent);
  }

  .tool-card-security .tool-icon {
    background: color-mix(in srgb, #c07a20 13%, var(--color-surface));
    color: #9a5e12;
    box-shadow: inset 0 0 0 1px color-mix(in srgb, #c07a20 20%, transparent);
  }

  .tool-card-optimize .tool-icon {
    background: color-mix(in srgb, #7a6aa8 12%, var(--color-surface));
    color: #62538e;
    box-shadow: inset 0 0 0 1px color-mix(in srgb, #7a6aa8 18%, transparent);
  }

  .tool-copy {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding-right: 0;
  }

  .tool-title {
    font-size: 13px;
    font-weight: 800;
    line-height: 1.35;
    overflow-wrap: anywhere;
  }

  .tool-detail {
    color: var(--color-muted);
    font-size: 11px;
    line-height: 1.4;
    overflow-wrap: anywhere;
  }

  .tool-card:has(.tool-batch) .tool-main {
    padding-right: 50px;
  }

  .tool-batch {
    position: absolute;
    right: 13px;
    bottom: 7px;
    min-width: 34px;
    box-sizing: border-box;
    padding: 1px 5px;
    border: 0;
    border-radius: 4px;
    background: transparent;
    color: var(--color-muted);
    font: inherit;
    font-size: 10px;
    font-weight: 700;
    line-height: 1.25;
    cursor: pointer;
  }

  .tool-batch:hover:not(:disabled) {
    color: var(--color-accent-deep);
    background: var(--color-accent-quiet);
  }

  .toolbox-empty {
    padding: 64px 16px;
    color: var(--color-muted);
    text-align: center;
  }

  .web-toolbox-footer {
    margin-top: auto;
    padding-top: 24px;
    display: flex;
    justify-content: center;
  }

  .web-toolbox-footer-card {
    box-sizing: border-box;
    min-width: min(340px, 100%);
    max-width: 100%;
    padding: 8px 24px;
    border: 1px solid transparent;
    border-radius: 999px;
    background: transparent;
    box-shadow: none;
    color: color-mix(in srgb, var(--color-text) 78%, #526581);
    text-align: center;
  }

  .web-footer-line {
    display: flex;
    align-items: baseline;
    justify-content: center;
    flex-wrap: wrap;
    gap: 6px;
    font-size: 14px;
    line-height: 1.18;
    font-weight: 500;
    letter-spacing: 0;
  }

  .web-footer-powered {
    margin-top: 5px;
    color: color-mix(in srgb, var(--color-muted) 84%, var(--color-text));
    font-size: 12px;
  }

  .web-toolbox-footer a {
    color: #1677ff;
    font-weight: 800;
    text-decoration: none;
  }

  .web-toolbox-footer a:hover {
    text-decoration: underline;
  }

  :global(:root[data-tepub-client="web-mobile"]) .toolbox-app {
    min-height: 100dvh;
    height: auto;
    overflow: visible;
  }

  :global(:root[data-tepub-client="web-mobile"]) .toolbox-content {
    width: calc(100% - 20px);
    flex: 0 0 auto;
    min-height: auto;
    padding: max(10px, env(safe-area-inset-top)) 0 max(18px, env(safe-area-inset-bottom));
    overflow: visible;
  }

  :global(:root[data-tepub-client="web-mobile"]) .tool-directory {
    grid-template-columns: 1fr;
  }

  :global(:root[data-tepub-client="web-mobile"]) .tool-main {
    min-height: 70px;
  }

  :global(:root[data-tepub-client="web-mobile"]) .toolbox-commandbar {
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 8px;
  }

  :global(:root[data-tepub-client="web-mobile"]) .toolbox-filter-tabs {
    grid-column: 1 / -1;
    grid-row: 1;
  }

  @media (max-width: 980px) {
    .tool-directory {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .toolbox-commandbar {
      grid-template-columns: minmax(0, 1fr) minmax(160px, 220px) auto;
    }
  }

  @media (max-width: 640px) {
    .toolbox-content {
      width: calc(100% - 20px);
      padding: 10px 0 20px;
    }

    .toolbox-commandbar {
      grid-template-columns: minmax(0, 1fr) 68px;
      gap: 8px;
      padding-top: 4px;
    }

    .toolbox-filter-tabs {
      grid-column: 1 / -1;
      grid-row: 1;
    }

    .toolbox-search {
      grid-column: 1;
      grid-row: 2;
    }

    .toolbox-home-actions {
      grid-column: 2;
      grid-row: 2;
      justify-self: end;
    }

    .tool-directory {
      grid-template-columns: 1fr;
    }

    .tool-card {
      min-height: 0;
    }

    .tool-main {
      grid-template-columns: 38px minmax(0, 1fr);
      min-height: 66px;
      padding: 9px 11px;
    }

    .tool-icon {
      width: 38px;
      height: 38px;
    }

    .tool-copy {
      padding-right: 0;
    }

    .tool-batch {
      right: 11px;
      bottom: 5px;
    }

    .web-toolbox-footer {
      margin-top: auto;
      padding-top: 20px;
    }

    .web-toolbox-footer-card {
      min-width: 0;
      width: 100%;
      padding: 8px 16px;
      border-radius: 20px;
    }

    .web-footer-line {
      font-size: 13px;
    }

    .web-footer-powered {
      font-size: 12px;
    }

    :global(.settings-shell.toolbox-settings-panel) {
      display: flex;
      width: min(96vw, 520px);
      min-width: 0;
      min-height: 0;
      max-height: min(90vh, 720px);
    }

    :global(.settings-shell.toolbox-settings-panel .settings-shell-tabs) {
      flex-direction: row;
      grid-column: auto;
      grid-row: auto;
      overflow-x: auto;
      border-right: 0;
      border-bottom: 1px solid var(--color-border);
      padding: 10px 14px;
    }

    :global(.settings-shell.toolbox-settings-panel .settings-shell-tabs .tab-btn) {
      width: auto;
      white-space: nowrap;
    }

    :global(.settings-shell.toolbox-settings-panel .settings-shell-body),
    :global(.settings-shell.toolbox-settings-panel .settings-shell-footer) {
      grid-column: auto;
      grid-row: auto;
    }
  }

  .toolbox-app.desktop-view,
  :global(:root[data-tepub-client="web-desktop"]) .toolbox-app {
    min-width: 1200px;
    min-height: 100vh;
    height: auto;
    overflow: visible;
  }

  .toolbox-app.desktop-view .toolbox-content,
  :global(:root[data-tepub-client="web-desktop"]) .toolbox-content {
    width: min(1240px, calc(100% - 48px));
    flex: 1 0 auto;
    min-height: auto;
    padding: 18px 0 30px;
    overflow: visible;
  }

  .toolbox-app.desktop-view .web-toolbox-footer,
  :global(:root[data-tepub-client="web-desktop"]) .web-toolbox-footer {
    margin-top: auto;
    padding-top: 24px;
  }

  .toolbox-app.desktop-view .web-toolbox-footer-card,
  :global(:root[data-tepub-client="web-desktop"]) .web-toolbox-footer-card {
    min-width: min(340px, 100%);
    width: auto;
    padding: 8px 24px;
    border-radius: 999px;
  }

  .toolbox-app.desktop-view .web-footer-line,
  :global(:root[data-tepub-client="web-desktop"]) .web-footer-line {
    font-size: 14px;
  }

  :global(:root[data-tepub-client="web-desktop"] .settings-shell.toolbox-settings-panel) {
    width: min(92vw, 860px);
    min-width: min(760px, calc(100vw - 48px));
    max-width: 860px;
    min-height: 480px;
    max-height: 84vh;
    display: grid;
    grid-template-columns: 150px minmax(0, 1fr);
    grid-template-rows: 58px minmax(0, 1fr) 64px;
  }
</style>
