<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { open, message, ask } from "@tauri-apps/plugin-dialog";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
  import LibraryGrid from "$lib/LibraryGrid.svelte";
  import LibraryListSimple from "$lib/LibraryListSimple.svelte";
  import LibraryPreview from "$lib/LibraryPreview.svelte";

  interface BookEntry {
    id: string;
    title: string;
    author: string;
    filePath: string;
    fileType: string;
    coverPath: string;
    addedAt: number;
    fileSize: number;
    subtitle: string;
    filename: string;
    createdAt?: number;
    modifiedAt?: number;
    publisher?: string;
    description?: string;
    epubUuid: string;
    maker: string;
    series: string;
    tags?: string[];
  }

  interface LibraryConfig {
    storageMode: string;
    customWorkDir: string;
    /** 编辑元数据保存时是否把 epub 内的修改时间改成"现在"，默认 false */
    updateModifiedOnEdit: boolean;
    /** 文件关联开关：是否注册 EPUB→阅读 / EPUB→编辑 / TXT→制作 EPUB */
    assocEpubRead?: boolean;
    assocEpubEdit?: boolean;
    assocTxtMakeEpub?: boolean;
    /** 入库命名模式："source"=源文件名 / "template"=按 namingTemplate 渲染 */
    namingMode?: string;
    /** 命名模板，支持 {title}{author}{subtitle}{series}{maker}{publisher}{tags} */
    namingTemplate?: string;
    /** 打开 TXT 编辑器后是否自动隐藏书库窗口 */
    closeLibraryOnTxtOpen?: boolean;
    /** 打开 EPUB 编辑器后是否自动隐藏书库窗口 */
    closeLibraryOnEpubOpen?: boolean;
    /** TXT 编辑器主窗口关闭行为："exit"=退出应用 / "library"=返回书库 */
    txtEditorCloseAction?: "exit" | "library";
  }

  interface LibraryData {
    config: LibraryConfig;
    books: BookEntry[];
  }

  let books: BookEntry[] = [];
  let libraryConfig: LibraryConfig = {
    storageMode: "copy_portable",
    customWorkDir: "",
    updateModifiedOnEdit: false,
    closeLibraryOnTxtOpen: true,
    closeLibraryOnEpubOpen: true,
    txtEditorCloseAction: "library",
  };
  let selectedBook: BookEntry | null = null;
  let viewMode: "grid" | "list-cover" | "list-simple" = "grid";
  let searchQuery = "";
  let sortColumn = "addedAt";
  let sortAsc = false;
  let isLoading = true;
  let showSettings = false;
  let showMetaEditor = false;
  let savingMetadata = false;
  let ingestStatus: "" | "ingesting" | "decrypting" = "";
  let ingestStatusTimer: ReturnType<typeof setTimeout> | null = null;

  // 首次启动书库目录选择
  interface AppModeInfo {
    isPortable: boolean;
    suggestedLibraryDir: string;
    isLibraryConfigured: boolean;
    portableMarkerPath: string;
    appDataDir: string;
  }
  let appMode: AppModeInfo | null = null;
  let firstLaunchOpen = false;
  let firstLaunchInput = "";
  let firstLaunchSaving = false;
  let coverCache: Map<string, string> = new Map();
  let contextMenuPos = { x: 0, y: 0 };
  let showContextMenu = false;
  // 已激活的标签筛选（多个 = AND 关系，全部命中才显示）
  let activeTagFilters: string[] = [];
  // 元数据编辑表单
  let metaForm = {
    title: "",
    author: "",
    subtitle: "",
    description: "",
    maker: "",
    series: "",
    tags: [] as string[],
    epubUuid: "",
  };
  let tagInput = "";   // "添加标签"输入框的当前值
  let tagPanelOpen = false;  // 点击输入框时展开的选择面板

  // 标签分类体系 —— 前三级有固定枚举，之后可自由多选
  const TAG_L1: readonly string[] = ["男频", "女频", "出版"];
  const TAG_L2: Record<string, string[]> = {
    "男频": ["玄幻", "奇幻", "武侠", "仙侠", "都市", "现实", "历史", "军事", "游戏", "体育", "科幻", "悬疑", "灵异", "无限流", "轻小说"],
    "女频": ["古代言情", "现代言情", "玄幻言情", "悬疑推理", "浪漫青春", "仙侠奇缘", "科幻空间", "游戏竞技", "现实生活", "轻小说"],
    "出版": ["文学", "小说", "散文诗歌", "历史", "传记", "哲学", "社科", "心理", "经管", "法律", "军事", "科技", "艺术", "教育", "励志", "健康", "旅游", "漫画", "童书", "工具书"],
  };
  const TAG_L3: Record<string, string[]> = {
    "男频": ["单女主", "多女主", "无女主"],
  };
  // 三级所有候选合并成一个集合，用于"是否属于固定分类"的快速判断
  const TAXONOMY_SET: Set<string> = new Set([
    ...TAG_L1,
    ...Object.values(TAG_L2).flat(),
    ...Object.values(TAG_L3).flat(),
  ]);
  // 元数据编辑面板的 UI 状态
  let productionExpanded = false;   // "制作"分组默认折叠
  let subtitleShown = false;        // 副标题字段：仅在有值或用户点击"添加"时显示
  // 入库时遇到「书名-作者.ext」同名冲突的弹框
  interface CollisionPromptState {
    open: boolean;
    filePath: string;
    suggested: string;
    inputValue: string;
    resolve: ((v: string | null) => void) | null;
  }
  let collisionPrompt: CollisionPromptState = {
    open: false, filePath: "", suggested: "", inputValue: "", resolve: null,
  };
  function promptCollisionRename(filePath: string, suggested: string): Promise<string | null> {
    return new Promise((resolve) => {
      collisionPrompt = { open: true, filePath, suggested, inputValue: suggested, resolve };
    });
  }
  function confirmCollisionPrompt() {
    const v = collisionPrompt.inputValue.trim();
    const r = collisionPrompt.resolve;
    collisionPrompt = { ...collisionPrompt, open: false, resolve: null };
    r?.(v.length > 0 ? v : null);
  }
  function cancelCollisionPrompt() {
    const r = collisionPrompt.resolve;
    collisionPrompt = { ...collisionPrompt, open: false, resolve: null };
    r?.(null);
  }

  function beginBookIngest(filePath: string) {
    if (ingestStatusTimer) {
      clearTimeout(ingestStatusTimer);
      ingestStatusTimer = null;
    }
    ingestStatus = "ingesting";
    if (/\.epub$/i.test(filePath)) {
      ingestStatusTimer = setTimeout(() => {
        ingestStatus = "decrypting";
        ingestStatusTimer = null;
      }, 1200);
    }
  }

  function endBookIngest() {
    if (ingestStatusTimer) {
      clearTimeout(ingestStatusTimer);
      ingestStatusTimer = null;
    }
    ingestStatus = "";
  }

  // 包一层：遇 BOOK_FILE_COLLISION 错误自动弹框让用户改名后重试
  async function addBookWithCollisionRetry(filePath: string): Promise<boolean> {
    let overrideFilename: string | undefined = undefined;
    for (let i = 0; i < 5; i++) {
      try {
        beginBookIngest(filePath);
        try {
          await invoke("add_book_to_library", {
            filePath,
            config: libraryConfig,
            overrideFilename,
          });
        } finally {
          endBookIngest();
        }
        return true;
      } catch (e: any) {
        const msg = String(e);
        if (msg.startsWith("BOOK_FILE_COLLISION:")) {
          const suggested = msg.substring("BOOK_FILE_COLLISION:".length);
          const userInput = await promptCollisionRename(filePath, suggested);
          if (userInput == null) return false; // 用户取消
          overrideFilename = userInput;
          continue;
        }
        throw e;
      }
    }
    throw new Error("命名冲突重试次数过多");
  }
  // 重命名本地文件弹框
  interface RenamePromptState {
    open: boolean;
    bookId: string;
    oldFilename: string;
    fileType: string;
    inputValue: string;
    error: string;
  }
  let renamePrompt: RenamePromptState = {
    open: false, bookId: "", oldFilename: "", fileType: "", inputValue: "", error: "",
  };
  function openRenameDialog(book: BookEntry) {
    const ft = book.fileType || "";
    const stem = (book.filename || "").replace(new RegExp(`\\.${ft}$`, "i"), "");
    renamePrompt = {
      open: true,
      bookId: book.id,
      oldFilename: book.filename || "",
      fileType: ft,
      inputValue: stem,
      error: "",
    };
  }
  async function confirmRenameBook() {
    const v = renamePrompt.inputValue.trim();
    if (!v) {
      renamePrompt = { ...renamePrompt, error: "文件名不能为空" };
      return;
    }
    try {
      await invoke("rename_book_file", {
        bookId: renamePrompt.bookId,
        newFilename: v,
      });
      renamePrompt = { ...renamePrompt, open: false, error: "" };
      await loadLibrary();
    } catch (e: any) {
      const msg = String(e);
      if (msg.startsWith("BOOK_FILE_COLLISION:")) {
        const suggested = msg.substring("BOOK_FILE_COLLISION:".length);
        renamePrompt = { ...renamePrompt, error: `已存在同名: ${suggested}` };
        return;
      }
      renamePrompt = { ...renamePrompt, error: `重命名失败: ${e}` };
    }
  }
  function cancelRenameBook() {
    renamePrompt = { ...renamePrompt, open: false, error: "" };
  }
  // 在系统资源管理器中打开并选中
  async function openFileLocation(book: BookEntry) {
    try {
      await invoke("reveal_in_explorer", { path: book.filePath });
    } catch (e: any) {
      await message(`打开文件位置失败: ${e}`, { title: "错误", kind: "error" });
    }
  }
  // 简洁列表列配置
  let listColumns: string[] = ["title", "author", "tags", "fileType", "fileSize", "modifiedAt"];
  // 书架设置
  // previewMode: "auto"=点击图书时显示(原默认行为)、"always"=始终显示、"never"=始终隐藏
  type PreviewMode = "auto" | "always" | "never";
  // 双击图书时的默认动作
  type DblClickAction = "read" | "edit" | "metadata";
  type UiTheme = "modern" | "classic" | "dark";
  interface ShelfSettings {
    gridShowTitle: boolean;
    gridShowAuthor: boolean;
    newBookFirst: boolean;
    previewMode: PreviewMode;
    dblClickAction: DblClickAction;
  }
  let shelfSettings: ShelfSettings = {
    gridShowTitle: true,
    gridShowAuthor: true,
    newBookFirst: true,
    previewMode: "auto",
    dblClickAction: "edit",
  };
  let appUiTheme: UiTheme = "modern";
  const VIEW_ICONS: Record<string, string> = { "grid": "▦", "list-cover": "☷", "list-simple": "☰" };

  // 预览区可见性：三态
  $: previewVisible =
    shelfSettings.previewMode === "always"
      ? true
      : shelfSettings.previewMode === "never"
        ? false
        : selectedBook !== null;

  function loadShelfSettings() {
    try {
      const saved = localStorage.getItem("shelf-settings");
      if (!saved) return;
      const parsed = JSON.parse(saved);
      // 旧版本只有 alwaysHidePreview 布尔，迁移到三态 previewMode
      if ("alwaysHidePreview" in parsed && !("previewMode" in parsed)) {
        parsed.previewMode = parsed.alwaysHidePreview ? "never" : "auto";
        delete parsed.alwaysHidePreview;
      }
      if (parsed.viewMode === "grid" || parsed.viewMode === "list-cover" || parsed.viewMode === "list-simple") {
        viewMode = parsed.viewMode;
      }
      delete parsed.viewMode;
      shelfSettings = { ...shelfSettings, ...parsed };
    } catch {}
  }

  function saveShelfSettings() {
    localStorage.setItem("shelf-settings", JSON.stringify({ ...shelfSettings, viewMode }));
  }

  function applyTheme(theme: UiTheme) {
    document.documentElement.setAttribute("data-theme", theme);
    const meta = document.querySelector('meta[name="theme-color"]');
    const colors: Record<UiTheme, string> = {
      modern: "#eef4f8",
      classic: "#f3f3f3",
      dark: "#151b23",
    };
    if (meta) meta.setAttribute("content", colors[theme]);
  }

  function loadAppUiTheme() {
    try {
      const stored = localStorage.getItem("app-settings");
      if (!stored) return;
      const parsed = JSON.parse(stored);
      if (parsed.uiTheme === "modern" || parsed.uiTheme === "classic" || parsed.uiTheme === "dark") {
        appUiTheme = parsed.uiTheme;
      }
    } catch {}
    applyTheme(appUiTheme);
  }

  function saveAppUiTheme() {
    let settings: Record<string, any> = {};
    try {
      const stored = localStorage.getItem("app-settings");
      if (stored) settings = JSON.parse(stored);
    } catch {}
    settings.uiTheme = appUiTheme;
    localStorage.setItem("app-settings", JSON.stringify(settings));
    applyTheme(appUiTheme);
  }

  function cycleViewMode() {
    if (viewMode === "grid") viewMode = "list-cover";
    else if (viewMode === "list-cover") viewMode = "list-simple";
    else viewMode = "grid";
    saveShelfSettings();
  }

  let filteredBooks: BookEntry[] = [];

  function applyFilterAndSort() {
    let result = books.filter(b => {
      // 标签筛选：AND 语义 —— 当前书必须包含所有已激活标签
      if (activeTagFilters.length > 0) {
        const bookTags = (b as any).tags as string[] | undefined;
        if (!bookTags || !activeTagFilters.every(t => bookTags.includes(t))) {
          return false;
        }
      }
      if (!searchQuery) return true;
      const q = searchQuery.toLowerCase();
      return b.title.toLowerCase().includes(q) ||
        b.author.toLowerCase().includes(q);
    });
    result.sort((a, b) => {
      let cmp = 0;
      switch (sortColumn) {
        case "title": cmp = a.title.localeCompare(b.title); break;
        case "author": cmp = a.author.localeCompare(b.author); break;
        case "subtitle": cmp = (a.subtitle || "").localeCompare(b.subtitle || ""); break;
        case "filename": cmp = (a.filename || "").localeCompare(b.filename || ""); break;
        case "fileSize": cmp = a.fileSize - b.fileSize; break;
        case "fileType": cmp = a.fileType.localeCompare(b.fileType); break;
        case "createdAt": cmp = (a.createdAt || 0) - (b.createdAt || 0); break;
        case "modifiedAt": cmp = (a.modifiedAt || 0) - (b.modifiedAt || 0); break;
        case "addedAt": default: cmp = a.addedAt - b.addedAt; break;
      }
      return sortAsc ? cmp : -cmp;
    });
    filteredBooks = result;
  }

  $: {
    books; searchQuery; sortColumn; sortAsc; activeTagFilters;
    applyFilterAndSort();
  }

  // 切换某个标签的激活状态（多次点同一标签 = 反复加入/移除筛选）
  function toggleTagFilter(tag: string) {
    if (activeTagFilters.includes(tag)) {
      activeTagFilters = activeTagFilters.filter(t => t !== tag);
    } else {
      activeTagFilters = [...activeTagFilters, tag];
    }
  }

  function clearTagFilters() {
    activeTagFilters = [];
  }

  // 启动入口：先确认应用模式 + 是否首次启动
  async function bootLibrary() {
    try {
      appMode = await invoke<AppModeInfo>("get_app_mode_info");
    } catch (e) {
      console.error("获取应用模式失败:", e);
      // 取不到也别卡死，按"已配置"流程走
      appMode = { isPortable: false, suggestedLibraryDir: "", isLibraryConfigured: true, portableMarkerPath: "", appDataDir: "" };
    }
    if (!appMode.isLibraryConfigured) {
      firstLaunchInput = appMode.isPortable ? appMode.suggestedLibraryDir : "";
      firstLaunchOpen = true;
      isLoading = false; // 解开 loading 让弹窗能显示
      return;
    }
    await loadLibrary();
  }

  async function browseFirstLaunchDir() {
    try {
      const selected = await open({ directory: true, multiple: false });
      if (typeof selected === "string") {
        firstLaunchInput = selected;
      }
    } catch (e) {
      console.error("选择目录失败:", e);
    }
  }

  async function confirmFirstLaunchDir() {
    const trimmed = firstLaunchInput.trim();
    if (!trimmed) {
      await message("请选择书库存放目录", { title: "需要选择目录", kind: "warning" });
      return;
    }
    if (firstLaunchSaving) return;
    firstLaunchSaving = true;
    try {
      libraryConfig = {
        storageMode: appMode?.isPortable ? "copy_portable" : "copy_custom",
        customWorkDir: trimmed,
        updateModifiedOnEdit: libraryConfig.updateModifiedOnEdit ?? false,
        closeLibraryOnTxtOpen: libraryConfig.closeLibraryOnTxtOpen ?? true,
        closeLibraryOnEpubOpen: libraryConfig.closeLibraryOnEpubOpen ?? true,
        txtEditorCloseAction: libraryConfig.txtEditorCloseAction ?? "library",
      };
      // 写一份空 library.json 到该目录，建立 pointer
      await invoke("save_library", { data: { config: libraryConfig, books: [] } });
      firstLaunchOpen = false;
      isLoading = true;
      await loadLibrary();
      try { appMode = await invoke<AppModeInfo>("get_app_mode_info"); } catch {}
    } catch (e: any) {
      console.error("保存书库目录失败:", e);
      await message(`保存失败: ${e}`, { title: "错误", kind: "error" });
    } finally {
      firstLaunchSaving = false;
    }
  }

  async function loadLibrary() {
    try {
      const data = await invoke<LibraryData>("load_library");
      books = (data.books || []).map(b => {
        if (!b.filename && b.filePath) {
          const segs = b.filePath.replace(/\\/g, "/").split("/");
          b.filename = segs[segs.length - 1] || "";
        }
        return b;
      });
      libraryConfig = data.config || { storageMode: "copy_portable", customWorkDir: "", updateModifiedOnEdit: false };
      // 兼容旧的 library.json（缺字段时塞默认）
      if (typeof libraryConfig.updateModifiedOnEdit !== "boolean") {
        libraryConfig.updateModifiedOnEdit = false;
      }
      if (!libraryConfig.namingMode) libraryConfig.namingMode = "template";
      if (!libraryConfig.namingTemplate) libraryConfig.namingTemplate = "{title}-{author}";
      if (typeof libraryConfig.closeLibraryOnTxtOpen !== "boolean") libraryConfig.closeLibraryOnTxtOpen = true;
      if (typeof libraryConfig.closeLibraryOnEpubOpen !== "boolean") libraryConfig.closeLibraryOnEpubOpen = true;
      if (libraryConfig.txtEditorCloseAction !== "exit" && libraryConfig.txtEditorCloseAction !== "library") {
        libraryConfig.txtEditorCloseAction = "library";
      }
      syncNamingPresetFromConfig();
      // 安装版没有 copy_portable 选项，旧配置或异常状态下自动迁移到 copy_custom
      if (appMode && !appMode.isPortable && libraryConfig.storageMode === "copy_portable") {
        libraryConfig.storageMode = "copy_custom";
        // 持久化迁移
        try { await saveLibraryConfig(); } catch {}
      }
      // 同步存储模式跟踪变量
      prevStorageMode = libraryConfig.storageMode;
      // 预加载封面
      for (const book of books) {
        if (book.coverPath && !coverCache.has(book.id)) {
          loadCover(book);
        }
      }
    } catch (e) {
      console.error("加载书库失败:", e);
    } finally {
      isLoading = false;
    }
  }

  async function loadCover(book: BookEntry) {
    if (!book.coverPath || coverCache.has(book.id)) return;
    try {
      const data = await invoke<number[]>("get_library_cover_data", { coverPath: book.coverPath });
      // 按 coverPath 扩展名推断 MIME，否则默认 jpeg
      const lower = book.coverPath.toLowerCase();
      let mime = "image/jpeg";
      if (lower.endsWith(".png")) mime = "image/png";
      else if (lower.endsWith(".webp")) mime = "image/webp";
      else if (lower.endsWith(".gif")) mime = "image/gif";
      const blob = new Blob([new Uint8Array(data)], { type: mime });
      coverCache.set(book.id, URL.createObjectURL(blob));
      coverCache = coverCache; // trigger reactivity
    } catch {
      coverCache.set(book.id, "");
    }
  }

  async function addBook() {
    let selected: string | string[] | null = null;
    try {
      selected = await open({
        filters: [{ name: "图书文件", extensions: ["epub", "txt"] }],
        multiple: true,
      });
    } catch (e: any) {
      console.error("打开文件对话框失败:", e);
      return;
    }
    if (!selected) return;
    const paths = Array.isArray(selected) ? selected : [selected];
    if (paths.length === 0) return;

    const failures: string[] = [];
    let added = 0;
    for (const p of paths) {
      try {
        const ok = await addBookWithCollisionRetry(p);
        if (ok) added++;
      } catch (e: any) {
        const name = p.replace(/\\/g, "/").split("/").pop() || p;
        failures.push(`${name}: ${e}`);
      }
    }
    await loadLibrary();
    if (failures.length > 0) {
      const head = added > 0
        ? `成功添加 ${added} 本，${failures.length} 本失败:\n`
        : `添加失败 (${failures.length}):\n`;
      await message(head + failures.join("\n"), {
        title: "添加图书",
        kind: added > 0 ? "warning" : "error",
      });
    }
  }

  // 打开文件 —— 直接进入编辑器，不加入书架；支持一次选择多个文件，每个开一个独立编辑器窗口
  async function openExternalFiles() {
    let selected: string | string[] | null = null;
    try {
      selected = await open({
        filters: [{ name: "图书文件", extensions: ["epub", "txt"] }],
        multiple: true,
      });
    } catch (e: any) {
      console.error("打开文件对话框失败:", e);
      return;
    }
    if (!selected) return;
    const paths = Array.isArray(selected) ? selected : [selected];
    if (paths.length === 0) return;

    // 仅一个文件时沿用原有逻辑（隐藏主窗、关闭后还原）
    if (paths.length === 1) {
      await openFilePathInEditor(paths[0], { hideMain: true });
      return;
    }
    // 多文件：每个独立窗口，主窗不隐藏，方便用户继续操作
    for (const p of paths) {
      await openFilePathInEditor(p, { hideMain: false });
    }
  }

  // 用一个文件路径打开编辑器；可选地隐藏主窗并在编辑器关闭时还原
  async function openFilePathInEditor(filePath: string, opts: { hideMain: boolean }) {
    const ext = filePath.split(".").pop()?.toLowerCase();
    const isEpub = ext === "epub";
    const encoded = encodeURIComponent(filePath);
    const url = isEpub ? `/epub-editor?file=${encoded}` : `/editor?file=${encoded}&fromLibrary=1`;
    const title = isEpub ? "TEpub-Editor-EPUB" : "TEpub-Editor-TXT";

    try {
      const appWindow = getCurrentWindow();
      const win = new WebviewWindow(`editor-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`, {
        url,
        title,
        width: isEpub ? 1200 : 1200,
        height: isEpub ? 740 : 740,
        dragDropEnabled: true,
        center: true,
      });

      const shouldHideMain = opts.hideMain && (isEpub ? libraryConfig.closeLibraryOnEpubOpen !== false : libraryConfig.closeLibraryOnTxtOpen !== false);
      if (shouldHideMain) {
        await appWindow.hide();
        win.once("tauri://destroyed", async () => {
          await appWindow.show();
          await appWindow.setFocus();
        });
      }
    } catch (e: any) {
      console.error("打开编辑器失败:", e);
      await message(`打开失败: ${e}`, { title: "错误", kind: "error" });
    }
  }

  async function removeBook(book: BookEntry) {
    try {
      await invoke("remove_book_from_library", { bookId: book.id });
      if (selectedBook?.id === book.id) selectedBook = null;
      await loadLibrary();
    } catch (e: any) {
      console.error("移除图书失败:", e);
      await message(`移除图书失败: ${e}`, { title: "错误", kind: "error" });
    }
  }

  async function openBook(book: BookEntry) {
    const encoded = encodeURIComponent(book.filePath);
    const isEpub = book.fileType === "epub";
    const url = isEpub ? `/epub-editor?file=${encoded}` : `/editor?file=${encoded}&fromLibrary=1`;
    const title = isEpub ? "TEpub-Editor-EPUB" : "TEpub-Editor-TXT";

    try {
      const appWindow = getCurrentWindow();
      const win = new WebviewWindow(`editor-${Date.now()}`, {
        url,
        title,
        width: isEpub ? 1200 : 1200,
        height: isEpub ? 740 : 740,
        dragDropEnabled: true,
        center: true,
      });

      const shouldHideMain = isEpub ? libraryConfig.closeLibraryOnEpubOpen !== false : libraryConfig.closeLibraryOnTxtOpen !== false;
      if (shouldHideMain) {
        await appWindow.hide();

        win.once("tauri://destroyed", async () => {
          await appWindow.show();
          await appWindow.setFocus();
        });
      }
    } catch (e: any) {
      console.error("打开编辑器失败:", e);
      await message(`打开失败: ${e}`, { title: "错误", kind: "error" });
    }
  }

  // 阅读（EPUB 走自定义阅读器；TXT 暂时回退到 TXT 编辑器）
  async function openReader(book: BookEntry) {
    const isEpub = book.fileType === "epub";
    if (!isEpub) {
      // TXT 阅读器尚未实现，直接走编辑器
      await openBook(book);
      return;
    }
    const encoded = encodeURIComponent(book.filePath);
    const url = `/reader?file=${encoded}`;
    const title = `${book.title || "阅读"} · 阅读`;

    try {
      const appWindow = getCurrentWindow();
      const win = new WebviewWindow(`reader-${Date.now()}`, {
        url,
        title,
        // 阅读器窗口纵横比 高:宽 = 8:5（严格：500*8/5=800）。
        // 选 800 是为了在 1366x768 这类老笔记本上仍能完整显示
        // （减去任务栏 + 标题栏后可用高度通常 ≥ 680，800 接近上限），
        // 1080p 屏幕上更是绰绰有余。
        width: 500,
        height: 800,
        dragDropEnabled: false,
        center: true,
      });

      await appWindow.hide();

      win.once("tauri://destroyed", async () => {
        await appWindow.show();
        await appWindow.setFocus();
      });
    } catch (e: any) {
      console.error("打开阅读器失败:", e);
      await message(`打开失败: ${e}`, { title: "错误", kind: "error" });
    }
  }

  // 简介在 textarea 里显示带首行缩进（每段两个全角空格），与右侧预览的
  // splitDescription 完全一致：按 \n+ 切段、trim、过滤空行，每段前置两个全角空格。
  // 注意 Rust 端 extract_first_tag 会对 dc:description 做 trim()（包含 U+3000），
  // 因此第一段的缩进通常被剥掉，由前端在显示时统一补回。
  // 保存时再剥掉首行的全角空格还原成纯文本写回 OPF。
  const DESC_INDENT = "　　";
  function descToForm(desc: string): string {
    if (!desc) return "";
    const paras = desc
      .split(/\r?\n+/)
      .map(p => p.replace(/^[　\s ]+/, "").replace(/[　\s ]+$/, ""))
      .filter(p => p.length > 0);
    if (paras.length === 0) return "";
    return paras.map(p => DESC_INDENT + p).join("\n");
  }
  function descFromForm(form: string): string {
    if (!form) return "";
    return form
      .split(/\r?\n/)
      .map(line => line.replace(/^[　\s ]+/, ""))
      .join("\n");
  }

  function openEditMetadata(book: BookEntry) {
    metaForm = {
      title: book.title,
      author: book.author,
      subtitle: book.subtitle || "",
      description: descToForm(book.description || ""),
      maker: book.maker || "",
      series: book.series || "",
      tags: Array.isArray((book as any).tags) ? [...(book as any).tags] : [],
      epubUuid: book.epubUuid || "",
    };
    tagInput = "";
    // 副标题为空时默认隐藏；制作信息默认折叠（保持面板紧凑，无内容才展开会显得空）
    subtitleShown = !!metaForm.subtitle;
    productionExpanded = false;
    showMetaEditor = true;
  }

  async function saveMetadata() {
    if (!selectedBook) return;
    if (savingMetadata) return;
    savingMetadata = true;
    try {
      const updated = await invoke<BookEntry>("update_book_metadata", {
        bookId: selectedBook.id,
        title: metaForm.title,
        author: metaForm.author,
        subtitle: metaForm.subtitle,
        description: descFromForm(metaForm.description),
        maker: metaForm.maker,
        series: metaForm.series,
        tags: metaForm.tags,
        epubUuid: metaForm.epubUuid.trim(),
      });
      // 更新本地数据
      const idx = books.findIndex(b => b.id === selectedBook!.id);
      if (idx >= 0) books[idx] = updated;
      selectedBook = updated;
      showMetaEditor = false;
    } catch (e: any) {
      console.error("保存元数据失败:", e);
      await message(`保存失败: ${e}`, { title: "错误", kind: "error" });
    } finally {
      savingMetadata = false;
    }
  }

  // 标签编辑：添加 / 删除
  function commitTag() {
    const value = tagInput.trim();
    if (!value) return;
    // 支持英文/中文逗号一次输入多个，自动拆分；去空、去重
    const parts = value.split(/[,，]/).map(s => s.trim()).filter(Boolean);
    for (const p of parts) {
      if (!metaForm.tags.includes(p)) {
        metaForm.tags = [...metaForm.tags, p];
      }
    }
    tagInput = "";
  }

  function removeTag(index: number) {
    metaForm.tags = metaForm.tags.filter((_, i) => i !== index);
  }

  // 用 tag 名删除（chip × 不再依赖 index，避免排序后错位）
  function removeTagByName(name: string) {
    metaForm.tags = metaForm.tags.filter(t => t !== name);
  }

  function onTagKeydown(e: KeyboardEvent) {
    if (e.key === "Enter" || e.key === "," || e.key === "，") {
      e.preventDefault();
      commitTag();
    } else if (e.key === "Backspace" && !tagInput && metaForm.tags.length > 0) {
      // 在空输入状态下按退格 → 删除最后一个标签（常见标签输入交互）
      e.preventDefault();
      metaForm.tags = metaForm.tags.slice(0, -1);
    } else if (e.key === "Escape") {
      tagPanelOpen = false;
      (e.currentTarget as HTMLElement).blur();
    }
  }

  // ----- 标签分级派生：从 metaForm.tags 推算当前 L1/L2/L3 选中值 -----
  $: currentL1 = metaForm.tags.find(t => TAG_L1.includes(t)) || "";
  $: currentL2List = currentL1 ? (TAG_L2[currentL1] || []) : [];
  $: currentL2 = metaForm.tags.find(t => currentL2List.includes(t)) || "";
  $: currentL3List = currentL1 && TAG_L3[currentL1] ? TAG_L3[currentL1] : [];
  $: currentL3 = metaForm.tags.find(t => currentL3List.includes(t)) || "";
  // 自定义标签 = 不属于任何固定分级
  $: customTagsOnBook = metaForm.tags.filter(t => !TAXONOMY_SET.has(t));
  // chip 显示顺序：L1 → L2 → L3 → 其他（保留用户加入的相对顺序）
  $: orderedTagsForDisplay = [
    ...(currentL1 ? [currentL1] : []),
    ...(currentL2 ? [currentL2] : []),
    ...(currentL3 ? [currentL3] : []),
    ...customTagsOnBook,
  ];

  // 前三级是否选完：男频需 L1+L2+L3，女频/出版需 L1+L2
  $: tiersComplete = (() => {
    if (!currentL1) return false;
    if (currentL1 === "男频") return !!(currentL2 && currentL3);
    return !!currentL2; // 女频 / 出版
  })();

  // 面板两态：
  //   Phase 1（"选分类"）：未选完 且 没在输入 → 只展示前三级单选
  //   Phase 2（"加自定义"）：选完 或 正在输入 → 只展示自定义建议（按输入筛选）
  $: tagPanelPhase = (tiersComplete || tagInput.trim()) ? "custom" : "tiers";

  // 全库其他书籍中已使用过的"自定义"标签，用于自动建议
  $: librarySuggestions = (() => {
    const seen = new Set<string>();
    for (const b of books) {
      const ts = (b as any).tags as string[] | undefined;
      if (!ts) continue;
      for (const t of ts) {
        if (!TAXONOMY_SET.has(t)) seen.add(t);
      }
    }
    // 排除已在当前书上的标签
    for (const t of metaForm.tags) seen.delete(t);
    return Array.from(seen).sort((a, b) => a.localeCompare(b, "zh"));
  })();

  // 输入框筛选后的建议列表
  $: filteredSuggestions = (() => {
    const q = tagInput.trim().toLowerCase();
    if (!q) return librarySuggestions;
    return librarySuggestions.filter(t => t.toLowerCase().includes(q));
  })();

  // 切换 L1：替换原有 L1；并清掉对新 L1 不再适用的 L2/L3
  function selectL1(value: string) {
    let next = metaForm.tags.filter(t => !TAG_L1.includes(t));
    const newL2List = TAG_L2[value] || [];
    const newL3List = TAG_L3[value] || [];
    // 移除属于"旧 L1"专属的 L2/L3：凡是 TAXONOMY 但既不在新 L2 也不在新 L3 的，删
    next = next.filter(t => {
      if (!TAXONOMY_SET.has(t)) return true;
      if (TAG_L1.includes(t)) return false; // 已在前面过滤
      return newL2List.includes(t) || newL3List.includes(t);
    });
    if (currentL1 !== value) {
      // 选中新的 L1，把它放到最前
      next = [value, ...next];
    }
    // 若再次点击当前 L1 则视为取消（next 已经移除该 L1）
    metaForm.tags = next;
  }

  function selectL2(value: string) {
    // 同一 L1 下只能选一个 L2 → 先去掉所有同级，再决定是否加回
    let next = metaForm.tags.filter(t => !currentL2List.includes(t));
    if (currentL2 !== value) next = [...next, value];
    metaForm.tags = next;
  }

  function selectL3(value: string) {
    let next = metaForm.tags.filter(t => !currentL3List.includes(t));
    if (currentL3 !== value) next = [...next, value];
    metaForm.tags = next;
  }

  function addSuggestion(value: string) {
    if (!metaForm.tags.includes(value)) {
      metaForm.tags = [...metaForm.tags, value];
    }
  }

  // 整个 .tags-editor 失去焦点时关闭面板（点击其内部按钮不会触发，因为焦点仍在容器内）
  function onTagsEditorFocusOut(e: FocusEvent) {
    const ct = e.currentTarget as HTMLElement;
    const next = e.relatedTarget as Node | null;
    if (!next || !ct.contains(next)) {
      // 给点击事件留一拍时间生效
      setTimeout(() => {
        if (!document.activeElement || !ct.contains(document.activeElement)) {
          // 还要把已输入但未提交的文字落地
          commitTag();
          tagPanelOpen = false;
        }
      }, 0);
    }
  }

  // 重新从 EPUB 文件提取元数据（用于补全早期添加时缺失的 UUID 等字段）
  let refreshing = false;
  async function refreshMetadata() {
    if (!selectedBook || refreshing) return;
    refreshing = true;
    try {
      const updated = await invoke<BookEntry>("refresh_book_metadata", {
        bookId: selectedBook.id,
      });
      const idx = books.findIndex(b => b.id === selectedBook!.id);
      if (idx >= 0) books[idx] = updated;
      selectedBook = updated;
      // 同步表单中已为空的字段（不覆盖用户正在输入的内容）
      if (!metaForm.title) metaForm.title = updated.title;
      if (!metaForm.author) metaForm.author = updated.author;
      if (!metaForm.description) metaForm.description = descToForm(updated.description || "");
      if (!metaForm.epubUuid) metaForm.epubUuid = updated.epubUuid || "";
    } catch (e: any) {
      console.error("重新提取元数据失败:", e);
      await message(`重新提取失败: ${e}`, { title: "错误", kind: "error" });
    } finally {
      refreshing = false;
    }
  }

  // 把 Unix 秒转成"YYYY-MM-DD HH:mm"字符串（用于"制作时间/修改时间"展示）
  function formatDateTime(ts: number | undefined): string {
    if (!ts) return "-";
    const d = new Date(ts * 1000);
    const y = d.getFullYear();
    const m = String(d.getMonth() + 1).padStart(2, "0");
    const day = String(d.getDate()).padStart(2, "0");
    const h = String(d.getHours()).padStart(2, "0");
    const min = String(d.getMinutes()).padStart(2, "0");
    return `${y}-${m}-${day} ${h}:${min}`;
  }

  async function changeCover() {
    if (!selectedBook) return;
    try {
      const selected = await open({
        filters: [{ name: "图片文件", extensions: ["jpg", "jpeg", "png", "gif"] }],
        multiple: false,
      });
      if (!selected) return;
      const data = await invoke<number[]>("read_binary_file", { path: selected });
      const newPath = await invoke<string>("update_book_cover", {
        bookId: selectedBook.id,
        coverData: data,
      });
      // 更新封面缓存
      const blob = new Blob([new Uint8Array(data)], { type: "image/jpeg" });
      if (coverCache.has(selectedBook.id)) {
        URL.revokeObjectURL(coverCache.get(selectedBook.id)!);
      }
      coverCache.set(selectedBook.id, URL.createObjectURL(blob));
      coverCache = coverCache;
      // 更新 selectedBook 和 books 中的引用
      selectedBook.coverPath = newPath;
      const idx = books.findIndex(b => b.id === selectedBook!.id);
      if (idx >= 0) books[idx].coverPath = newPath;
    } catch (e: any) {
      console.error("更换封面失败:", e);
      await message(`更换封面失败: ${e}`, { title: "错误", kind: "error" });
    }
  }

  // 复制到剪贴板，并显示一个短暂的视觉反馈
  let copyFeedbackKey = "";
  async function copyText(text: string, key = "default") {
    if (!text) return;
    try {
      await navigator.clipboard.writeText(text);
      copyFeedbackKey = key;
      setTimeout(() => { if (copyFeedbackKey === key) copyFeedbackKey = ""; }, 1200);
    } catch (e) {
      console.error("复制失败:", e);
    }
  }

  function handleContextMenu(e: MouseEvent, book: BookEntry) {
    e.preventDefault();
    selectedBook = book;
    contextMenuPos = { x: e.clientX, y: e.clientY };
    showContextMenu = true;
  }

  function closeContextMenu() {
    showContextMenu = false;
  }

  function handleBookClick(book: BookEntry) {
    selectedBook = book;
  }

  // 点击非图书区域时取消选中（依靠 data-context-type 判断，三种视图组件都已标记）
  function handleBrowserAreaClick(e: MouseEvent) {
    const target = e.target as HTMLElement | null;
    if (!target) return;
    if (!target.closest('[data-context-type="library-book"]')) {
      selectedBook = null;
    }
  }

  function handleBookOpen(book: BookEntry) {
    // 列表/网格双击或回车触发，按设置分派
    switch (shelfSettings.dblClickAction) {
      case "edit":
        openBook(book);
        break;
      case "metadata":
        openEditMetadata(book);
        break;
      case "read":
      default:
        openReader(book);
        break;
    }
  }

  function handleSort(col: string) {
    if (sortColumn === col) {
      sortAsc = !sortAsc;
    } else {
      sortColumn = col;
      sortAsc = false;
    }
  }

  async function handleDragDrop(event: DragEvent) {
    event.preventDefault();
    const files = event.dataTransfer?.files;
    if (!files) return;
    for (let i = 0; i < files.length; i++) {
      const path = (files[i] as any).path;
      if (path) {
        try {
          await addBookWithCollisionRetry(path);
        } catch (e) {
          console.error("拖放添加失败:", e);
        }
      }
    }
    await loadLibrary();
  }

  function formatFileSize(bytes: number): string {
    if (bytes < 1024) return bytes + " B";
    if (bytes < 1048576) return (bytes / 1024).toFixed(1) + " KB";
    return (bytes / 1048576).toFixed(1) + " MB";
  }

  async function saveLibraryConfig() {
    try {
      const data = await invoke<LibraryData>("load_library");
      await invoke("save_library", {
        data: { ...data, config: libraryConfig }
      });
    } catch (e) {
      console.error("保存设置失败:", e);
    }
  }

  // ---- 入库命名模板 ----
  const NAMING_PRESETS: { label: string; value: string }[] = [
    { label: "{title}-{author}", value: "{title}-{author}" },
    { label: "{title}[{author}]", value: "{title}[{author}]" },
    { label: "{title}[{author}]{tags}", value: "{title}[{author}]{tags}" },
    { label: "{series}-{title}[{author}]", value: "{series}-{title}[{author}]" },
    { label: "{series}-{title}[{author}][{maker}]{tags}", value: "{series}-{title}[{author}][{maker}]{tags}" },
  ];
  // 当前下拉选中的预设；若库里存的模板不在预设里就显示 "custom"
  let namingPresetSel: string = "{title}-{author}";
  // 同步：libraryConfig 变 → 调整预设下拉
  function syncNamingPresetFromConfig() {
    const t = (libraryConfig.namingTemplate || "").trim();
    if (!t) {
      namingPresetSel = "{title}-{author}";
      return;
    }
    namingPresetSel = NAMING_PRESETS.some(p => p.value === t) ? t : "custom";
  }
  // 命名模式或模板变更 → 保存配置 + 询问是否立即批量重命名
  async function onNamingChanged() {
    await saveLibraryConfig();
    const localCount = books.filter(b =>
      libraryConfig.customWorkDir
        ? b.filePath.startsWith(libraryConfig.customWorkDir)
        : true
    ).length;
    if (localCount === 0) return;
    const yes = await ask(
      `是否立即按新规则重命名书库中的 ${localCount} 本本地书文件？\n（仅会重命名复制进书库的副本，引用模式下的原文件不会动）`,
      { kind: "info", title: "应用新命名" },
    );
    if (!yes) return;
    try {
      const summary = await invoke<{ renamed: number; skipped: number; failed: number; failures: string[] }>(
        "rebuild_book_filenames",
        { config: libraryConfig },
      );
      let msg = `已重命名 ${summary.renamed} 本，跳过 ${summary.skipped} 本`;
      if (summary.failed > 0) {
        msg += `，失败 ${summary.failed} 本：\n` + summary.failures.slice(0, 5).join("\n");
      }
      await message(msg, { title: "重命名完成", kind: summary.failed > 0 ? "warning" : "info" });
      await loadLibrary();
    } catch (e: any) {
      await message(`重命名失败: ${e}`, { title: "错误", kind: "error" });
    }
  }
  function onNamingPresetChange(v: string) {
    namingPresetSel = v;
    if (v === "custom") {
      // 切到自定义但保留当前模板，让用户继续编辑
      if (!libraryConfig.namingTemplate) libraryConfig.namingTemplate = "{title}-{author}";
    } else {
      libraryConfig.namingTemplate = v;
    }
    onNamingChanged();
  }

  // 跟踪上一次成功设置的 storageMode，用于"切到 copy_custom 但取消选目录"时回滚
  let prevStorageMode = libraryConfig.storageMode;

  async function pickCustomWorkDir(): Promise<boolean> {
    try {
      const selected = await open({ directory: true, multiple: false });
      if (typeof selected === "string" && selected.trim()) {
        libraryConfig.customWorkDir = selected;
        await saveLibraryConfig();
        return true;
      }
      return false;
    } catch (e) {
      console.error("选择目录失败:", e);
      return false;
    }
  }

  async function onStorageModeChange() {
    const newMode = libraryConfig.storageMode;
    // 第一次切到 copy_custom 自动弹文件夹选择器；取消则回滚到之前的模式
    if (newMode === "copy_custom" && prevStorageMode !== "copy_custom") {
      const ok = await pickCustomWorkDir();
      if (!ok) {
        libraryConfig.storageMode = prevStorageMode;
        return;
      }
    }
    prevStorageMode = newMode;
    await saveLibraryConfig();
  }

  // 文件关联：写注册表 + 持久化到 libraryConfig
  async function toggleFileAssoc(verb: "epub-read" | "epub-edit" | "txt-make-epub", enabled: boolean) {
    try {
      await invoke("set_file_assoc", { verb, enabled });
    } catch (e: any) {
      console.error("设置文件关联失败:", e);
      await message(`设置失败: ${e}`, { title: "错误", kind: "error" });
      // 写注册表失败：把 toggle 状态回滚
      if (verb === "epub-read") libraryConfig.assocEpubRead = !enabled;
      else if (verb === "epub-edit") libraryConfig.assocEpubEdit = !enabled;
      else if (verb === "txt-make-epub") libraryConfig.assocTxtMakeEpub = !enabled;
      return;
    }
    await saveLibraryConfig();
  }

  onMount(async () => {
    loadShelfSettings();
    loadAppUiTheme();
    await bootLibrary();

    // 主窗口固定标题为 TEpub-Editor（避免被 editor/reader 子流程残留的 TXT/EPUB 标题污染）
    try {
      await getCurrentWindow().setTitle("TEpub-Editor");
    } catch (_) {}

    // 检查启动参数（文件关联，支持 --action= 路由）
    try {
      const launchInfo = await invoke<{ filePath: string | null; action: string | null }>("get_launch_info");
      const filePath = launchInfo?.filePath;
      if (filePath) {
        const ext = filePath.split(".").pop()?.toLowerCase();
        const isEpub = ext === "epub";
        const action = launchInfo?.action || "";
        const encoded = encodeURIComponent(filePath);

        let url: string;
        let title: string;
        let width = 1200;
        let height = 740;
        if (isEpub) {
          if (action === "reader") {
            url = `/reader?file=${encoded}`;
            title = "TEpub-Editor-Reader";
            width = 500;
            height = 800;
          } else {
            // 默认 / --action=epub-editor → 编辑器
            url = `/epub-editor?file=${encoded}`;
            title = "TEpub-Editor-EPUB";
          }
        } else {
          // .txt 默认或 --action=make-epub 都进 TXT 编辑器（编辑器内部有制作 EPUB 入口）
          url = `/editor?file=${encoded}&fromLibrary=1`;
          title = "TEpub-Editor-TXT";
          width = 1200;
          height = 740;
        }

        const appWindow = getCurrentWindow();
        const win = new WebviewWindow(`editor-${Date.now()}`, {
          url,
          title,
          width,
          height,
          dragDropEnabled: true,
          center: true,
        });

        const shouldHideMain = isEpub ? libraryConfig.closeLibraryOnEpubOpen !== false : libraryConfig.closeLibraryOnTxtOpen !== false;
        if (shouldHideMain) {
          await appWindow.hide();

          win.once("tauri://destroyed", async () => {
            await appWindow.show();
            await appWindow.setFocus();
          });
        }
      }
    } catch {}

    // 拖放处理 (Tauri)
    const appWindow = getCurrentWindow();
    appWindow.onDragDropEvent((ev: any) => {
      if (ev.payload?.type === "drop" && ev.payload?.paths) {
        (async () => {
          for (const p of ev.payload.paths) {
            try {
              await addBookWithCollisionRetry(p);
            } catch (e) {
              console.error("拖放添加失败:", e);
            }
          }
          await loadLibrary();
        })();
      }
    });
  });

  function backToLibrary() {
    // 从编辑器/元数据页返回书库
    window.location.href = "/library";
  }
