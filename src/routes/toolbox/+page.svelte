<script lang="ts">
  import { onMount } from "svelte";
  import { platform, type PlatformWindowHandle } from "$lib/platform";
  import CustomSelect from "$lib/CustomSelect.svelte";
  import SettingsShell from "$lib/SettingsShell.svelte";
  import {
    applyTheme,
    loadAppSettings,
    newAiProvider,
    providerToProofingConfig,
    saveAppSettings,
    type AiProviderConfig,
    type GlobalAppSettings,
  } from "$lib/appSettings";

  type ToolId =
    | "library"
    | "txt-epub"
    | "epub-edit"
    | "epub-read"
    | "font-encrypt"
    | "font-decrypt"
    | "file-encrypt"
    | "file-decrypt"
    | "epub-reformat"
    | "image-convert"
    | "image-tools"
    | "epub-diagnose";

  type Tool = {
    id: ToolId;
    icon: string;
    title: string;
    detail: string;
    action: string;
  };

  type ToolGroup = {
    id: string;
    title: string;
    meta: string;
    tools: Tool[];
    gridClass: string;
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

  const tools: Tool[] = [
    {
      id: "library",
      icon: "LIB",
      title: "书库",
      detail: "管理图书与元数据",
      action: "进入",
    },
    {
      id: "txt-epub",
      icon: "TXT",
      title: "TXT 制作 EPUB",
      detail: "选择 TXT 文件",
      action: "打开",
    },
    {
      id: "epub-edit",
      icon: "EPUB",
      title: "EPUB 编辑器",
      detail: "选择 EPUB 文件",
      action: "编辑",
    },
    {
      id: "epub-read",
      icon: "READ",
      title: "EPUB 阅读器",
      detail: "选择 EPUB 文件",
      action: "阅读",
    },
    {
      id: "font-encrypt",
      icon: "FONT+",
      title: "字体加密",
      detail: "选择 EPUB 文件",
      action: "处理",
    },
    {
      id: "font-decrypt",
      icon: "FONT-",
      title: "字体解密",
      detail: "选择 EPUB 和对应 TXT",
      action: "处理",
    },
    {
      id: "file-encrypt",
      icon: "LOCK",
      title: "文件加密",
      detail: "选择 EPUB 文件",
      action: "处理",
    },
    {
      id: "file-decrypt",
      icon: "OPEN",
      title: "文件解密",
      detail: "选择 EPUB 文件",
      action: "处理",
    },
    {
      id: "epub-reformat",
      icon: "REFIT",
      title: "EPUB 重构",
      detail: "整理目录与引用",
      action: "处理",
    },
    {
      id: "image-convert",
      icon: "IMG",
      title: "图片转换",
      detail: "转换 EPUB 内 WebP",
      action: "处理",
    },
    {
      id: "image-tools",
      icon: "COVER",
      title: "图片处理",
      detail: "制作全屏封面与阅微横幅",
      action: "打开",
    },
    {
      id: "epub-diagnose",
      icon: "CHK",
      title: "EPUB 诊断",
      detail: "检查 OPF、manifest 和内部引用",
      action: "诊断",
    },
  ];

  const toolGroups: ToolGroup[] = [
    {
      id: "open",
      title: "常用入口",
      meta: "书库 / 新窗口",
      tools: tools.filter((tool) => tool.id === "library" || tool.id === "txt-epub" || tool.id === "epub-edit" || tool.id === "epub-read"),
      gridClass: "open-grid",
    },
    {
      id: "process",
      title: "EPUB 处理",
      meta: "生成新文件",
      tools: tools.filter((tool) => tool.id === "font-encrypt" || tool.id === "font-decrypt" || tool.id === "file-encrypt" || tool.id === "file-decrypt" || tool.id === "epub-reformat" || tool.id === "image-convert" || tool.id === "image-tools" || tool.id === "epub-diagnose"),
      gridClass: "process-grid",
    },
  ];

  let busyTool: ToolId | "" = "";
  let statusText = "";
  let showSettings = false;
  let toolboxSettingsActiveTab: "general" | "assoc" | "api" = "general";
  const toolboxSettingsTabs = [
    { id: "general", label: "通用" },
    { id: "assoc", label: "文件关联" },
    { id: "api", label: "API 配置" },
  ];
  let appSettings: GlobalAppSettings = loadAppSettings();
  let apiEditorOpen = false;
  let apiEditorMode: "new" | "edit" = "new";
  let apiEditorId = "";
  let apiDraft: AiProviderConfig = newAiProvider({ kind: "text" });
  let apiSettingsMessage = "";

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
        url: `/editor?file=${encoded}&fromLibrary=1`,
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
        url: `/reader?file=${encoded}`,
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
      url: `/epub-editor?file=${encoded}`,
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
      statusText = paths.length > 1 ? `${paths.length} files opened` : "File opened";
    } catch (e) {
      console.warn("Failed to process launch files:", e);
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
        statusText = "图片处理窗口已打开";
        return;
      }

      if (isBatchTool(tool.id)) {
        await openBatchWindow(tool);
        return;
      }

      const selected = await platform.openDialog<string | string[] | null>({
        multiple: false,
        filters: [
          tool.id === "txt-epub"
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
    if (platform.isWeb && isWebRouteTool(tool.id)) return;
    event.preventDefault();
    pickFile(tool);
  }

  function isWebRouteTool(id: ToolId) {
    return id === "image-tools" || id === "txt-epub";
  }

  function webToolHref(id: ToolId) {
    if (id === "image-tools") return "/toolbox/image-tools";
    if (id === "txt-epub") return "/toolbox/make-epub?view=desktop";
    return "#";
  }

  function isRootToolbox() {
    return typeof window !== "undefined" && window.location.pathname === "/";
  }

  async function openLibrary() {
    if (isRootToolbox()) {
      window.location.href = "/library";
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

    window.location.href = "/library";
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
      window.location.href = "/toolbox/image-tools";
      return;
    }

    const win = await createToolWindow(windowLabel("image-tools"), {
      url: "/toolbox/image-tools",
      title: "TEpub-Editor-图片处理",
      width: TOOLBOX_WINDOW_WIDTH,
      height: TOOLBOX_WINDOW_HEIGHT,
      minWidth: TOOLBOX_WINDOW_WIDTH,
      minHeight: TOOLBOX_WINDOW_HEIGHT,
      dragDropEnabled: true,
      center: true,
    });
    await hideToolboxHomeWhileOpen(win);
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
        url: `/batch-progress?taskId=${encodeURIComponent(taskId)}&tool=${encodeURIComponent(tool.title)}`,
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
    if (tool.id === "txt-epub") {
      const win = await createToolWindow(windowLabel("editor"), {
        url: `/editor?file=${encoded}&fromLibrary=1`,
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
        url: `/epub-editor?file=${encoded}`,
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
      url: `/reader?file=${encoded}`,
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
    apiEditorOpen = false;
    apiSettingsMessage = "";
    if (!toolboxSettingsTabs.some((tab) => tab.id === toolboxSettingsActiveTab)) {
      toolboxSettingsActiveTab = "general";
    }
    showSettings = true;
  }

  function setToolboxSettingsTab(tabId: string) {
    if (tabId === "general" || tabId === "assoc" || tabId === "api") {
      toolboxSettingsActiveTab = tabId;
    }
  }

  onMount(() => {
    loadGlobalSettingsWithLegacy();
    openLaunchFiles();
  });
</script>

<svelte:window on:keydown={handleKeydown} />

<main class="toolbox-app">
  <section class="toolbox-content" aria-label="工具列表">
    <div class="toolbox-title-row">
      <div>
        <h1>工具箱</h1>
        <div class="toolbox-subtitle">常用工具</div>
      </div>
      <div class="toolbox-title-actions">
        {#if statusText}
          <span class="toolbox-inline-status" aria-live="polite">{statusText}</span>
        {/if}
        <button class="toolbox-settings-btn" type="button" on:click={openSettings} title="设置" aria-label="设置">⚙</button>
      </div>
    </div>

    {#each toolGroups as group}
      <section class="tool-section" aria-labelledby={`toolbox-section-${group.id}`}>
        <div class="section-head">
          <h2 id={`toolbox-section-${group.id}`}>{group.title}</h2>
          <span>{group.meta}</span>
        </div>
        <div class={`tool-grid ${group.gridClass}`}>
          {#each group.tools as tool}
            <div
              class="tool-card"
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
              <span class="tool-action">{tool.action}</span>
              </a>
              {#if isBatchTool(tool.id)}
                <button
                  class="tool-batch"
                  type="button"
                  on:click={() => runBatchForFolder(tool)}
                  disabled={busyTool !== ""}
                  title="选择文件夹批量处理"
                >
                  批量
                </button>
              {/if}
            </div>
          {/each}
        </div>
      </section>
    {/each}
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
      {#if toolboxSettingsActiveTab === "general"}
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
    overflow: hidden;
    background: var(--color-canvas);
  }

  .toolbox-app {
    box-sizing: border-box;
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--color-canvas);
    color: var(--color-text);
    font-family: var(--font-ui);
  }

  .toolbox-title-row {
    min-height: 50px;
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 20px;
    margin-bottom: 28px;
  }

  .toolbox-title-actions {
    display: flex;
    align-items: flex-start;
    justify-content: flex-end;
    gap: 10px;
    min-width: 0;
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
    width: 34px;
    padding: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 16px;
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

  .toolbox-title-row h1 {
    margin: 0;
    font-size: 22px;
    line-height: 1.2;
    font-weight: 800;
    letter-spacing: 0;
  }

  .toolbox-subtitle {
    margin-top: 4px;
    color: var(--color-muted);
    font-size: 12px;
    line-height: 1.2;
  }

  .toolbox-inline-status {
    flex-shrink: 0;
    max-width: min(420px, 42vw);
    margin-top: 4px;
    color: var(--color-muted);
    font-size: 12px;
    line-height: 1.4;
    text-align: right;
    overflow-wrap: anywhere;
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
    width: min(1100px, calc(100% - 48px));
    flex: 1;
    min-height: 0;
    margin: 0 auto;
    padding: 30px 0 34px;
    overflow: auto;
  }

  .tool-section + .tool-section {
    margin-top: 26px;
  }

  .section-head {
    height: 24px;
    margin-bottom: 10px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }

  .section-head h2 {
    margin: 0;
    color: var(--color-text);
    font-size: 14px;
    line-height: 1.3;
    font-weight: 800;
    letter-spacing: 0;
  }

  .section-head span {
    color: var(--color-muted);
    font-size: 12px;
    line-height: 1.2;
    white-space: nowrap;
  }

  .tool-grid {
    display: grid;
    gap: 12px;
  }

  .open-grid {
    grid-template-columns: repeat(4, minmax(0, 1fr));
  }

  .process-grid {
    grid-template-columns: repeat(4, minmax(0, 1fr));
  }

  .tool-card {
    position: relative;
    min-height: 104px;
    box-sizing: border-box;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
    color: var(--color-text);
    text-align: left;
    box-shadow: var(--shadow-xs);
    overflow: hidden;
    transition: border-color var(--transition-fast), background var(--transition-fast), box-shadow var(--transition-fast);
  }

  .tool-card:hover:not(.tool-card-disabled) {
    border-color: var(--color-border-strong);
    background: var(--color-hover);
    box-shadow: var(--shadow-sm);
  }

  .tool-card-disabled {
    opacity: 0.68;
  }

  .tool-main {
    width: 100%;
    min-height: 104px;
    box-sizing: border-box;
    display: grid;
    grid-template-columns: 48px minmax(0, 1fr);
    align-items: center;
    gap: 12px;
    padding: 16px 18px;
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
    width: 48px;
    height: 48px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-sm);
    background: var(--color-accent-quiet);
    color: var(--color-accent-deep);
    font-size: 11px;
    font-weight: 800;
    letter-spacing: 0;
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--color-accent) 10%, transparent);
  }

  .tool-copy {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding-right: 58px;
  }

  .tool-title {
    font-size: 15px;
    font-weight: 800;
    line-height: 1.35;
    overflow-wrap: anywhere;
  }

  .tool-detail {
    color: var(--color-muted);
    font-size: 12px;
    line-height: 1.45;
    overflow-wrap: anywhere;
  }

  .tool-action {
    position: absolute;
    top: 14px;
    right: 14px;
    min-width: 40px;
    box-sizing: border-box;
    padding: 3px 8px;
    border: 1px solid color-mix(in srgb, var(--color-accent) 22%, transparent);
    border-radius: 999px;
    background: var(--color-accent-quiet);
    color: var(--color-accent-deep);
    font-size: 12px;
    font-weight: 800;
    line-height: 1.25;
    text-align: center;
  }

  .tool-batch {
    position: absolute;
    right: 14px;
    bottom: 12px;
    min-width: 40px;
    box-sizing: border-box;
    padding: 3px 8px;
    border: 1px solid var(--color-border);
    border-radius: 999px;
    background: var(--color-surface);
    color: var(--color-muted);
    font: inherit;
    font-size: 12px;
    font-weight: 800;
    line-height: 1.25;
    cursor: pointer;
  }

  .tool-batch:hover:not(:disabled) {
    border-color: var(--color-border-strong);
    color: var(--color-text);
    background: var(--color-hover);
  }

  @media (max-width: 980px) {
    .open-grid,
    .process-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }

  @media (max-width: 640px) {
    .toolbox-content {
      width: calc(100% - 24px);
      padding: 16px 0 20px;
    }

    .toolbox-title-row {
      min-height: 0;
      flex-direction: column;
      gap: 8px;
      margin-bottom: 18px;
    }

    .toolbox-inline-status {
      max-width: 100%;
      text-align: left;
    }

    .toolbox-title-actions {
      width: 100%;
      justify-content: space-between;
    }

    .open-grid,
    .process-grid {
      grid-template-columns: 1fr;
    }

    .tool-card {
      min-height: 88px;
    }

    .tool-main {
      grid-template-columns: 44px minmax(0, 1fr);
      min-height: 88px;
      padding: 14px;
    }

    .tool-icon {
      width: 44px;
      height: 44px;
    }

    .tool-action {
      position: static;
      grid-column: 2;
      justify-self: start;
      margin-top: -2px;
    }

    .tool-copy {
      padding-right: 52px;
    }

    .tool-batch {
      right: 14px;
      bottom: 12px;
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
</style>