</script>

<svelte:window
  on:dragover={(e) => e.preventDefault()}
  on:drop={handleDragDrop}
/>

<div class="library-app" data-context-type="library-root">
  <!-- 顶部工具栏 -->
  <div class="toolbar">
    <div class="toolbar-left">
      <button class="tb-btn primary" on:click={addBook}>+ 添加图书</button>
      <button class="tb-btn" on:click={openExternalFiles}>📂 打开文件</button>
      <div class="search-box">
        <input
          type="text"
          placeholder="搜索书名或作者..."
          bind:value={searchQuery}
        />
      </div>
    </div>
    <div class="toolbar-center"></div>
    <div class="toolbar-right">
      {#if ingestStatus}
        <span class="ingest-status">
          {ingestStatus === "decrypting" ? "解密入库中" : "入库中"}
        </span>
      {/if}
      <span class="book-count">{books.length} 本书</span>
      <button class="tb-btn" on:click={cycleViewMode} title="切换视图">{VIEW_ICONS[viewMode]}</button>
      <button class="tb-btn" on:click={() => showSettings = !showSettings} title="书库设置">⚙</button>
    </div>
  </div>

  <!-- 已激活的标签筛选条 —— 仅当至少有一个筛选时显示 -->
  {#if activeTagFilters.length > 0}
    <div class="filter-bar">
      <span class="filter-label">已筛选:</span>
      {#each activeTagFilters as tag}
        <button
          type="button"
          class="filter-chip"
          on:click={() => toggleTagFilter(tag)}
          title="点击移除该筛选"
        >
          <span class="filter-chip-text">{tag}</span>
          <span class="filter-chip-remove">×</span>
        </button>
      {/each}
      <button type="button" class="filter-clear" on:click={clearTagFilters}>清除全部</button>
      <span class="filter-count">{filteredBooks.length} / {books.length} 本</span>
    </div>
  {/if}

  <!-- 主体区域 -->
  <div class="main-body">
    <!-- svelte-ignore a11y-click-events-have-key-events -->
    <!-- svelte-ignore a11y-no-static-element-interactions -->
    <div class="browser-area" on:click={handleBrowserAreaClick}>
      {#if isLoading}
        <div class="empty-state">加载中...</div>
      {:else if filteredBooks.length === 0}
        <div class="empty-state">
          {#if books.length === 0}
            <div class="empty-icon">📚</div>
            <div class="empty-text">书库为空</div>
            <div class="empty-hint">点击「+ 添加图书」或拖放文件到此处</div>
          {:else}
            <div class="empty-text">无匹配结果</div>
          {/if}
        </div>
      {:else if viewMode === "grid"}
        <LibraryGrid
          books={filteredBooks}
          {coverCache}
          {selectedBook}
          showTitle={shelfSettings.gridShowTitle}
          showAuthor={shelfSettings.gridShowAuthor}
          on:select={(e) => selectedBook = e.detail}
          on:open={(e) => handleBookOpen(e.detail)}
          on:context={(e) => handleContextMenu(e.detail.event, e.detail.book)}
        />
      {:else if viewMode === "list-cover"}
        <LibraryListSimple
          books={filteredBooks}
          {selectedBook}
          {formatFileSize}
          columns={listColumns}
          {sortColumn}
          {sortAsc}
          onSort={handleSort}
          {activeTagFilters}
          onTagClick={toggleTagFilter}
          showCover={true}
          {coverCache}
          on:select={(e) => selectedBook = e.detail}
          on:open={(e) => handleBookOpen(e.detail)}
          on:context={(e) => handleContextMenu(e.detail.event, e.detail.book)}
          on:columnChange={(e) => listColumns = e.detail}
        />
      {:else}
        <LibraryListSimple
          books={filteredBooks}
          {selectedBook}
          {formatFileSize}
          columns={listColumns}
          {sortColumn}
          {sortAsc}
          onSort={handleSort}
          {activeTagFilters}
          onTagClick={toggleTagFilter}
          on:select={(e) => selectedBook = e.detail}
          on:open={(e) => handleBookOpen(e.detail)}
          on:context={(e) => handleContextMenu(e.detail.event, e.detail.book)}
          on:columnChange={(e) => listColumns = e.detail}
        />
      {/if}
    </div>

    <!-- 预览面板：仅在选中图书且未设置始终隐藏时显示 -->
    {#if previewVisible}
      <div class="preview-area">
        <LibraryPreview
          book={selectedBook}
          {coverCache}
          {formatFileSize}
          on:open={selectedBook ? () => handleBookOpen(selectedBook) : undefined}
          on:remove={selectedBook ? () => removeBook(selectedBook) : undefined}
        />
      </div>
    {/if}
  </div>

  <!-- 右键菜单 -->
  {#if showContextMenu && selectedBook}
    <!-- svelte-ignore a11y-click-events-have-key-events -->
    <!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
    <div
      class="context-overlay"
      on:click={closeContextMenu}
      on:contextmenu={(e) => { e.preventDefault(); closeContextMenu(); }}
    >
      <div class="context-menu" style="left: {contextMenuPos.x}px; top: {contextMenuPos.y}px;">
        <button class="ctx-item" on:click={() => { openReader(selectedBook!); closeContextMenu(); }}>阅读</button>
        <button class="ctx-item" on:click={() => { openBook(selectedBook!); closeContextMenu(); }}>编辑文件</button>
        <button class="ctx-item" on:click={() => { openEditMetadata(selectedBook!); closeContextMenu(); }}>编辑元数据</button>
        <div class="ctx-separator"></div>
        <button class="ctx-item" on:click={() => { openFileLocation(selectedBook!); closeContextMenu(); }}>打开文件位置</button>
        <button class="ctx-item" on:click={() => { openRenameDialog(selectedBook!); closeContextMenu(); }}>重命名本地文件</button>
        <div class="ctx-separator"></div>
        <button class="ctx-item danger" on:click={() => { removeBook(selectedBook!); closeContextMenu(); }}>从书库移除</button>
      </div>
    </div>
  {/if}

  <!-- 设置面板 -->
  {#if showSettings}
    <!-- svelte-ignore a11y-click-events-have-key-events -->
    <!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
    <div class="settings-overlay" on:click={(e) => { if (e.target === e.currentTarget) showSettings = false; }}>
      <div class="settings-panel library-settings-panel">
        <div class="settings-header">
          <h3>书库设置</h3>
          <button class="settings-close" on:click={() => showSettings = false} title="关闭">×</button>
        </div>

        <div class="settings-body">
          <!-- 文件存储 -->
          <section class="settings-section">
            <div class="section-title">文件存储</div>
            <div class="set-row">
              <label class="set-label">存储方式</label>
              <select class="set-control" bind:value={libraryConfig.storageMode} on:change={onStorageModeChange}>
                {#if appMode?.isPortable}
                  <option value="copy_portable">复制到 books/ 文件夹</option>
                {/if}
                <option value="index">仅索引文件位置（不复制）</option>
                <option value="copy_custom">复制到指定工作目录</option>
              </select>
            </div>
            {#if libraryConfig.storageMode === "copy_custom"}
              <div class="set-row">
                <label class="set-label">工作目录</label>
                <span class="custom-dir-display" title={libraryConfig.customWorkDir}>{libraryConfig.customWorkDir || "未选择"}</span>
                <button class="tb-btn" on:click={pickCustomWorkDir}>更改</button>
              </div>
            {/if}
            <label class="set-row toggle-row">
              <span class="set-label">保存时更新修改日期</span>
              <input type="checkbox" bind:checked={libraryConfig.updateModifiedOnEdit} on:change={saveLibraryConfig} />
            </label>
            <div class="set-row">
              <label class="set-label">入库命名方式</label>
              <select
                class="set-control"
                bind:value={libraryConfig.namingMode}
                on:change={onNamingChanged}
              >
                <option value="template">按模板重命名</option>
                <option value="source">使用源文件名</option>
              </select>
            </div>
            {#if libraryConfig.namingMode === "template"}
              <div class="set-row">
                <label class="set-label">命名模板</label>
                <select
                  class="set-control"
                  value={namingPresetSel}
                  on:change={(e) => onNamingPresetChange((e.currentTarget as HTMLSelectElement).value)}
                >
                  {#each NAMING_PRESETS as p}
                    <option value={p.value}>{p.label}</option>
                  {/each}
                  <option value="custom">自定义…</option>
                </select>
              </div>
              {#if namingPresetSel === "custom"}
                <div class="set-row">
                  <label class="set-label" style="visibility: hidden;">_</label>
                  <input
                    type="text"
                    class="set-control"
                    bind:value={libraryConfig.namingTemplate}
                    on:change={onNamingChanged}
                    placeholder="例如: {`{title}-{author}`}"
                  />
                </div>
              {/if}
              <p class="section-hint" style="margin-top: 4px;">
                可用占位符：<code>{`{title} {author} {subtitle} {series} {maker} {publisher} {tags}`}</code>。
                <code>{`{tags}`}</code> 自动展开为 <code>[标签1][标签2]…</code>。
              </p>
            {/if}
          </section>

          <!-- 文件关联 -->
          <section class="settings-section">
            <div class="section-title">注册文件打开方式</div>
            <p class="section-hint">右键 .epub / .txt 文件时显示的菜单项。</p>
            <label class="set-row toggle-row">
              <span class="set-label">EPUB 阅读 <small>(.epub)</small></span>
              <input
                type="checkbox"
                bind:checked={libraryConfig.assocEpubRead}
                on:change={(e) => toggleFileAssoc("epub-read", (e.currentTarget as HTMLInputElement).checked)}
              />
            </label>
            <label class="set-row toggle-row">
              <span class="set-label">EPUB 编辑 <small>(.epub)</small></span>
              <input
                type="checkbox"
                bind:checked={libraryConfig.assocEpubEdit}
                on:change={(e) => toggleFileAssoc("epub-edit", (e.currentTarget as HTMLInputElement).checked)}
              />
            </label>
            <label class="set-row toggle-row">
              <span class="set-label">制作 EPUB <small>(.txt)</small></span>
              <input
                type="checkbox"
                bind:checked={libraryConfig.assocTxtMakeEpub}
                on:change={(e) => toggleFileAssoc("txt-make-epub", (e.currentTarget as HTMLInputElement).checked)}
              />
            </label>
          </section>

          <!-- 书架显示 -->
          <section class="settings-section">
            <div class="section-title">编辑器窗口</div>
            <label class="set-row toggle-row">
              <span class="set-label">打开 TXT 编辑器时隐藏书库</span>
              <input
                type="checkbox"
                bind:checked={libraryConfig.closeLibraryOnTxtOpen}
                on:change={saveLibraryConfig}
              />
            </label>
            <label class="set-row toggle-row">
              <span class="set-label">打开 EPUB 编辑器时隐藏书库</span>
              <input
                type="checkbox"
                bind:checked={libraryConfig.closeLibraryOnEpubOpen}
                on:change={saveLibraryConfig}
              />
            </label>
            <div class="set-row">
              <label class="set-label">关闭 TXT 编辑器</label>
              <select
                class="set-control"
                bind:value={libraryConfig.txtEditorCloseAction}
                on:change={saveLibraryConfig}
              >
                <option value="library">返回书库</option>
                <option value="exit">直接退出应用</option>
              </select>
            </div>
          </section>

          <section class="settings-section">
            <div class="section-title">书架显示</div>
            <div class="set-row">
              <label class="set-label">界面主题</label>
              <select
                class="set-control"
                bind:value={appUiTheme}
                on:change={saveAppUiTheme}
              >
                <option value="modern">现代</option>
                <option value="classic">经典</option>
                <option value="dark">深色</option>
              </select>
            </div>
            <label class="set-row toggle-row">
              <span class="set-label">九宫格显示书名</span>
              <input type="checkbox" bind:checked={shelfSettings.gridShowTitle} on:change={saveShelfSettings} />
            </label>
            <label class="set-row toggle-row">
              <span class="set-label">九宫格显示作者</span>
              <input type="checkbox" bind:checked={shelfSettings.gridShowAuthor} on:change={saveShelfSettings} />
            </label>
            <div class="set-row">
              <label class="set-label">预览区显示</label>
              <select
                class="set-control"
                bind:value={shelfSettings.previewMode}
                on:change={saveShelfSettings}
              >
                <option value="always">始终显示</option>
                <option value="auto">点击显示</option>
                <option value="never">始终隐藏</option>
              </select>
            </div>
            <div class="set-row">
              <label class="set-label">双击图书时</label>
              <select
                class="set-control"
                bind:value={shelfSettings.dblClickAction}
                on:change={saveShelfSettings}
              >
                <option value="read">阅读</option>
                <option value="edit">编辑</option>
                <option value="metadata">编辑元数据</option>
              </select>
            </div>
            <div class="set-row">
              <label class="set-label">新书排序</label>
              <select
                class="set-control"
                value={shelfSettings.newBookFirst ? "first" : "last"}
                on:change={(e) => {
                  shelfSettings.newBookFirst = (e.target as HTMLSelectElement).value === "first";
                  saveShelfSettings();
                }}
              >
                <option value="first">新加入的在前</option>
                <option value="last">新加入的在后</option>
              </select>
            </div>
          </section>
        </div>

        <div class="settings-footer">
          <button class="tb-btn primary" on:click={() => showSettings = false}>完成</button>
        </div>
      </div>
    </div>
  {/if}

  <!-- 首次启动：选择书库存放目录 -->
  {#if firstLaunchOpen}
    <div class="settings-overlay first-launch-overlay">
      <div class="settings-panel first-launch-panel">
        <div class="settings-header">
          <h3>选择书库存放目录</h3>
        </div>
        <div class="settings-body">
          <p class="first-launch-hint">
            {#if appMode?.isPortable}
              检测到 <strong>绿色版</strong>。书库默认存放在程序同级目录的 <code>EPUB</code> 文件夹下，可改成任意路径。
            {:else}
              检测到 <strong>安装版</strong>。请选择一个目录用来存放书库（library.json、封面缓存、复制进来的图书副本都会放在这里）。
            {/if}
          </p>
          <div class="set-row">
            <label class="set-label">书库目录</label>
            <input
              class="set-control"
              type="text"
              bind:value={firstLaunchInput}
              placeholder={appMode?.isPortable ? appMode.suggestedLibraryDir : "点击右侧选择目录..."}
              disabled={firstLaunchSaving}
            />
            <button class="tb-btn" on:click={browseFirstLaunchDir} disabled={firstLaunchSaving}>浏览...</button>
          </div>
          <p class="first-launch-tip">
            后续可在「设置」里调整。如果想让安装版变绿色版（数据全部跟随程序），在程序所在目录创建一个 <code>portable.txt</code> 空文件即可。
          </p>
        </div>
        <div class="meta-edit-actions">
          <button class="tb-btn primary" on:click={confirmFirstLaunchDir} disabled={firstLaunchSaving}>
            {#if firstLaunchSaving}
              <span class="saving-spinner" aria-hidden="true"></span>
              保存中...
            {:else}
              确定
            {/if}
          </button>
        </div>
      </div>
    </div>
  {/if}

  <!-- 入库时书名-作者重名弹框 -->
  {#if collisionPrompt.open}
    <!-- svelte-ignore a11y-click-events-have-key-events -->
    <!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
    <div class="settings-overlay" on:click={(e) => { if (e.target === e.currentTarget) cancelCollisionPrompt(); }}>
      <div class="settings-panel" style="max-width: 480px;">
        <div class="settings-header">
          <h3>同名文件已存在</h3>
          <button class="settings-close" on:click={cancelCollisionPrompt} title="取消">×</button>
        </div>
        <div class="settings-body">
          <p style="margin: 0 0 8px; color: var(--color-text-soft);">
            工作目录中已存在 <code>{collisionPrompt.suggested}</code>，请输入新的文件名：
          </p>
          <input
            type="text"
            bind:value={collisionPrompt.inputValue}
            style="width: 100%; padding: 8px; border: 1px solid var(--color-border); border-radius: 4px; box-sizing: border-box;"
            on:keydown={(e) => {
              if (e.key === "Enter") confirmCollisionPrompt();
              if (e.key === "Escape") cancelCollisionPrompt();
            }}
          />
          <p style="margin: 8px 0 0; color: var(--color-muted); font-size: 12px;">
            可省略扩展名，会自动按文件类型补回。
          </p>
        </div>
        <div class="settings-footer" style="display: flex; gap: 8px; justify-content: flex-end; padding: 12px 16px;">
          <button class="tb-btn" on:click={cancelCollisionPrompt}>取消</button>
          <button class="tb-btn primary" on:click={confirmCollisionPrompt}>保存</button>
        </div>
      </div>
    </div>
  {/if}

  <!-- 重命名本地文件弹框 -->
  {#if renamePrompt.open}
    <!-- svelte-ignore a11y-click-events-have-key-events -->
    <!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
    <div class="settings-overlay" on:click={(e) => { if (e.target === e.currentTarget) cancelRenameBook(); }}>
      <div class="settings-panel" style="max-width: 480px;">
        <div class="settings-header">
          <h3>重命名本地文件</h3>
          <button class="settings-close" on:click={cancelRenameBook} title="取消">×</button>
        </div>
        <div class="settings-body">
          <p style="margin: 0 0 8px; color: var(--color-text-soft);">
            当前: <code>{renamePrompt.oldFilename}</code>
          </p>
          <input
            type="text"
            bind:value={renamePrompt.inputValue}
            style="width: 100%; padding: 8px; border: 1px solid var(--color-border); border-radius: 4px; box-sizing: border-box;"
            on:keydown={(e) => {
              if (e.key === "Enter") confirmRenameBook();
              if (e.key === "Escape") cancelRenameBook();
            }}
          />
          <p style="margin: 8px 0 0; color: var(--color-muted); font-size: 12px;">
            可省略扩展名，会自动补 .{renamePrompt.fileType}。
          </p>
          {#if renamePrompt.error}
            <p style="margin: 8px 0 0; color: #c62828; font-size: 13px;">{renamePrompt.error}</p>
          {/if}
        </div>
        <div class="settings-footer" style="display: flex; gap: 8px; justify-content: flex-end; padding: 12px 16px;">
          <button class="tb-btn" on:click={cancelRenameBook}>取消</button>
          <button class="tb-btn primary" on:click={confirmRenameBook}>保存</button>
        </div>
      </div>
    </div>
  {/if}

  <!-- 元数据编辑面板 -->
  {#if showMetaEditor && selectedBook}
    <!-- svelte-ignore a11y-click-events-have-key-events -->
    <!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
    <div class="settings-overlay" on:click={(e) => { if (e.target === e.currentTarget && !savingMetadata) showMetaEditor = false; }}>
      <div class="settings-panel meta-editor-panel">
        <div class="settings-header">
          <h3>编辑元数据 <span class="meta-title-hint">— {selectedBook.title}</span></h3>
          <button class="settings-close" on:click={() => showMetaEditor = false} title="关闭" disabled={savingMetadata}>×</button>
        </div>

        <div class="meta-edit-body">
          <div class="meta-edit-main">
            <div class="meta-fields">
              <!-- 基本信息 -->
              <div class="meta-section">
                <div class="meta-section-head"><span>基本</span></div>
                <div class="set-row">
                  <label>书名</label>
                  <input type="text" bind:value={metaForm.title} />
                </div>

                {#if subtitleShown}
                  <div class="set-row">
                    <label>副标题</label>
                    <input
                      type="text"
                      bind:value={metaForm.subtitle}
                      placeholder="可选"
                      autofocus
                    />
                    <button
                      class="meta-inline-btn"
                      title="清除副标题"
                      on:click={() => { metaForm.subtitle = ""; subtitleShown = false; }}
                    >×</button>
                  </div>
                {:else}
                  <div class="set-row meta-add-row">
                    <label></label>
                    <button class="meta-add-link" on:click={() => subtitleShown = true}>+ 添加副标题</button>
                  </div>
                {/if}

                <div class="set-row">
                  <label>作者</label>
                  <input type="text" bind:value={metaForm.author} />
                </div>
                <div class="set-row tags-row">
                  <label>标签</label>
                  <!-- svelte-ignore a11y-no-static-element-interactions -->
                  <div
                    class="tags-editor"
                    class:open={tagPanelOpen}
                    on:focusout={onTagsEditorFocusOut}
                  >
                    <div class="tags-chip-row">
                      {#each orderedTagsForDisplay as tag (tag)}
                        <span
                          class="tag-chip"
                          class:tag-chip-tier={TAXONOMY_SET.has(tag)}
                        >
                          <span class="tag-chip-text">{tag}</span>
                          <button
                            type="button"
                            class="tag-chip-remove"
                            on:click={() => removeTagByName(tag)}
                            title="移除标签"
                          >×</button>
                        </span>
                      {/each}
                      <input
                        class="tag-input"
                        type="text"
                        bind:value={tagInput}
                        placeholder={metaForm.tags.length === 0 ? "点击此处选择/添加标签…" : "+ 标签"}
                        on:focus={() => tagPanelOpen = true}
                        on:click={() => tagPanelOpen = true}
                        on:keydown={onTagKeydown}
                      />
                    </div>

                    {#if tagPanelOpen}
                      <div class="tag-panel">
                        {#if tagPanelPhase === "tiers"}
                          <!-- Phase 1：前三级单选 -->
                          <!-- 第一级（单选） -->
                          <div class="tier-row">
                            <span class="tier-label">类型</span>
                            <div class="tier-options">
                              {#each TAG_L1 as opt}
                                <button
                                  type="button"
                                  class="tier-opt"
                                  class:active={currentL1 === opt}
                                  on:click={() => selectL1(opt)}
                                >{opt}</button>
                              {/each}
                            </div>
                          </div>

                          <!-- 第二级（依赖第一级，单选） -->
                          {#if currentL1 && currentL2List.length > 0}
                            <div class="tier-row">
                              <span class="tier-label">分类</span>
                              <div class="tier-options">
                                {#each currentL2List as opt}
                                  <button
                                    type="button"
                                    class="tier-opt"
                                    class:active={currentL2 === opt}
                                    on:click={() => selectL2(opt)}
                                  >{opt}</button>
                                {/each}
                              </div>
                            </div>
                          {/if}

                          <!-- 第三级（仅男频，单选） -->
                          {#if currentL3List.length > 0}
                            <div class="tier-row">
                              <span class="tier-label">主角</span>
                              <div class="tier-options">
                                {#each currentL3List as opt}
                                  <button
                                    type="button"
                                    class="tier-opt"
                                    class:active={currentL3 === opt}
                                    on:click={() => selectL3(opt)}
                                  >{opt}</button>
                                {/each}
                              </div>
                            </div>
                          {/if}

                          <div class="tag-panel-hint">
                            选完前三级后,可在此添加自定义标签
                          </div>
                        {:else}
                          <!-- Phase 2：自定义标签建议（按输入筛选） -->
                          {#if filteredSuggestions.length > 0}
                            <div class="tier-row">
                              <span class="tier-label">建议</span>
                              <div class="tier-options tier-suggestions">
                                {#each filteredSuggestions.slice(0, 60) as t}
                                  <button
                                    type="button"
                                    class="tier-opt suggest"
                                    on:click={() => addSuggestion(t)}
                                  >+ {t}</button>
                                {/each}
                                {#if filteredSuggestions.length > 60}
                                  <span class="tier-more">…还有 {filteredSuggestions.length - 60}</span>
                                {/if}
                              </div>
                            </div>
                          {:else if tagInput.trim()}
                            <div class="tag-panel-empty">
                              库内没有匹配 "<b>{tagInput.trim()}</b>" 的标签 — 回车将其作为新标签添加
                            </div>
                          {:else}
                            <div class="tag-panel-empty">
                              输入文字搜索已用过的标签,或按回车直接添加新标签
                            </div>
                          {/if}

                          <div class="tag-panel-hint">
                            回车 / 逗号 添加;退格 删除最后一个;Esc 关闭
                          </div>
                        {/if}
                      </div>
                    {/if}
                  </div>
                </div>
                <div class="set-row meta-textarea-row">
                  <label>简介</label>
                  <textarea
                    class="meta-description"
                    bind:value={metaForm.description}
                    placeholder="本书简介..."
                    rows="4"
                  ></textarea>
                </div>
                <div class="set-row">
                  <label>文件名</label>
                  <input
                    class="meta-readonly"
                    type="text"
                    value={selectedBook.filename || "-"}
                    readonly
                    title={selectedBook.filename}
                  />
                </div>
                {#if selectedBook.fileType === "epub"}
                  <div class="set-row">
                    <label>UUID</label>
                    <input
                      class="meta-uuid-inline"
                      type="text"
                      bind:value={metaForm.epubUuid}
                      placeholder="urn:uuid:... 或留空"
                      title={metaForm.epubUuid || "可手动编辑或点击 ↻ 自动提取"}
                    />
                    <button
                      class="meta-icon-btn"
                      class:copied={copyFeedbackKey === "uuid"}
                      on:click={() => copyText(metaForm.epubUuid, "uuid")}
                      disabled={!metaForm.epubUuid}
                      title={metaForm.epubUuid ? "复制 UUID" : "无 UUID 可复制"}
                    >
                      {copyFeedbackKey === "uuid" ? "✓" : "⎘"}
                    </button>
                    <button
                      class="meta-icon-btn"
                      on:click={refreshMetadata}
                      disabled={refreshing}
                      title="从 EPUB 文件重新读取元数据"
                    >
                      {refreshing ? "…" : "↻"}
                    </button>
                  </div>
                {/if}
              </div>

              <!-- 制作信息（默认折叠，置于最底；标题在左、箭头在右） -->
              <div class="meta-section meta-collapsible" class:expanded={productionExpanded}>
                <button
                  class="meta-section-head meta-section-toggle"
                  on:click={() => productionExpanded = !productionExpanded}
                  aria-expanded={productionExpanded}
                >
                  <span>制作信息</span>
                  {#if !productionExpanded && (metaForm.maker || metaForm.series)}
                    <span class="meta-section-hint">已填写</span>
                  {/if}
                  <span class="meta-collapse-arrow">{productionExpanded ? "▾" : "▸"}</span>
                </button>
                {#if productionExpanded}
                  <div class="meta-section-body">
                    <div class="set-row">
                      <label>制作者</label>
                      <input type="text" bind:value={metaForm.maker} placeholder="可选" />
                    </div>
                    <div class="set-row">
                      <label>制作系列</label>
                      <input type="text" bind:value={metaForm.series} placeholder="可选" />
                    </div>
                    <div class="set-row">
                      <label>制作时间</label>
                      <input
                        class="meta-readonly"
                        type="text"
                        value={formatDateTime(selectedBook.createdAt)}
                        readonly
                      />
                    </div>
                    <div class="set-row">
                      <label>修改时间</label>
                      <input
                        class="meta-readonly"
                        type="text"
                        value={formatDateTime(selectedBook.modifiedAt)}
                        readonly
                      />
                    </div>
                  </div>
                {/if}
              </div>
            </div>

            <div class="meta-cover-side">
              {#if coverCache.get(selectedBook.id)}
                <img class="meta-cover-preview" src={coverCache.get(selectedBook.id)} alt="" />
              {:else}
                <div class="meta-cover-placeholder">{selectedBook.title[0]}</div>
              {/if}
              <button class="tb-btn" on:click={changeCover}>更换封面</button>
            </div>
          </div>
        </div>

        <div class="meta-edit-actions">
          <button class="tb-btn" on:click={() => { showMetaEditor = false; }} disabled={savingMetadata}>取消</button>
          <button class="tb-btn primary" on:click={saveMetadata} disabled={savingMetadata}>
            {#if savingMetadata}
              <span class="saving-spinner" aria-hidden="true"></span>
              保存中...
            {:else}
              保存
            {/if}
          </button>
        </div>
      </div>
      {#if savingMetadata}
        <div class="saving-overlay" aria-live="polite">
          <div class="saving-box">
            <span class="saving-spinner big" aria-hidden="true"></span>
            <span>正在写入 EPUB，请稍候…</span>
          </div>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .library-app {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--color-canvas);
    color: var(--color-text);
    font-family: var(--font-ui);
    user-select: none;
  }

  /* 工具栏 */
  .toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 16px;
    background: var(--color-canvas);
    border-bottom: 1px solid var(--color-border);
    gap: 12px;
    flex-shrink: 0;
  }

  .toolbar-left,
  .toolbar-center,
  .toolbar-right {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .tb-btn {
    padding: 6px 12px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
    color: var(--color-text-soft);
    cursor: pointer;
    font-size: 13px;
    transition: background var(--transition-fast);
  }

  .tb-btn:hover {
    background: var(--color-hover);
    color: var(--color-text);
  }

  .tb-btn.active {
    background: var(--color-accent-soft);
    color: var(--color-accent-deep);
    border-color: var(--color-accent);
  }

  .tb-btn.primary {
    background: var(--gradient-accent);
    color: #fff;
    border: none;
    font-weight: 700;
    padding: 7px 16px;
  }

  .tb-btn.primary:hover {
    opacity: 0.9;
  }

  .search-box input {
    width: 200px;
    padding: 6px 10px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
    color: var(--color-text);
    font-size: 13px;
  }

  .search-box input:focus {
    border-color: var(--color-accent);
    box-shadow: var(--focus-ring);
    outline: none;
  }

  @media (max-width: 760px) {
    .library-settings-panel {
      min-width: 0;
      width: min(96vw, 520px);
    }

    .library-settings-panel .settings-body {
      grid-template-columns: 1fr;
    }
  }

  .view-toggles {
    display: flex;
    gap: 2px;
  }

  .view-toggles .tb-btn {
    padding: 6px 10px;
    font-size: 16px;
  }


  .book-count {
    color: var(--color-muted);
    font-size: 12px;
  }

  .ingest-status {
    display: inline-flex;
    align-items: center;
    height: 28px;
    padding: 0 10px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--color-accent) 14%, transparent);
    color: var(--color-accent);
    font-size: 12px;
    font-weight: 700;
    white-space: nowrap;
  }

  /* 已激活筛选条 —— 介于工具栏和主体之间 */
  .filter-bar {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
    padding: 6px 16px;
    background: var(--color-surface-soft);
    border-bottom: 1px solid var(--color-border);
    flex-shrink: 0;
  }

  .filter-label {
    font-size: 11px;
    font-weight: 700;
    color: var(--color-muted);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-right: 2px;
  }

  .filter-chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 4px 2px 10px;
    background: var(--color-accent);
    color: #fff;
    border: none;
    border-radius: 999px;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    transition: opacity var(--transition-fast);
  }

  .filter-chip:hover { opacity: 0.85; }

  .filter-chip-remove {
    width: 18px;
    height: 18px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
    font-size: 14px;
    line-height: 1;
    background: rgba(255, 255, 255, 0.2);
  }

  .filter-clear {
    padding: 2px 10px;
    border: 1px dashed var(--color-border);
    background: transparent;
    color: var(--color-muted);
    border-radius: var(--radius-xs);
    font-size: 11px;
    cursor: pointer;
    transition: background var(--transition-fast), color var(--transition-fast);
  }

  .filter-clear:hover {
    background: var(--color-hover);
    color: var(--color-text);
  }

  .filter-count {
    margin-left: auto;
    font-size: 11px;
    color: var(--color-muted);
  }

  /* 主体 */
  .main-body {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  .browser-area {
    flex: 1;
    overflow-y: auto;
    padding: 8px;
  }

  .preview-area {
    width: 280px;
    flex-shrink: 0;
    border-left: 1px solid var(--color-border);
    background: var(--color-surface-soft);
    overflow-y: auto;
  }

  /* 空状态 */
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--color-muted);
    gap: 8px;
  }

  .empty-icon {
    font-size: 64px;
    opacity: 0.4;
  }

  .empty-text {
    font-size: 18px;
    font-weight: 700;
  }

  .empty-hint {
    font-size: 13px;
    color: var(--color-muted);
  }

  /* 右键菜单 */
  .context-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 1000;
    background: transparent;
  }

  .context-menu {
    position: fixed;
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-pop);
    padding: 4px;
    min-width: 160px;
    z-index: 1001;
  }

  .ctx-item {
    display: block;
    width: 100%;
    padding: 8px 16px;
    border: none;
    background: none;
    color: var(--color-text);
    font-size: 13px;
    text-align: left;
    cursor: pointer;
    border-radius: var(--radius-xs);
  }

  .ctx-item:hover {
    background: var(--color-accent-soft);
    color: var(--color-accent-deep);
  }

  .ctx-item.danger {
    color: var(--color-danger);
  }

  .ctx-item.danger:hover {
    background: var(--color-danger-soft);
  }

  .ctx-separator {
    height: 1px;
    background: var(--color-border);
    margin: 4px 0;
  }

  /* 设置面板 */
  .settings-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.35);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1002;
    backdrop-filter: blur(6px);
    animation: fadeIn 0.18s ease-out;
  }

  @keyframes fadeIn {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  .settings-panel {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    min-width: 460px;
    max-width: 520px;
    max-height: 80vh;
    display: flex;
    flex-direction: column;
    box-shadow: var(--shadow-pop);
    overflow: hidden;
    animation: panelIn 0.2s ease-out;
  }

  .library-settings-panel {
    border-radius: 14px;
    min-width: 720px;
    max-width: 820px;
    max-height: 84vh;
  }

  @keyframes panelIn {
    from { opacity: 0; transform: translateY(8px) scale(0.98); }
    to { opacity: 1; transform: translateY(0) scale(1); }
  }

  .settings-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 22px;
    border-bottom: 1px solid var(--color-border);
    background: linear-gradient(180deg, rgba(255, 255, 255, 0.96), rgba(248, 250, 252, 0.96));
  }

  .settings-panel h3 {
    margin: 0;
    color: var(--color-text);
    font-size: 15px;
    font-weight: 700;
    letter-spacing: 0.3px;
  }

  .settings-close {
    width: 26px;
    height: 26px;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--color-muted);
    font-size: 20px;
    line-height: 1;
    cursor: pointer;
    transition: background var(--transition-fast), color var(--transition-fast);
  }

  .settings-close:hover {
    background: var(--color-hover);
    color: var(--color-text);
  }

  .settings-body {
    flex: 1;
    overflow-y: auto;
    padding: 16px 20px;
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .library-settings-panel .settings-body {
    padding: 18px 20px;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 14px;
    background: var(--color-canvas);
  }

  .settings-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .library-settings-panel .settings-section {
    gap: 10px;
    min-width: 0;
    padding: 14px;
    border: 1px solid var(--color-border);
    border-radius: 10px;
    background: rgba(255, 255, 255, 0.82);
  }

  .library-settings-panel .settings-section:first-child {
    grid-column: 1 / -1;
  }

  .section-title {
    font-size: 11px;
    font-weight: 700;
    color: var(--color-muted);
    text-transform: uppercase;
    letter-spacing: 0.6px;
    padding-bottom: 4px;
    border-bottom: 1px solid var(--color-border);
    margin-bottom: 4px;
  }

  .settings-footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px 20px;
    border-top: 1px solid var(--color-border);
    background: var(--color-canvas);
  }

  .set-row {
    display: flex;
    align-items: center;
    gap: 12px;
    margin: 0;
  }

  .set-label {
    flex-shrink: 0;
    color: var(--color-text-soft);
    font-size: 13px;
    min-width: 120px;
  }

  .set-control {
    flex: 1;
    padding: 6px 10px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
    color: var(--color-text);
    font-size: 13px;
    transition: border-color var(--transition-fast), box-shadow var(--transition-fast);
  }

  .set-control:focus {
    border-color: var(--color-accent);
    box-shadow: var(--focus-ring);
    outline: none;
  }

  /* 复选框行：标签占据空间，复选框靠右 */
  .toggle-row {
    cursor: pointer;
    padding: 6px 8px;
    margin: 0 -8px;
    border-radius: var(--radius-sm);
    transition: background var(--transition-fast);
  }

  .toggle-row:hover {
    background: var(--color-hover);
  }

  .toggle-row .set-label {
    flex: 1;
    min-width: 0;
    color: var(--color-text);
  }

  .toggle-row input[type="checkbox"] {
    flex-shrink: 0;
    width: 16px;
    height: 16px;
    accent-color: var(--color-accent);
    cursor: pointer;
  }

  /* 兼容旧式 .set-row label / input / select / textarea（元数据编辑面板仍在使用） */
  .set-row label {
    flex-shrink: 0;
    color: var(--color-text-soft);
    font-size: 13px;
  }

  .set-row select,
  .set-row input,
  .set-row textarea {
    flex: 1;
    padding: 6px 10px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
    color: var(--color-text);
  }

  .set-row textarea {
    resize: vertical;
    font-family: var(--font-ui);
  }

  @media (max-width: 760px) {
    .library-settings-panel {
      min-width: 0;
      width: min(96vw, 520px);
    }

    .library-settings-panel .settings-body {
      grid-template-columns: 1fr;
    }
  }

  /* 元数据编辑面板：复用 settings-panel 头/体/底三段式 */
  .meta-editor-panel {
    min-width: 640px;
    max-width: 720px;
  }

  .meta-title-hint {
    color: var(--color-muted);
    font-weight: 500;
    font-size: 13px;
    margin-left: 4px;
    /* 长书名截断，避免标题栏过宽 */
    display: inline-block;
    max-width: 360px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    vertical-align: bottom;
  }

  .meta-edit-body {
    flex: 1;
    overflow-y: auto;
    padding: 16px 20px;
  }

  .meta-edit-main {
    display: flex;
    gap: 24px;
    align-items: flex-start;
  }

  .meta-fields {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  /* 元数据分组 */
  .meta-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .meta-section-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    font-size: 11px;
    font-weight: 700;
    color: var(--color-muted);
    text-transform: uppercase;
    letter-spacing: 0.6px;
    padding-bottom: 4px;
    border-bottom: 1px dashed var(--color-border);
  }

  /* 可折叠分组（如"制作信息"）：标题在左，箭头在右 */
  .meta-collapsible .meta-section-toggle {
    width: 100%;
    border: none;
    background: transparent;
    cursor: pointer;
    text-align: left;
    color: var(--color-muted);
    font: inherit;
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.6px;
    padding: 4px 0;
    border-bottom: 1px dashed var(--color-border);
    display: flex;
    align-items: center;
    gap: 8px;
    transition: color var(--transition-fast);
  }

  .meta-collapsible .meta-section-toggle:hover {
    color: var(--color-text);
  }

  /* 箭头被推到行尾 */
  .meta-collapse-arrow {
    margin-left: auto;
    display: inline-block;
    width: 12px;
    color: var(--color-muted);
    flex-shrink: 0;
    text-align: right;
  }

  /* "已填写"小提示在标题后、箭头前 */
  .meta-section-hint {
    font-size: 10px;
    font-weight: 500;
    color: var(--color-accent);
    text-transform: none;
    letter-spacing: 0;
    padding: 1px 6px;
    background: var(--color-accent-soft);
    border-radius: var(--radius-xs);
  }

  .meta-section-body {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-top: 8px;
  }

  /* 副标题等"添加字段"按钮 —— 替代空 input */
  .meta-add-row .meta-add-link {
    border: 1px dashed var(--color-border);
    background: transparent;
    color: var(--color-muted);
    border-radius: var(--radius-sm);
    padding: 5px 12px;
    font-size: 12px;
    cursor: pointer;
    flex: 1;
    text-align: left;
    transition: background var(--transition-fast), color var(--transition-fast), border-color var(--transition-fast);
  }

  .meta-add-row .meta-add-link:hover {
    background: var(--color-hover);
    color: var(--color-accent);
    border-color: var(--color-accent);
    border-style: solid;
  }

  /* set-row 中的小型行内动作按钮（如清除字段） */
  .meta-inline-btn {
    flex-shrink: 0;
    width: 26px;
    height: 26px;
    border: 1px solid var(--color-border);
    background: var(--color-surface);
    color: var(--color-muted);
    border-radius: var(--radius-xs);
    cursor: pointer;
    font-size: 14px;
    line-height: 1;
    transition: background var(--transition-fast), color var(--transition-fast), border-color var(--transition-fast);
  }

  .meta-inline-btn:hover {
    background: var(--color-danger-soft, var(--color-hover));
    color: var(--color-danger, var(--color-text));
    border-color: var(--color-danger, var(--color-text-soft));
  }

  /* 只读字段（如文件名、制作时间、修改时间） */
  .meta-readonly {
    background: var(--color-surface-soft) !important;
    color: var(--color-text-soft) !important;
    cursor: default;
    font-family: var(--font-mono);
    font-size: 12px !important;
  }

  .meta-readonly:focus {
    box-shadow: none !important;
  }

  /* UUID 行内显示 —— 用 mono 字体，字号小一点便于完整展示 */
  .meta-uuid-inline {
    font-family: var(--font-mono) !important;
    font-size: 11px !important;
    letter-spacing: -0.2px;
  }

  /* 行内小图标按钮（复制 / 重新提取） */
  .meta-icon-btn {
    flex-shrink: 0;
    width: 28px;
    height: 28px;
    border: 1px solid var(--color-border);
    background: var(--color-surface);
    color: var(--color-muted);
    border-radius: var(--radius-xs);
    cursor: pointer;
    font-size: 13px;
    line-height: 1;
    transition: background var(--transition-fast), color var(--transition-fast), border-color var(--transition-fast);
  }

  .meta-icon-btn:hover:not(:disabled) {
    background: var(--color-hover);
    color: var(--color-text);
    border-color: var(--color-text-soft);
  }

  .meta-icon-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .meta-icon-btn.copied {
    background: var(--color-accent-soft);
    color: var(--color-accent-deep);
    border-color: var(--color-accent);
  }

  /* 标签编辑器 ：chips + 行内输入 + 选择面板 */
  .tags-row {
    align-items: flex-start !important;
  }

  .tags-row > label {
    /* 让 label 与 chips 第一行水平居中 */
    margin-top: 6px;
  }

  .tags-editor {
    flex: 1;
    min-width: 0;
    /* 容器需 position: relative 给绝对定位的弹层定位 */
    position: relative;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
    transition: border-color var(--transition-fast), box-shadow var(--transition-fast);
  }

  .tags-editor:focus-within,
  .tags-editor.open {
    border-color: var(--color-accent);
    box-shadow: var(--focus-ring);
  }

  /* chips + 输入框这一行（横向）—— 编辑器主体高度由它决定 */
  .tags-chip-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    padding: 4px 6px;
  }

  /* 标签胶囊 —— 编辑态 */
  .tag-chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 4px 2px 10px;
    border-radius: 999px;
    background: var(--color-accent-soft);
    color: var(--color-accent-deep);
    font-size: 12px;
    font-weight: 600;
    line-height: 1.4;
    max-width: 100%;
  }

  /* 三级分类 chip：稍带边框区分于自由标签 */
  .tag-chip.tag-chip-tier {
    background: var(--color-accent);
    color: #fff;
  }

  .tag-chip-text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tag-chip-remove {
    flex-shrink: 0;
    width: 18px;
    height: 18px;
    border: none;
    background: transparent;
    color: inherit;
    border-radius: 50%;
    font-size: 14px;
    line-height: 1;
    cursor: pointer;
    opacity: 0.6;
    transition: background var(--transition-fast), opacity var(--transition-fast);
  }

  .tag-chip-remove:hover {
    background: rgba(0, 0, 0, 0.15);
    opacity: 1;
  }

  /* 内联"+ 标签"输入框 —— 没有可见边框，与 chips 同处一容器 */
  .tag-input {
    flex: 1;
    min-width: 80px;
    border: none !important;
    outline: none;
    background: transparent !important;
    color: var(--color-text);
    font-size: 12px !important;
    padding: 4px 6px !important;
  }

  .tag-input:focus {
    box-shadow: none !important;
  }

  /* 选择面板 —— 绝对定位悬浮在编辑器下方，类似浏览器搜索建议下拉 */
  .tag-panel {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    right: 0;
    z-index: 100;
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    background: var(--color-surface);
    border: 1px solid var(--color-accent);
    border-radius: var(--radius-sm);
    box-shadow: var(--shadow-pop);
    /* 限制最大高度，超过滚动；不会撑爆元数据弹窗，也不挤占文本流 */
    max-height: 280px;
    overflow-y: auto;
    /* 弹层进入动画：从 -4px 滑入 + 透明 → 不透明 */
    animation: tagPanelIn 0.12s ease-out;
  }

  @keyframes tagPanelIn {
    from { opacity: 0; transform: translateY(-4px); }
    to   { opacity: 1; transform: translateY(0); }
  }

  .tier-row {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    flex-wrap: nowrap;
  }

  .tier-label {
    flex-shrink: 0;
    width: 32px;
    margin-top: 4px;
    font-size: 10px;
    font-weight: 700;
    color: var(--color-muted);
    text-transform: uppercase;
    letter-spacing: 0.4px;
    text-align: right;
  }

  .tier-options {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .tier-options.tier-suggestions {
    /* 建议列表可能很长，控制最大高度 */
    max-height: 96px;
    overflow-y: auto;
  }

  /* tier 单选按钮 */
  .tier-opt {
    border: 1px solid var(--color-border);
    background: var(--color-surface);
    color: var(--color-text-soft);
    padding: 3px 10px;
    border-radius: 999px;
    font-size: 11px;
    cursor: pointer;
    line-height: 1.6;
    transition: background var(--transition-fast), color var(--transition-fast), border-color var(--transition-fast);
  }

  .tier-opt:hover {
    background: var(--color-hover);
    color: var(--color-text);
    border-color: var(--color-text-soft);
  }

  .tier-opt.active {
    background: var(--color-accent);
    color: #fff;
    border-color: var(--color-accent);
  }

  /* 自定义建议按钮：与 tier-opt 同形但用淡背景 */
  .tier-opt.suggest {
    color: var(--color-text-soft);
    border-style: dashed;
  }

  .tier-opt.suggest:hover {
    background: var(--color-accent-soft);
    color: var(--color-accent-deep);
    border-color: var(--color-accent);
    border-style: solid;
  }

  .tier-more {
    align-self: center;
    font-size: 10px;
    color: var(--color-muted);
    padding: 0 6px;
  }

  .tag-panel-hint {
    font-size: 10px;
    color: var(--color-muted);
    border-top: 1px dashed var(--color-border);
    padding-top: 6px;
    text-align: center;
  }

  /* Phase 2 没有匹配建议时的提示 */
  .tag-panel-empty {
    padding: 12px 4px;
    font-size: 11px;
    color: var(--color-muted);
    text-align: center;
    line-height: 1.5;
  }

  .tag-panel-empty b {
    color: var(--color-text-soft);
  }

  .meta-edit-body .set-row {
    margin: 0;
  }

  .meta-fields .set-row {
    align-items: center;
  }

  .meta-fields .set-row label {
    width: 64px;
    flex-shrink: 0;
    text-align: right;
    color: var(--color-text-soft);
    font-size: 13px;
  }

  /* 简介 textarea：不带 label，全宽 */
  .meta-description {
    width: 100%;
    box-sizing: border-box;
    padding: 8px 10px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
    color: var(--color-text);
    font-family: var(--font-ui);
    font-size: 13px;
    resize: vertical;
    transition: border-color var(--transition-fast), box-shadow var(--transition-fast);
  }

  .meta-description:focus {
    border-color: var(--color-accent);
    box-shadow: var(--focus-ring);
    outline: none;
  }

  /* 简介行：textarea 跟随 set-row 节奏(空 label + flex:1 textarea)，
     使其右边沿与上方输入对齐；textarea 应顶端起始而不是 center */
  .meta-textarea-row {
    align-items: flex-start !important;
  }

  /* 简介 label 顶部下移一点，与 textarea 第一行水平对齐 */
  .meta-textarea-row > label {
    margin-top: 6px;
  }

  /* 给 set-row 中的所有可编辑控件强制 border-box，否则
     content-box + flex:1 + padding 会让右侧轻微越界，与设了
     box-sizing: border-box 的 textarea 不同宽 */
  .meta-edit-body .set-row input,
  .meta-edit-body .set-row select,
  .meta-edit-body .set-row textarea {
    box-sizing: border-box;
  }

  .meta-cover-side {
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
  }

  .meta-cover-preview {
    width: 120px;
    height: 160px;
    object-fit: cover;
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-border);
    box-shadow: var(--shadow-xs);
  }

  .meta-cover-placeholder {
    width: 120px;
    height: 160px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--color-surface-soft);
    border-radius: var(--radius-sm);
    font-size: 48px;
    font-weight: 700;
    color: var(--color-muted);
  }

  .meta-edit-actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
    padding: 12px 20px;
    border-top: 1px solid var(--color-border);
    background: var(--color-canvas);
  }

  /* 保存中蒙层 + 转圈 */
  .saving-overlay {
    position: absolute;
    inset: 0;
    background: rgba(0, 0, 0, 0.35);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 10;
    border-radius: var(--radius-md);
  }
  .saving-box {
    background: var(--color-surface);
    color: var(--color-text);
    padding: 16px 22px;
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-pop);
    display: inline-flex;
    align-items: center;
    gap: 12px;
    font-size: 14px;
  }
  .saving-spinner {
    display: inline-block;
    width: 12px;
    height: 12px;
    border: 2px solid var(--color-border);
    border-top-color: var(--color-accent);
    border-radius: 50%;
    animation: saving-spin 0.8s linear infinite;
    vertical-align: -1px;
    margin-right: 4px;
  }
  .saving-spinner.big {
    width: 18px;
    height: 18px;
    border-width: 3px;
    margin-right: 0;
  }
  @keyframes saving-spin {
    to { transform: rotate(360deg); }
  }

  /* 首次启动弹窗 */
  .first-launch-overlay {
    z-index: 2000;
  }
  .first-launch-panel {
    min-width: 520px;
    max-width: 640px;
  }
  .first-launch-hint {
    margin: 0 0 14px 0;
    color: var(--color-text);
    line-height: 1.6;
    font-size: 13px;
  }
  .first-launch-hint code,
  .first-launch-tip code {
    background: var(--color-hover);
    padding: 1px 6px;
    border-radius: var(--radius-xs);
    font-size: 12px;
  }
  .first-launch-tip {
    margin: 12px 0 0 0;
    color: var(--color-muted);
    font-size: 12px;
    line-height: 1.5;
  }

  /* copy_custom 工作目录的只读显示 */
  .custom-dir-display {
    flex: 1;
    padding: 6px 10px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-canvas);
    color: var(--color-text);
    font-size: 13px;
    font-family: var(--font-mono, monospace);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }

  .section-hint {
    margin: -4px 0 8px 0;
    color: var(--color-muted);
    font-size: 12px;
    line-height: 1.5;
  }
  .toggle-row .set-label small {
    color: var(--color-muted);
    font-size: 11px;
    margin-left: 4px;
  }
</style>
