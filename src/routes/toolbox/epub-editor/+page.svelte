<script lang="ts">
  import { base } from "$app/paths";
  import { page } from "$app/stores";
  import { onDestroy, onMount, tick } from "svelte";
  import { EditorView } from "@codemirror/view";
  import CustomSelect from "$lib/CustomSelect.svelte";
  import EpubCodeEditor from "$lib/EpubCodeEditor.svelte";
  import FileTreeItem from "$lib/FileTreeItem.svelte";
  import TocNode from "$lib/TocNode.svelte";
  import ToolImportPage from "$lib/ToolImportPage.svelte";
  import { hasUnsavedEpubChanges } from "$lib/unsavedChanges";
  import {
    addWebEpubResource,
    deleteWebEpubResource,
    exportWebEpubBlob,
    guessWebEpubMediaType,
    loadWebEpub,
    normalizeZipPath,
    readWebEpubBlob,
    readWebEpubText,
    renameWebEpubResource,
    updateWebEpubMetadata,
    updateWebEpubText,
    type WebEpubDocument,
    type WebEpubFileEntry,
    type WebEpubMetadata,
    type WebEpubNavItem,
  } from "$lib/webEpub";

  type WebFileTreeNode = {
    name: string;
    path: string;
    file_type: "folder" | "html" | "css" | "xml" | "image" | "font" | "text" | "other";
    children?: WebFileTreeNode[];
    file?: WebEpubFileEntry;
    size?: number;
    mediaType?: string;
    kind?: WebEpubFileEntry["kind"];
    editable?: boolean;
  };

  type WebTocTreeNode = {
    id: string;
    label: string;
    src: string;
    children?: WebTocTreeNode[];
  };

  function appPath(path: string) {
    return `${base}${path.startsWith("/") ? path : `/${path}`}`;
  }

  const FIND_MODE_OPTIONS = [
    { value: "text", label: "普通文本" },
    { value: "regex", label: "正则表达式" },
  ];

  const SEARCH_SCOPE_OPTIONS = [
    { value: "current", label: "当前文件" },
    { value: "open", label: "已打开" },
    { value: "html", label: "HTML文件" },
    { value: "selected", label: "选中文件" },
    { value: "all", label: "全部文件" },
  ];

  const SEARCH_DIRECTION_OPTIONS = [
    { value: "down", label: "向下" },
    { value: "up", label: "向上" },
  ];

  let fileInput: HTMLInputElement | null = null;
  let resourceInput: HTMLInputElement | null = null;
  let doc: WebEpubDocument | null = null;
  let selectedFile: WebEpubFileEntry | null = null;
  let selectedImage: WebEpubFileEntry | null = null;
  let editorText = "";
  let savedEditorText = "";
  let previewHtml = "";
  let previewBlobUrls: string[] = [];
  let imagePreviewUrl = "";
  let coverPreviewUrl = "";
  let epubCodeEditorComponent: EpubCodeEditor | null = null;
  let metadataDraft: WebEpubMetadata = emptyMetadata();
  let metadataDirty = false;
  let documentDirty = false;
  let busy = false;
  let status = "选择 EPUB 文件后在浏览器内解包，编辑文本资源并导出新文件。";
  let activeTab: "files" | "metadata" = "files";
  let rightTab: "preview" | "toc" = "preview";
  let restorePageOverflow: (() => void) | null = null;
  let expandedFolders = new Set<string>();
  let multiSelectedFiles = new Set<string>();
  let selectedTreePath = "";
  let pendingImportFolder = "";
  let fileMenu: { path: string; x: number; y: number } | null = null;
  let fileTitleMap = new Map<string, string>();
  let findPattern = "";
  let replacePattern = "";
  let findHistory: string[] = [];
  let replaceHistory: string[] = [];
  let showFindHistory = false;
  let showReplaceHistory = false;
  let isRegex = false;
  let searchScope: "current" | "open" | "html" | "selected" | "all" = "current";
  let searchDirection: "down" | "up" = "down";
  let wrapAround = true;
  let textOnly = false;
  let searchMessage = "";
  let currentMatchInfo: { path: string; from: number; to: number } | null = null;
  let previewBuildId = 0;
  let readerPreviewFontSize = 18;
  let readerTocCollapsed = false;
  $: readerMode = $page.url.searchParams.get("mode") === "reader";
  $: headTitle = readerMode ? "Web EPUB 阅读器 - TEpub Editor" : "Web EPUB 编辑器 - TEpub Editor";
  $: importHeading = readerMode ? "选择 EPUB 文件开始阅读" : "选择 EPUB 文件开始编辑";
  $: importDescription = readerMode
    ? "导入后在浏览器内解包，左侧显示目录，中间以宽屏滚动预览方式阅读。"
    : "支持无 DRM 的标准 EPUB，导入后可编辑文件结构、资源和元数据。";
  $: importAction = readerMode ? "选择 EPUB 文件" : "选择 EPUB 文件";
  $: currentPreviewTitle = selectedFile
    ? (fileTitleMap.get(selectedFile.path) || selectedFile.name)
    : (doc?.metadata.title || doc?.fileName || "未选择章节");

  $: dirty = editorText !== savedEditorText;
  $: hasUnsavedChanges = hasUnsavedEpubChanges(readerMode, Boolean(doc), dirty, metadataDirty, documentDirty);
  $: editableFiles = doc?.files.filter((file) => file.editable) ?? [];
  $: imageFiles = doc?.files.filter((file) => file.kind === "image") ?? [];
  $: imageCount = doc?.files.filter((file) => file.kind === "image").length ?? 0;
  $: textCount = doc?.files.filter((file) => file.editable).length ?? 0;
  $: fileTree = doc
    ? buildFileTree(
        doc.files,
        doc.spine.map((item) => item.manifest?.fullPath || "").filter(Boolean),
        collectNavPaths(doc.navItems),
      )
    : [];
  $: selectedTreeNode = selectedTreePath ? { path: selectedTreePath } : (selectedFile || selectedImage);
  $: tocTree = doc ? buildTocTree(doc.navItems) : [];

  function fileByPath(path: string | undefined) {
    if (!doc || !path) return null;
    return doc.files.find((file) => file.path === path) || null;
  }

  function handleBeforeUnload(event: BeforeUnloadEvent) {
    if (!hasUnsavedChanges) return;
    event.preventDefault();
    event.returnValue = "";
  }

  function isEditablePath(path: string | undefined) {
    return !!fileByPath(path)?.editable;
  }

  function treeFileType(kind: WebEpubFileEntry["kind"]): WebFileTreeNode["file_type"] {
    if (kind === "xhtml") return "html";
    return kind;
  }

  function collectNavPaths(items: WebEpubNavItem[]) {
    const paths: string[] = [];
    for (const item of items) {
      if (item.fullPath) paths.push(item.fullPath);
      if (item.children) paths.push(...collectNavPaths(item.children));
    }
    return paths;
  }

  function sortTreeNodes(nodes: WebFileTreeNode[], parentName = "", spinePaths: string[] = [], tocPaths: string[] = []) {
    const rootPriority = ["oebps", "meta-inf"];
    const oebpsFilePriority = ["content.opf", "toc.ncx"];
    const oebpsFolderPriority = ["text", "styles", "fonts", "images"];

    function weight(node: WebFileTreeNode) {
      const name = node.name.toLowerCase();
      if (!parentName) {
        const index = rootPriority.indexOf(name);
        return index >= 0 ? index : 100;
      }
      if (parentName === "oebps") {
        if (node.file_type !== "folder") {
          const index = oebpsFilePriority.indexOf(name);
          return index >= 0 ? index : 200;
        }
        const index = oebpsFolderPriority.indexOf(name);
        return index >= 0 ? 300 + index : 400;
      }
      if (parentName === "text") {
        const spineIndex = spinePaths.indexOf(node.path);
        if (spineIndex >= 0) return spineIndex;
        const tocIndex = tocPaths.indexOf(node.path);
        if (tocIndex >= 0) return 10000 + tocIndex;
        return 20000;
      }
      return 0;
    }

    nodes.sort((left, right) => {
      const leftWeight = weight(left);
      const rightWeight = weight(right);
      if (leftWeight !== rightWeight) return leftWeight - rightWeight;
      return left.name.localeCompare(right.name, "zh-Hans-CN", { numeric: true, sensitivity: "base" });
    });
    for (const node of nodes) {
      if (node.children) sortTreeNodes(node.children, node.name.toLowerCase(), spinePaths, tocPaths);
    }
    return nodes;
  }

  function buildFileTree(files: WebEpubFileEntry[], spinePaths: string[] = [], tocPaths: string[] = []) {
    const roots: WebFileTreeNode[] = [];
    const folders = new Map<string, WebFileTreeNode>();

    function getFolder(path: string, name: string, target: WebFileTreeNode[]) {
      const existing = folders.get(path);
      if (existing) return existing;
      const folder: WebFileTreeNode = { name, path, file_type: "folder", children: [] };
      folders.set(path, folder);
      target.push(folder);
      return folder;
    }

    for (const file of files) {
      const parts = file.path.split("/").filter(Boolean);
      let target = roots;
      let folderPath = "";
      for (const part of parts.slice(0, -1)) {
        folderPath = folderPath ? `${folderPath}/${part}` : part;
        const folder = getFolder(folderPath, part, target);
        target = folder.children || [];
      }
      target.push({
        name: file.name,
        path: file.path,
        file_type: treeFileType(file.kind),
        file,
        size: file.size,
        mediaType: file.mediaType,
        kind: file.kind,
        editable: file.editable,
      });
    }

    return sortTreeNodes(roots, "", spinePaths, tocPaths);
  }

  function defaultExpandedFolders(files: WebEpubFileEntry[]) {
    const next = new Set<string>();
    for (const file of files) {
      const parts = file.path.split("/").filter(Boolean);
      let folderPath = "";
      for (const part of parts.slice(0, -1)) {
        folderPath = folderPath ? `${folderPath}/${part}` : part;
        if (!folderPath.includes("/")) next.add(folderPath);
      }
    }
    return next;
  }

  function expandParentFolders(path: string) {
    const parts = path.split("/").filter(Boolean);
    const next = new Set(expandedFolders);
    let folderPath = "";
    for (const part of parts.slice(0, -1)) {
      folderPath = folderPath ? `${folderPath}/${part}` : part;
      next.add(folderPath);
    }
    expandedFolders = next;
  }

  async function revealTreePath(path: string) {
    activeTab = "files";
    expandParentFolders(path);
    await tick();
    const escapedPath = typeof CSS !== "undefined" && CSS.escape
      ? CSS.escape(path)
      : path.replace(/["\\]/g, "\\$&");
    const element = document.querySelector(`.tree-node[data-path="${escapedPath}"]`);
    element?.scrollIntoView({ block: "center", behavior: "smooth" });
  }

  function toggleFolder(path: string) {
    const next = new Set(expandedFolders);
    if (next.has(path)) next.delete(path);
    else next.add(path);
    expandedFolders = next;
  }

  async function selectTreeFile(node: WebFileTreeNode) {
    const file = node.file || fileByPath(node.path);
    if (!file) return;
    selectedTreePath = file.path;
    expandParentFolders(file.path);
    if (file.kind === "image") {
      await openImage(file);
      return;
    }
    if (file.editable) {
      await openFile(file);
      return;
    }
    status = `${file.path} 暂不支持在 Web 版中直接编辑。`;
  }

  function getFileIcon(type: string) {
    switch (type) {
      case "folder":
        return "📁";
      case "html":
        return "📄";
      case "css":
        return "🎨";
      case "xml":
        return "⚙️";
      case "image":
        return "🖼️";
      case "font":
        return "🔤";
      default:
        return "📄";
    }
  }

  function getFileDescription(node: WebFileTreeNode) {
    if (node.file_type === "folder") return "";
    if (node.file_type === "html") {
      const title = fileTitleMap.get(node.path);
      if (title) return title;
    }
    const name = node.name.toLowerCase();
    if (name === "container.xml") return "容器配置";
    if (name.endsWith(".opf")) return "元数据";
    if (name.includes("toc") || name.endsWith(".ncx")) return "目录结构";
    if (node.file_type === "css") return "样式表";
    if (node.file_type === "font") return `font · ${fileSizeLabel(node.size || 0)}`;
    if (node.file_type === "image") return `image · ${fileSizeLabel(node.size || 0)}`;
    return `${node.file_type} · ${fileSizeLabel(node.size || 0)}`;
  }

  function getFileLanguage(kind: WebEpubFileEntry["kind"]): "html" | "css" | "xml" | "other" {
    if (kind === "xhtml") return "html";
    if (kind === "css") return "css";
    if (kind === "xml") return "xml";
    return "other";
  }

  function handleEditorChange(content: string) {
    editorText = content;
    if (selectedFile?.kind !== "xhtml") previewHtml = "";
  }

  function currentEditorContent() {
    return epubCodeEditorComponent?.getView()?.state.doc.toString() ?? editorText;
  }

  function currentEditorCursor(direction: "down" | "up", fallbackLength: number) {
    const selection = epubCodeEditorComponent?.getView()?.state.selection.main;
    if (!selection) return direction === "up" ? fallbackLength : 0;
    return direction === "up" ? selection.from : selection.to;
  }

  function selectEditorRange(from: number, to: number) {
    const view = epubCodeEditorComponent?.getView();
    if (!view) return;
    view.focus();
    view.dispatch({
      selection: { anchor: from, head: to },
      effects: EditorView.scrollIntoView(from, { y: "center" }),
    });
  }

  function buildTocTree(items: WebEpubNavItem[], prefix = "toc") {
    return items.map((item, index): WebTocTreeNode => ({
      id: `${prefix}-${index}`,
      label: item.label || item.href || `目录 ${index + 1}`,
      src: item.fullPath,
      children: item.children?.length ? buildTocTree(item.children, `${prefix}-${index}`) : undefined,
    }));
  }

  function pickFile() {
    fileInput?.click();
  }

  function emptyMetadata(): WebEpubMetadata {
    return {
      title: "",
      creator: "",
      language: "zh-CN",
      identifier: "",
      description: "",
      publisher: "",
      date: "",
      subject: "",
    };
  }

  function resetMetadataDraft() {
    metadataDraft = doc ? { ...doc.metadata } : emptyMetadata();
    metadataDirty = false;
  }

  function markMetadataDirty() {
    metadataDirty = true;
  }

  function extractXhtmlTitle(source: string) {
    try {
      const parsed = new DOMParser().parseFromString(source, "application/xhtml+xml");
      const title = parsed.querySelector("title")?.textContent?.replace(/\s+/g, " ").trim()
        || parsed.querySelector("h1,h2,h3")?.textContent?.replace(/\s+/g, " ").trim()
        || "";
      return title.slice(0, 80);
    } catch {
      const match = source.match(/<(?:title|h1|h2|h3)[^>]*>([\s\S]*?)<\/(?:title|h1|h2|h3)>/i);
      return (match?.[1] || "").replace(/<[^>]+>/g, " ").replace(/\s+/g, " ").trim().slice(0, 80);
    }
  }

  async function loadFileTitles() {
    if (!doc) {
      fileTitleMap = new Map();
      return;
    }
    const next = new Map<string, string>();
    const files = doc.files.filter((file) => file.kind === "xhtml");
    for (const file of files) {
      try {
        const title = extractXhtmlTitle(await readWebEpubText(doc, file.path));
        if (title) next.set(file.path, title);
      } catch {
        // Ignore unreadable title candidates; opening the file will still report the real error.
      }
    }
    fileTitleMap = next;
  }

  onMount(() => {
    loadSearchHistory();
    const bodyOverflow = document.body.style.overflow;
    const htmlOverflow = document.documentElement.style.overflow;
    document.body.style.overflow = "hidden";
    document.documentElement.style.overflow = "hidden";
    restorePageOverflow = () => {
      document.body.style.overflow = bodyOverflow;
      document.documentElement.style.overflow = htmlOverflow;
    };
  });

  onDestroy(() => {
    revokePreviewUrls();
    revokeImagePreviewUrl();
    revokeCoverPreviewUrl();
    restorePageOverflow?.();
  });

  async function loadEpubFile(file: File) {
    busy = true;
    status = "正在解包 EPUB...";
    selectedFile = null;
    editorText = "";
    savedEditorText = "";
    selectedImage = null;
    selectedTreePath = "";
    fileTitleMap = new Map();
    previewHtml = "";
    resetMetadataDraft();
    revokePreviewUrls();
    revokeImagePreviewUrl();
    revokeCoverPreviewUrl();

    try {
      doc = await loadWebEpub(file);
      documentDirty = false;
      expandedFolders = defaultExpandedFolders(doc.files);
      resetMetadataDraft();
      await loadFileTitles();
      await loadCoverPreview();
      status = `已导入 ${file.name}，读取到 ${doc.files.length} 个文件 / ${doc.spine.length} 个阅读项`;
      const firstEditable = doc.spine.map((item) => fileByPath(item.manifest?.fullPath)).find((entry): entry is WebEpubFileEntry => !!entry?.editable)
        || doc.files.find((entry) => entry.editable)
        || null;
      busy = false;
      if (firstEditable) await openFile(firstEditable, true);
    } catch (error) {
      doc = null;
      fileTitleMap = new Map();
      resetMetadataDraft();
      selectedImage = null;
      revokeImagePreviewUrl();
      revokeCoverPreviewUrl();
      status = `导入失败：${String(error)}`;
    } finally {
      busy = false;
      if (fileInput) fileInput.value = "";
    }
  }

  async function onFileChange(event: Event) {
    const file = (event.currentTarget as HTMLInputElement).files?.[0];
    if (!file) return;
    await loadEpubFile(file);
  }

  function handleImportFiles(event: CustomEvent<File[]>) {
    const file = event.detail[0];
    if (file) void loadEpubFile(file);
  }

  function persistCurrentFile(options: { refreshPreview?: boolean } = {}) {
    if (readerMode) return;
    if (!doc || !selectedFile || !dirty) return;
    const content = currentEditorContent();
    editorText = content;
    updateWebEpubText(doc, selectedFile.path, content);
    documentDirty = true;
    savedEditorText = content;
    if (options.refreshPreview && selectedFile.kind === "xhtml") {
      void refreshPreviewFromEditor();
    }
  }

  async function openFile(file: WebEpubFileEntry, force = false) {
    if (!doc || !file.editable || (busy && !force)) return false;
    persistCurrentFile({ refreshPreview: false });
    busy = true;
    status = `正在读取 ${file.path}`;
    try {
      selectedFile = file;
      selectedTreePath = file.path;
      expandParentFolders(file.path);
      editorText = await readWebEpubText(doc, file.path);
      savedEditorText = editorText;
      if (file.kind === "xhtml") {
        previewHtml = "";
        rightTab = "preview";
        void refreshPreviewFor(file, editorText);
      } else {
        revokePreviewUrls();
        previewHtml = "";
      }
      currentMatchInfo = null;
      status = `已打开 ${file.path}`;
      return true;
    } catch (error) {
      status = `读取失败：${String(error)}`;
      return false;
    } finally {
      busy = false;
    }
  }

  async function openPath(path: string | undefined) {
    const file = fileByPath(path);
    if (!file) return false;
    if (file.kind === "image") return await openImage(file);
    if (file.editable) return await openFile(file);
    return false;
  }

  function stageCurrentFile() {
    persistCurrentFile({ refreshPreview: true });
  }

  async function refreshPreviewFromEditor() {
    if (!doc || !selectedFile || selectedFile.kind !== "xhtml") {
      previewHtml = "";
      revokePreviewUrls();
      return;
    }
    await refreshPreviewFor(selectedFile, currentEditorContent());
  }

  async function refreshPreviewFor(file: WebEpubFileEntry, source: string) {
    if (!doc || file.kind !== "xhtml") return;
    const requestId = ++previewBuildId;
    try {
      const result = await buildPreviewHtmlFor(file, source);
      if (requestId !== previewBuildId || selectedFile?.path !== file.path) {
        for (const url of result.urls) URL.revokeObjectURL(url);
        return;
      }
      revokePreviewUrls();
      previewBlobUrls = result.urls;
      previewHtml = result.html;
      rightTab = "preview";
    } catch (error) {
      if (requestId !== previewBuildId) return;
      previewHtml = "";
      status = `预览失败：${String(error)}`;
      revokePreviewUrls();
    }
  }

  function prepareOpfMutation() {
    if (!doc || selectedFile?.path !== doc.opfPath || !dirty) return true;
    const ok = window.confirm("当前 OPF 源码有未暂存修改，继续会先暂存 OPF 再写入资源清单。是否继续？");
    if (!ok) return false;
    stageCurrentFile();
    return true;
  }

  async function syncOpenOpfSource() {
    if (!doc || selectedFile?.path !== doc.opfPath) return;
    editorText = await readWebEpubText(doc, doc.opfPath);
    savedEditorText = editorText;
    previewHtml = "";
    revokePreviewUrls();
  }

  function isLikelyCover(file: WebEpubFileEntry) {
    if (!doc) return false;
    const manifestItem = doc.manifest.find((item) => item.fullPath === file.path);
    const haystack = `${manifestItem?.id || ""} ${manifestItem?.href || ""} ${manifestItem?.properties || ""} ${file.name}`.toLowerCase();
    return haystack.includes("cover-image") || /\bcover\b/.test(haystack) || haystack.includes("封面");
  }

  function pickInitialImage() {
    const files = doc?.files.filter((file) => file.kind === "image") ?? [];
    return files.find(isLikelyCover) || files[0] || null;
  }

  function revokeImagePreviewUrl() {
    if (imagePreviewUrl) URL.revokeObjectURL(imagePreviewUrl);
    imagePreviewUrl = "";
  }

  function revokeCoverPreviewUrl() {
    if (coverPreviewUrl) URL.revokeObjectURL(coverPreviewUrl);
    coverPreviewUrl = "";
  }

  async function loadCoverPreview() {
    revokeCoverPreviewUrl();
    const cover = pickInitialImage();
    if (!doc || !cover) return;
    try {
      const blob = await readWebEpubBlob(doc, cover.path, mimeForPath(cover.path));
      coverPreviewUrl = URL.createObjectURL(blob);
    } catch {
      coverPreviewUrl = "";
    }
  }

  async function openImage(file: WebEpubFileEntry, force = false) {
    if (!doc || file.kind !== "image" || (busy && !force)) return false;
    persistCurrentFile({ refreshPreview: false });
    busy = true;
    status = `正在读取图片 ${file.path}`;
    try {
      const blob = await readWebEpubBlob(doc, file.path, mimeForPath(file.path));
      revokeImagePreviewUrl();
      selectedImage = file;
      selectedFile = null;
      selectedTreePath = file.path;
      expandParentFolders(file.path);
      editorText = "";
      savedEditorText = "";
      previewHtml = "";
      revokePreviewUrls();
      currentMatchInfo = null;
      imagePreviewUrl = URL.createObjectURL(blob);
      status = `已打开图片 ${file.path}`;
      return true;
    } catch (error) {
      status = `图片读取失败：${String(error)}`;
      return false;
    } finally {
      busy = false;
    }
  }

  function safeResourceName(name: string) {
    const cleaned = name.trim().replace(/[\\:*?"<>|]+/g, "_").replace(/\s+/g, "_");
    return cleaned || "resource";
  }

  function uniqueResourcePath(path: string) {
    if (!doc) return normalizeZipPath(path);
    const clean = normalizeZipPath(path);
    if (!doc.zip.file(clean)) return clean;
    const dot = clean.lastIndexOf(".");
    const slash = clean.lastIndexOf("/");
    const base = dot > slash ? clean.slice(0, dot) : clean;
    const ext = dot > slash ? clean.slice(dot) : "";
    let index = 2;
    let next = `${base}-${index}${ext}`;
    while (doc.zip.file(next)) {
      index += 1;
      next = `${base}-${index}${ext}`;
    }
    return next;
  }

  function defaultImportPath(fileName: string) {
    if (!doc) return safeResourceName(fileName);
    const mediaType = guessWebEpubMediaType(fileName);
    const name = safeResourceName(fileName);
    if (mediaType.startsWith("image/")) return uniqueResourcePath(`${doc.opfDir}Images/${name}`);
    if (mediaType === "text/css") return uniqueResourcePath(`${doc.opfDir}Styles/${name}`);
    if (mediaType.includes("font")) return uniqueResourcePath(`${doc.opfDir}Fonts/${name}`);
    if (mediaType === "application/xhtml+xml") return uniqueResourcePath(`${doc.opfDir}Text/${name}`);
    return uniqueResourcePath(`${doc.opfDir}${name}`);
  }

  function folderImportPath(folderPath: string, fileName: string) {
    const folder = normalizeZipPath(folderPath);
    const name = safeResourceName(fileName);
    return uniqueResourcePath(folder ? `${folder}/${name}` : name);
  }

  function pickFolderResourceImport(node: WebFileTreeNode, event?: MouseEvent) {
    event?.stopPropagation();
    if (!doc || busy || node.file_type !== "folder") return;
    pendingImportFolder = node.path;
    resourceInput?.click();
  }

  async function dropFilesIntoFolder(node: WebFileTreeNode, files: FileList) {
    if (!doc || busy || node.file_type !== "folder" || files.length === 0) return;
    await importResourceFiles(Array.from(files), node.path, false);
  }

  async function importResourceFiles(files: File[], targetFolder = "", askPath = true) {
    if (!doc || files.length === 0) return;
    if (!prepareOpfMutation()) {
      if (resourceInput) resourceInput.value = "";
      return;
    }
    busy = true;
    const importedPaths: string[] = [];
    let lastEntry: WebEpubFileEntry | undefined;
    try {
      for (const file of files) {
        const initialPath = targetFolder ? folderImportPath(targetFolder, file.name) : defaultImportPath(file.name);
        const requestedPath = askPath && files.length === 1
          ? window.prompt("导入到 EPUB 内部路径：", initialPath)
          : initialPath;
        if (!requestedPath) continue;
        const path = uniqueResourcePath(requestedPath);
        const mediaType = file.type || guessWebEpubMediaType(path);
        const addToSpine = mediaType === "application/xhtml+xml"
          && (files.length === 1 ? window.confirm("将该 XHTML 加入阅读顺序？") : /\/text\//i.test(path));
        status = `正在导入资源 ${path}`;
        const entry = await addWebEpubResource(doc, {
          path,
          content: await file.arrayBuffer(),
          mediaType,
          addToSpine,
          afterPath: selectedFile?.path,
        });
        importedPaths.push(path);
        expandParentFolders(path);
        lastEntry = entry;
      }
      doc = doc;
      documentDirty = true;
      await loadCoverPreview();
      await loadFileTitles();
      if (lastEntry && files.length === 1) {
        if (lastEntry.editable) await openFile(lastEntry, true);
        else if (lastEntry.kind === "image") await openImage(lastEntry, true);
        else selectedTreePath = lastEntry.path;
      } else {
        await syncOpenOpfSource();
      }
      status = importedPaths.length > 0
        ? `已导入 ${importedPaths.length} 个资源。`
        : "未导入资源。";
    } catch (error) {
      status = `导入资源失败：${String(error)}`;
    } finally {
      busy = false;
      pendingImportFolder = "";
      if (resourceInput) resourceInput.value = "";
    }
  }

  async function onResourceImport(event: Event) {
    const files = Array.from((event.currentTarget as HTMLInputElement).files || []);
    if (!doc || files.length === 0) return;
    await importResourceFiles(files, pendingImportFolder, !pendingImportFolder);
  }

  function openFileMenu(node: WebFileTreeNode, event: MouseEvent) {
    if (node.file_type === "folder" || !node.path) return;
    event.stopPropagation();
    fileMenu = { path: node.path, x: event.clientX, y: event.clientY };
  }

  function closeFileMenu() {
    fileMenu = null;
  }

  async function deleteResourcePath(path: string) {
    if (!doc || busy) return;
    const current = fileByPath(path);
    if (!current) return;
    const currentPath = current.path;
    if (currentPath === doc.opfPath) {
      status = "不能删除 OPF 核心文件。";
      return;
    }
    const ok = window.confirm(`确认删除资源？\n${currentPath}\n\n会同步移除 OPF manifest/spine 引用，导出后生效。`);
    if (!ok) return;
    busy = true;
    status = `正在删除 ${currentPath}`;
    try {
      await deleteWebEpubResource(doc, currentPath);
      doc = doc;
      documentDirty = true;
      if (selectedFile?.path === currentPath) {
        selectedFile = null;
        editorText = "";
        savedEditorText = "";
        previewHtml = "";
        revokePreviewUrls();
      }
      if (selectedImage?.path === currentPath) {
        selectedImage = null;
        revokeImagePreviewUrl();
      }
      selectedTreePath = "";
      await loadCoverPreview();
      await loadFileTitles();
      status = `已删除 ${currentPath}`;
    } catch (error) {
      status = `删除失败：${String(error)}`;
    } finally {
      busy = false;
      closeFileMenu();
    }
  }

  function renamedPathForInput(path: string, input: string) {
    const value = input.trim().replace(/[\\:*?"<>|]+/g, "_");
    if (!value) return "";
    if (value.includes("/")) return normalizeZipPath(value);
    return normalizeZipPath(`${dirname(path)}${value}`);
  }

  async function renameResourcePath(path: string) {
    if (!doc || busy) return;
    const current = fileByPath(path);
    if (!current) return;
    if (path === doc.opfPath) {
      status = "不能重命名 OPF 核心文件。";
      return;
    }
    const requestedName = window.prompt("新的文件名或 EPUB 内部路径：", current.name);
    if (requestedName === null) return;
    const nextPath = renamedPathForInput(path, requestedName);
    if (!nextPath || nextPath === path) return;
    if (selectedFile?.path === path && dirty) stageCurrentFile();
    busy = true;
    status = `正在重命名 ${path}`;
    try {
      const renamed = await renameWebEpubResource(doc, path, nextPath);
      doc = doc;
      documentDirty = true;
      if (selectedFile?.path === path) {
        selectedFile = renamed || fileByPath(nextPath);
        selectedTreePath = nextPath;
      } else if (selectedImage?.path === path) {
        selectedImage = renamed || fileByPath(nextPath);
        selectedTreePath = nextPath;
      } else if (selectedTreePath === path) {
        selectedTreePath = nextPath;
      }
      expandParentFolders(nextPath);
      await loadCoverPreview();
      await loadFileTitles();
      await syncOpenOpfSource();
      status = `已重命名为 ${nextPath}`;
    } catch (error) {
      status = `重命名失败：${String(error)}`;
    } finally {
      busy = false;
      closeFileMenu();
    }
  }

  async function saveMetadata() {
    if (!doc || busy || !metadataDirty) return;
    if (selectedFile?.path === doc.opfPath && dirty) {
      const ok = window.confirm("当前 OPF 源码有未暂存修改，保存元数据会重新写入 OPF。是否继续？");
      if (!ok) return;
    }
    busy = true;
    status = "正在写入 OPF 元数据...";
    try {
      const saved = await updateWebEpubMetadata(doc, metadataDraft);
      documentDirty = true;
      metadataDraft = { ...saved };
      metadataDirty = false;
      if (selectedFile?.path === doc.opfPath) {
        editorText = await readWebEpubText(doc, doc.opfPath);
        savedEditorText = editorText;
        previewHtml = "";
        revokePreviewUrls();
      }
      status = "元数据已写入，导出 EPUB 后生效。";
    } catch (error) {
      status = `元数据保存失败：${String(error)}`;
    } finally {
      busy = false;
    }
  }

  async function exportEpub() {
    if (!doc || busy) return;
    if (dirty) stageCurrentFile();
    busy = true;
    status = "正在重新打包 EPUB...";
    try {
      const blob = await exportWebEpubBlob(doc);
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement("a");
      anchor.href = url;
      anchor.download = outputFileName();
      anchor.click();
      URL.revokeObjectURL(url);
      documentDirty = false;
      status = `已导出 ${anchor.download}`;
    } catch (error) {
      status = `导出失败：${String(error)}`;
    } finally {
      busy = false;
    }
  }

  function outputFileName() {
    if (!doc) return "edited.epub";
    const stem = doc.fileName.replace(/\.epub$/i, "") || "edited";
    return `${stem}.web-edited.epub`;
  }

  function fileSizeLabel(size: number) {
    if (!size) return "-";
    if (size < 1024) return `${size} B`;
    if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)} KB`;
    return `${(size / 1024 / 1024).toFixed(1)} MB`;
  }

  function dirname(path: string) {
    const index = path.lastIndexOf("/");
    return index >= 0 ? path.slice(0, index + 1) : "";
  }

  function resolveRelativePath(basePath: string, href: string) {
    const cleanHref = href.split("#")[0] || "";
    if (!cleanHref || /^(data:|blob:|https?:|mailto:|tel:|#)/i.test(cleanHref)) return "";
    try {
      return normalizeZipPath(`${dirname(basePath)}${decodeURIComponent(cleanHref).replace(/^\//, "")}`);
    } catch {
      return normalizeZipPath(`${dirname(basePath)}${cleanHref.replace(/^\//, "")}`);
    }
  }

  function mimeForPath(path: string) {
    const file = fileByPath(path);
    if (file?.mediaType) return file.mediaType;
    const ext = path.split(".").pop()?.toLowerCase() || "";
    if (ext === "css") return "text/css";
    if (ext === "svg") return "image/svg+xml";
    if (ext === "png") return "image/png";
    if (ext === "jpg" || ext === "jpeg") return "image/jpeg";
    if (ext === "webp") return "image/webp";
    if (ext === "gif") return "image/gif";
    if (ext === "woff") return "font/woff";
    if (ext === "woff2") return "font/woff2";
    return "application/octet-stream";
  }

  function previewableSpineFiles() {
    if (!doc) return [];
    return doc.spine
      .map((item) => fileByPath(item.manifest?.fullPath))
      .filter((file): file is WebEpubFileEntry => !!file && file.kind === "xhtml" && file.editable);
  }

  function currentPreviewIndex() {
    if (!selectedFile) return -1;
    return previewableSpineFiles().findIndex((file) => file.path === selectedFile?.path);
  }

  function previewNavLabel() {
    const files = previewableSpineFiles();
    const index = currentPreviewIndex();
    if (index < 0 || files.length === 0) return "未定位到书脊";
    return `${index + 1} / ${files.length}`;
  }

  function canPreviewRelative(delta: number) {
    const files = previewableSpineFiles();
    const index = currentPreviewIndex();
    if (files.length === 0) return false;
    const nextIndex = index < 0 ? 0 : index + delta;
    return nextIndex >= 0 && nextIndex < files.length;
  }

  function canAdjustReaderFont(delta: number) {
    const nextSize = readerPreviewFontSize + delta;
    return nextSize >= 14 && nextSize <= 26;
  }

  async function adjustReaderFont(delta: number) {
    if (!readerMode || busy || !canAdjustReaderFont(delta)) return;
    readerPreviewFontSize += delta;
    if (selectedFile?.kind === "xhtml") await refreshPreviewFromEditor();
  }

  function toggleReaderToc() {
    if (!readerMode) return;
    readerTocCollapsed = !readerTocCollapsed;
  }

  function revokePreviewUrls() {
    for (const url of previewBlobUrls) URL.revokeObjectURL(url);
    previewBlobUrls = [];
  }

  function makePreviewUrl(blob: Blob, type: string, urls: string[]) {
    const url = URL.createObjectURL(blob.type === type ? blob : new Blob([blob], { type }));
    urls.push(url);
    return url;
  }

  function rewriteCssUrls(css: string, cssPath: string, urlByPath: Map<string, string>) {
    return css.replace(/url\(\s*(['"]?)(.*?)\1\s*\)/gi, (match, quote, rawUrl) => {
      const targetPath = resolveRelativePath(cssPath, rawUrl.trim());
      const url = targetPath ? urlByPath.get(targetPath) : "";
      return url ? `url(${quote || ""}${url}${quote || ""})` : match;
    });
  }

  function rewriteHtmlResources(source: string, htmlPath: string, urlByPath: Map<string, string>) {
    const htmlDoc = new DOMParser().parseFromString(source, "text/html");
    const attrs = ["src", "href", "poster"];
    for (const element of Array.from(htmlDoc.querySelectorAll("*"))) {
      for (const attr of attrs) {
        const value = element.getAttribute(attr);
        if (!value) continue;
        const targetPath = resolveRelativePath(htmlPath, value);
        const url = targetPath ? urlByPath.get(targetPath) : "";
        if (url) element.setAttribute(attr, url);
      }
    }
    const style = htmlDoc.createElement("style");
    const bodyMaxWidth = readerMode ? "980px" : "820px";
    const bodyPadding = readerMode ? "42px 56px 64px" : "28px 34px";
    const bodyFontSize = readerMode ? `${readerPreviewFontSize}px` : "inherit";
    style.textContent = `
      html, body { margin: 0; padding: 0; background: #fffdf9; color: #172033; }
      html { overflow-x: hidden; }
      body { box-sizing: border-box; max-width: ${bodyMaxWidth}; margin: 0 auto; padding: ${bodyPadding}; font-size: ${bodyFontSize}; line-height: 1.8; overflow-x: hidden; }
      img, svg, video { max-width: 100%; height: auto; }
      .tepub-footnote-backdrop { position: fixed; inset: 0; z-index: 9998; display: flex; align-items: center; justify-content: center; padding: 20px; box-sizing: border-box; background: rgba(15, 23, 42, .72); }
      .tepub-footnote-dialog { position: relative; width: min(90vw, 720px); max-height: 88vh; overflow: auto; padding: 16px; box-sizing: border-box; border-radius: 8px; background: #fff; box-shadow: 0 16px 48px rgba(0, 0, 0, .32); }
      .tepub-footnote-dialog img { display: block; max-width: 100%; max-height: calc(88vh - 40px); width: auto; height: auto; margin: 0 auto; }
      .tepub-footnote-dialog figure { margin: 0; }
    `;
    htmlDoc.head.prepend(style);
    const script = htmlDoc.createElement("script");
    script.textContent = `
      document.addEventListener("click", function (event) {
        var clicked = event.target;
        var trigger = clicked && clicked.closest ? clicked.closest('a[role="doc-noteref"]') : null;
        if (!trigger) return;
        var href = trigger.getAttribute("href") || "";
        if (href.charAt(0) !== "#") return;
        var note = document.getElementById(decodeURIComponent(href.slice(1)));
        if (!note) return;
        event.preventDefault();
        event.stopPropagation();
        var old = document.querySelector(".tepub-footnote-backdrop");
        if (old) old.remove();
        var backdrop = document.createElement("div");
        backdrop.className = "tepub-footnote-backdrop";
        var dialog = document.createElement("div");
        dialog.className = "tepub-footnote-dialog";
        dialog.innerHTML = note.innerHTML;
        backdrop.appendChild(dialog);
        backdrop.addEventListener("click", function (closeEvent) {
          if (closeEvent.target === backdrop) backdrop.remove();
        });
        document.body.appendChild(backdrop);
      }, true);
    `;
    htmlDoc.head.appendChild(script);
    return `<!doctype html>${htmlDoc.documentElement.outerHTML}`;
  }

  function collectHtmlPreviewRefs(source: string, htmlPath: string) {
    const assets = new Set<string>();
    const css = new Set<string>();
    const htmlDoc = new DOMParser().parseFromString(source, "text/html");
    const addPath = (path: string) => {
      const file = fileByPath(path);
      if (!file || file.path === htmlPath) return;
      if (file.kind === "css") css.add(file.path);
      else assets.add(file.path);
    };
    for (const element of Array.from(htmlDoc.querySelectorAll("*"))) {
      for (const attr of ["src", "href", "poster"]) {
        const value = element.getAttribute(attr);
        if (!value) continue;
        const path = resolveRelativePath(htmlPath, value);
        if (path) addPath(path);
      }
      const inlineStyle = element.getAttribute("style");
      if (inlineStyle) {
        for (const path of collectCssUrlPaths(inlineStyle, htmlPath)) addPath(path);
      }
    }
    for (const style of Array.from(htmlDoc.querySelectorAll("style"))) {
      for (const path of collectCssUrlPaths(style.textContent || "", htmlPath)) addPath(path);
    }
    return { assets, css };
  }

  function collectCssUrlPaths(css: string, cssPath: string) {
    const paths = new Set<string>();
    css.replace(/url\(\s*(['"]?)(.*?)\1\s*\)/gi, (_match, _quote, rawUrl) => {
      const path = resolveRelativePath(cssPath, rawUrl.trim());
      if (path) paths.add(path);
      return "";
    });
    css.replace(/@import\s+(?:url\(\s*)?(['"])(.*?)\1\s*\)?/gi, (_match, _quote, rawUrl) => {
      const path = resolveRelativePath(cssPath, rawUrl.trim());
      if (path) paths.add(path);
      return "";
    });
    return paths;
  }

  async function buildPreviewHtmlFor(currentFile: WebEpubFileEntry, source: string) {
    if (!doc) return { html: "", urls: [] };
    const urls: string[] = [];
    const urlByPath = new Map<string, string>();
    const refs = collectHtmlPreviewRefs(source, currentFile.path);
    const cssContent = new Map<string, string>();
    const cssQueue = Array.from(refs.css);
    const seenCss = new Set<string>();

    while (cssQueue.length > 0) {
      const path = cssQueue.shift()!;
      if (seenCss.has(path)) continue;
      seenCss.add(path);
      const file = fileByPath(path);
      const zipFile = file ? doc.zip.file(file.path) : null;
      if (!file || !zipFile) continue;
      const css = await zipFile.async("string");
      cssContent.set(file.path, css);
      for (const refPath of collectCssUrlPaths(css, file.path)) {
        const refFile = fileByPath(refPath);
        if (!refFile) continue;
        if (refFile.kind === "css") cssQueue.push(refFile.path);
        else refs.assets.add(refFile.path);
      }
    }

    for (const path of refs.assets) {
      const file = fileByPath(path);
      if (!file) continue;
      const zipFile = doc.zip.file(file.path);
      if (!zipFile) continue;
      const blob = await zipFile.async("blob");
      urlByPath.set(file.path, makePreviewUrl(blob, mimeForPath(file.path), urls));
    }

    for (const [path, css] of cssContent) {
      const rewritten = rewriteCssUrls(css, path, urlByPath);
      urlByPath.set(path, makePreviewUrl(new Blob([rewritten], { type: "text/css" }), "text/css", urls));
    }

    return { html: rewriteHtmlResources(source, currentFile.path, urlByPath), urls };
  }

  async function buildPreviewHtml() {
    if (!doc || !selectedFile) return "";
    const result = await buildPreviewHtmlFor(selectedFile, editorText);
    revokePreviewUrls();
    previewBlobUrls = result.urls;
    return result.html;
  }

  async function previewCurrentFile() {
    if (!selectedFile || busy) return;
    if (selectedFile.kind !== "xhtml") {
      status = "当前文件不是 XHTML/HTML 章节，暂不能预览。";
      return;
    }
    busy = true;
    status = `正在生成 ${selectedFile.path} 的预览`;
    revokePreviewUrls();
    try {
      previewHtml = await buildPreviewHtml();
      rightTab = "preview";
      status = `已生成 ${selectedFile.path} 的预览`;
    } catch (error) {
      previewHtml = "";
      status = `预览失败：${String(error)}`;
      revokePreviewUrls();
    } finally {
      busy = false;
    }
  }

  async function previewFile(file: WebEpubFileEntry) {
    if (busy || file.kind !== "xhtml") return;
    if (selectedFile?.path === file.path) {
      await refreshPreviewFromEditor();
      return;
    }
    await openFile(file);
  }

  async function previewRelative(delta: number) {
    const files = previewableSpineFiles();
    if (files.length === 0) return;
    const index = currentPreviewIndex();
    const nextIndex = index < 0 ? 0 : index + delta;
    const target = files[nextIndex];
    if (target) await previewFile(target);
  }

  async function previewPath(path: string | undefined) {
    const file = fileByPath(path);
    if (!file) return false;
    if (file.kind === "xhtml") {
      await previewFile(file);
      return true;
    }
    return await openPath(path);
  }

  async function handleTocSelect(path: string) {
    const opened = readerMode ? await previewPath(path) : await openPath(path);
    if (opened) await revealTreePath(path);
  }

  function addToHistory(value: string, history: string[]) {
    const clean = value.trim();
    if (!clean) return history;
    return [clean, ...history.filter((item) => item !== clean)].slice(0, 20);
  }

  function saveSearchHistory() {
    localStorage.setItem("epub-find-history", JSON.stringify(findHistory));
    localStorage.setItem("epub-replace-history", JSON.stringify(replaceHistory));
  }

  function loadSearchHistory() {
    try {
      findHistory = JSON.parse(localStorage.getItem("epub-find-history") || "[]");
      replaceHistory = JSON.parse(localStorage.getItem("epub-replace-history") || "[]");
      if (findHistory[0]) findPattern = findHistory[0];
      if (replaceHistory[0]) replacePattern = replaceHistory[0];
    } catch {
      findHistory = [];
      replaceHistory = [];
    }
  }

  function rememberSearchInputs() {
    if (findPattern) findHistory = addToHistory(findPattern, findHistory);
    if (replacePattern) replaceHistory = addToHistory(replacePattern, replaceHistory);
    saveSearchHistory();
  }

  function selectFindHistory(value: string) {
    findPattern = value;
    showFindHistory = false;
  }

  function selectReplaceHistory(value: string) {
    replacePattern = value;
    showReplaceHistory = false;
  }

  function removeFromFindHistory(index: number) {
    findHistory = findHistory.filter((_, i) => i !== index);
    saveSearchHistory();
  }

  function removeFromReplaceHistory(index: number) {
    replaceHistory = replaceHistory.filter((_, i) => i !== index);
    saveSearchHistory();
  }

  function closeFindReplace() {
    showFindHistory = false;
    showReplaceHistory = false;
    currentMatchInfo = null;
    searchMessage = "";
  }

  function getFilesInScope() {
    if (!doc) return [];
    if (searchScope === "current") return selectedFile ? [selectedFile] : [];
    if (searchScope === "open") return selectedFile ? [selectedFile] : [];
    if (searchScope === "html") return doc.files.filter((file) => file.kind === "xhtml" && file.editable);
    if (searchScope === "selected") {
      const selected = Array.from(multiSelectedFiles)
        .map((path) => fileByPath(path))
        .filter((file): file is WebEpubFileEntry => !!file?.editable);
      return selected.length > 0 ? selected : (selectedFile ? [selectedFile] : []);
    }
    return doc.files.filter((file) => file.editable);
  }

  function regexForFind() {
    if (!isRegex) return null;
    return new RegExp(findPattern, "g");
  }

  function findMatchInContent(content: string, start: number, direction: "down" | "up") {
    if (!findPattern) return null;
    if (isRegex) {
      const regex = regexForFind();
      if (!regex) return null;
      const matches: { from: number; to: number }[] = [];
      let match: RegExpExecArray | null;
      while ((match = regex.exec(content))) {
        const from = match.index;
        const to = from + match[0].length;
        if (to === from) regex.lastIndex += 1;
        matches.push({ from, to });
      }
      if (direction === "up") return [...matches].reverse().find((item) => item.from < start) || null;
      return matches.find((item) => item.from >= start) || null;
    }
    if (direction === "up") {
      const index = content.lastIndexOf(findPattern, Math.max(0, start));
      return index >= 0 ? { from: index, to: index + findPattern.length } : null;
    }
    const index = content.indexOf(findPattern, Math.max(0, start));
    return index >= 0 ? { from: index, to: index + findPattern.length } : null;
  }

  async function contentForFile(file: WebEpubFileEntry) {
    if (!doc) return "";
    if (selectedFile?.path === file.path) return currentEditorContent();
    return readWebEpubText(doc, file.path);
  }

  async function selectMatch(file: WebEpubFileEntry, from: number, to: number) {
    if (selectedFile?.path !== file.path) {
      const opened = file.kind === "image" ? await openImage(file) : await openFile(file);
      if (!opened) return;
    }
    currentMatchInfo = { path: file.path, from, to };
    await tick();
    selectEditorRange(from, to);
  }

  async function findNextInDirection(direction: "down" | "up") {
    if (!doc || !selectedFile) return;
    if (!findPattern) {
      searchMessage = "请输入查找内容";
      return;
    }
    rememberSearchInputs();
    const files = getFilesInScope();
    if (files.length === 0) {
      searchMessage = "没有可搜索文件";
      return;
    }
    const currentIndex = Math.max(0, files.findIndex((file) => file.path === selectedFile?.path));
    const ordered = direction === "up"
      ? [...files.slice(0, currentIndex + 1).reverse(), ...files.slice(currentIndex + 1).reverse()]
      : [...files.slice(currentIndex), ...files.slice(0, currentIndex)];

    try {
      for (const file of ordered) {
        const content = await contentForFile(file);
        const isCurrent = file.path === selectedFile.path;
        const cursor = isCurrent
          ? (currentMatchInfo?.path === file.path
            ? (direction === "up" ? currentMatchInfo.from - 1 : currentMatchInfo.to)
            : currentEditorCursor(direction, content.length))
          : (direction === "up" ? content.length : 0);
        let match = findMatchInContent(content, cursor, direction);
        if (!match && wrapAround && isCurrent && files.length === 1) {
          match = findMatchInContent(content, direction === "up" ? content.length : 0, direction);
        }
        if (match) {
          await selectMatch(file, match.from, match.to);
          searchMessage = `已定位 ${file.name}`;
          return;
        }
        if (!wrapAround && isCurrent) break;
      }
    } catch {
      searchMessage = "正则表达式错误";
      return;
    }
    searchMessage = "未找到匹配内容";
  }

  async function findNext() {
    await findNextInDirection(searchDirection);
  }

  async function findPrev() {
    await findNextInDirection(searchDirection === "down" ? "up" : "down");
  }

  async function performReplace() {
    if (!selectedFile || !currentMatchInfo || currentMatchInfo.path !== selectedFile.path) {
      searchMessage = "请先查找";
      return;
    }
    const before = editorText.slice(0, currentMatchInfo.from);
    const after = editorText.slice(currentMatchInfo.to);
    editorText = `${before}${replacePattern}${after}`;
    currentMatchInfo = {
      path: selectedFile.path,
      from: currentMatchInfo.from,
      to: currentMatchInfo.from + replacePattern.length,
    };
    rememberSearchInputs();
    await tick();
    selectEditorRange(currentMatchInfo.from, currentMatchInfo.to);
    searchMessage = "已替换";
    await findNext();
  }

  function replaceAllInContent(content: string) {
    if (!findPattern) return { content, count: 0 };
    if (isRegex) {
      const regex = regexForFind();
      if (!regex) return { content, count: 0 };
      const matches = content.match(regex);
      return { content: content.replace(regex, replacePattern), count: matches?.length || 0 };
    }
    let count = 0;
    let index = 0;
    while ((index = content.indexOf(findPattern, index)) !== -1) {
      count += 1;
      index += findPattern.length;
    }
    return { content: content.split(findPattern).join(replacePattern), count };
  }

  async function performReplaceAll() {
    if (!doc || !findPattern) {
      searchMessage = "请输入查找内容";
      return;
    }
    rememberSearchInputs();
    let total = 0;
    try {
      for (const file of getFilesInScope()) {
        const original = await contentForFile(file);
        const replaced = replaceAllInContent(original);
        if (replaced.count === 0) continue;
        total += replaced.count;
        updateWebEpubText(doc, file.path, replaced.content);
        documentDirty = true;
        if (selectedFile?.path === file.path) {
          editorText = replaced.content;
          savedEditorText = replaced.content;
        }
      }
      if (selectedFile?.kind === "xhtml") await refreshPreviewFromEditor();
      await loadFileTitles();
      currentMatchInfo = null;
      searchMessage = `已替换 ${total} 处`;
    } catch (error) {
      searchMessage = `替换失败：${String(error)}`;
    }
  }

  async function countMatches() {
    if (!findPattern) {
      searchMessage = "请输入查找内容";
      return;
    }
    let total = 0;
    try {
      for (const file of getFilesInScope()) {
        const content = await contentForFile(file);
        if (isRegex) total += content.match(regexForFind() || /$a/g)?.length || 0;
        else {
          let index = 0;
          while ((index = content.indexOf(findPattern, index)) !== -1) {
            total += 1;
            index += findPattern.length;
          }
        }
      }
      searchMessage = `共 ${total} 处匹配`;
    } catch {
      searchMessage = "正则表达式错误";
    }
  }
</script>

<svelte:head>
  <title>{headTitle}</title>
</svelte:head>

<svelte:window on:click={closeFileMenu} on:beforeunload={handleBeforeUnload} />

<div class="web-epub-page">
  <input bind:this={fileInput} class="file-input" type="file" accept=".epub,application/epub+zip" on:change={onFileChange} />
  <input bind:this={resourceInput} class="file-input" type="file" multiple on:change={onResourceImport} />

  {#if !doc}
    <div class="import-shell">
      <ToolImportPage
        mark={readerMode ? "READ" : "EPUB"}
        kicker={readerMode ? "EPUB READER" : "EPUB EDITOR"}
        title={readerMode ? "EPUB 阅读器" : "EPUB 编辑器"}
        description={importDescription}
        privacy={`文件仅在当前设备中读取，选择后继续进入原有${readerMode ? "阅读" : "编辑"}页面。`}
        outputLabel={readerMode ? "阅读格式" : "编辑格式"}
        outputValue="EPUB"
        features={readerMode ? [
          { title: "目录导航", detail: "读取 NAV 与 NCX 章节目录" },
          { title: "正文预览", detail: "按阅读顺序打开 EPUB 内容" },
          { title: "本地阅读", detail: "文件不会上传到服务器" },
        ] : [
          { title: "文件结构", detail: "浏览并管理 EPUB 内部资源" },
          { title: "内容编辑", detail: "编辑 XHTML、CSS 与元数据" },
          { title: "重新导出", detail: "处理完成后生成新的 EPUB" },
        ]}
        prompt={importHeading}
        hint={readerMode ? "选择 EPUB 后进入原有阅读器" : "选择 EPUB 后进入原有编辑器"}
        actionLabel={importAction}
        accept=".epub,application/epub+zip"
        {busy}
        on:select={pickFile}
        on:files={handleImportFiles}
      />
    </div>
  {:else}
    <main class="workspace" class:reader-workspace={readerMode} class:reader-toc-collapsed={readerMode && readerTocCollapsed}>
      <aside class="left-panel">
        <div class="tree-header">
          <h2>{readerMode ? "目录" : "文件结构"}</h2>
          <div class="tree-actions">
            {#if readerMode}
              <button type="button" on:click={pickFile} disabled={busy} title="重新选择 EPUB">换书</button>
            {:else}
              <button type="button" class="primary" on:click={exportEpub} disabled={busy} title="导出 EPUB">导出</button>
            {/if}
          </div>
        </div>

        <section class="book-summary">
          <strong>{doc.metadata.title || doc.fileName}</strong>
          <span>{doc.metadata.creator || "未知作者"}</span>
          <small>{doc.fileName}</small>
        </section>

        {#if readerMode}
          <div class="toc-container reader-toc-container">
            {#if tocTree.length === 0}
              <p class="panel-empty">未读取到 NAV / NCX 目录。</p>
            {:else}
              <div class="toc-list">
                {#each tocTree as item (item.id)}
                  <TocNode {item} selectedSrc={selectedFile?.path || ""} onSelect={handleTocSelect} />
                {/each}
              </div>
            {/if}
          </div>
        {:else}
        <div class="tabs">
          <button type="button" class:active={activeTab === "files"} on:click={() => (activeTab = "files")}>文件</button>
          <button type="button" class:active={activeTab === "metadata"} on:click={() => (activeTab = "metadata")}>信息</button>
        </div>

        {#if activeTab === "files"}
          <div class="tree-content">
            {#each fileTree as node (node.path)}
              <FileTreeItem
                {node}
                {expandedFolders}
                selectedFile={selectedTreeNode}
                {multiSelectedFiles}
                {toggleFolder}
                selectFile={selectTreeFile}
                {getFileIcon}
                {getFileDescription}
                onFolderAdd={pickFolderResourceImport}
                onFolderDrop={dropFilesIntoFolder}
                onFileMenu={openFileMenu}
              />
            {/each}
          </div>
        {:else if activeTab === "metadata"}
          <form class="metadata-form" on:submit|preventDefault={saveMetadata}>
            <div class="metadata-top">
              <div class="metadata-title-fields">
                <label>
                  <span>书名</span>
                  <input type="text" bind:value={metadataDraft.title} on:input={markMetadataDirty} autocomplete="off" />
                </label>
                <label>
                  <span>作者</span>
                  <input type="text" bind:value={metadataDraft.creator} on:input={markMetadataDirty} autocomplete="off" />
                </label>
              </div>
              <div class="metadata-cover" title="封面预览">
                {#if coverPreviewUrl}
                  <img src={coverPreviewUrl} alt="封面预览" />
                {:else}
                  <span>无封面</span>
                {/if}
              </div>
            </div>
            <label class="metadata-identifier">
              <span>标识符</span>
              <input type="text" bind:value={metadataDraft.identifier} on:input={markMetadataDirty} autocomplete="off" />
            </label>
            <label class="full">
              <span>简介</span>
              <textarea bind:value={metadataDraft.description} on:input={markMetadataDirty} rows="5"></textarea>
            </label>
            <div class="metadata-meta">
              <span title={doc.opfPath}>OPF：{doc.opfPath}</span>
              <span>{doc.files.length} 个文件，{textCount} 个可编辑，{imageCount} 张图片</span>
            </div>
            <div class="metadata-actions">
              <button type="button" on:click={resetMetadataDraft} disabled={!metadataDirty || busy}>还原</button>
              <button type="submit" class="primary" disabled={!metadataDirty || busy}>保存元数据</button>
            </div>
          </form>
        {/if}
        {/if}
      </aside>

      {#if !readerMode}
      <section class="editor-panel">
        <div class="editor-head">
          <div>
            <strong>{selectedFile?.path || selectedImage?.path || "未选择文件"}</strong>
          </div>
          <div class="editor-actions">
            <button type="button" class="primary" on:click={stageCurrentFile} disabled={!selectedFile || !dirty || busy}>暂存</button>
          </div>
        </div>

        {#if selectedFile}
          <div class="editor-content">
            <EpubCodeEditor
              bind:this={epubCodeEditorComponent}
              doc={editorText}
              language={getFileLanguage(selectedFile.kind)}
              onChange={handleEditorChange}
              onSave={stageCurrentFile}
              onOpenSearch={() => {
                setTimeout(() => document.getElementById("epub-search-input-fr")?.focus(), 50);
              }}
            />
          </div>
        {:else if selectedImage}
          <div class="editor-image-preview">
            {#if imagePreviewUrl}
              <img src={imagePreviewUrl} alt={selectedImage.name} />
            {:else}
              <span>正在读取图片...</span>
            {/if}
          </div>
        {:else}
          <div class="editor-empty">选择 XHTML、CSS、XML、文本或图片资源。</div>
        {/if}

        {#if selectedFile && selectedFile.editable}
          <div class="find-replace-panel">
            <div class="fr-row">
              <span class="fr-label">查找:</span>
              <div class="fr-input-wrapper">
                <input
                  id="epub-search-input-fr"
                  type="text"
                  class="fr-input"
                  bind:value={findPattern}
                  on:keydown={(e) => e.key === "Enter" && findNext()}
                  on:click={() => {
                    showFindHistory = !showFindHistory;
                    showReplaceHistory = false;
                  }}
                  title="显示查找历史"
                />
                {#if showFindHistory && findHistory.length > 0}
                  <div class="fr-history-dropdown">
                    {#each findHistory as item, i}
                      <div class="fr-history-item">
                        <span class="fr-history-text" on:click={() => selectFindHistory(item)} on:keydown={(e) => e.key === "Enter" && selectFindHistory(item)} role="button" tabindex="0">{item}</span>
                        <button type="button" class="fr-history-del" on:click|stopPropagation={() => removeFromFindHistory(i)} title="删除">&times;</button>
                      </div>
                    {/each}
                  </div>
                {/if}
              </div>
              <div class="fr-actions fr-find-actions">
                <button type="button" class="fr-btn fr-btn-text" on:click={countMatches} title="计数">计数</button>
                <button type="button" class="fr-btn fr-btn-text" on:click={findPrev} title="上一个">上一个</button>
                <button type="button" class="fr-btn fr-btn-text" on:click={findNext} title="下一个">下一个</button>
              </div>
            </div>
            <div class="fr-row">
              <span class="fr-label">替换:</span>
              <div class="fr-input-wrapper">
                <input
                  type="text"
                  class="fr-input"
                  bind:value={replacePattern}
                  on:focus={() => {
                    showFindHistory = false;
                    showReplaceHistory = false;
                  }}
                />
                <button
                  type="button"
                  class="fr-dropdown-btn"
                  on:click={() => {
                    showReplaceHistory = !showReplaceHistory;
                    showFindHistory = false;
                  }}
                  title="显示替换历史"
                >▾</button>
                {#if showReplaceHistory && replaceHistory.length > 0}
                  <div class="fr-history-dropdown">
                    {#each replaceHistory as item, i}
                      <div class="fr-history-item">
                        <span class="fr-history-text" on:click={() => selectReplaceHistory(item)} on:keydown={(e) => e.key === "Enter" && selectReplaceHistory(item)} role="button" tabindex="0">{item}</span>
                        <button type="button" class="fr-history-del" on:click|stopPropagation={() => removeFromReplaceHistory(i)} title="删除">&times;</button>
                      </div>
                    {/each}
                  </div>
                {/if}
              </div>
              <div class="fr-actions">
                <button type="button" class="fr-btn fr-btn-text" on:click={performReplace} title="替换">替换</button>
                <button type="button" class="fr-btn fr-btn-wide fr-danger" on:click={performReplaceAll} title="全部替换">全部替换</button>
              </div>
            </div>
            <div class="fr-row fr-options">
              <span class="fr-label">搜索选项:</span>
              <CustomSelect
                className="fr-custom-select fr-select-sm"
                value={isRegex ? "regex" : "text"}
                options={FIND_MODE_OPTIONS}
                on:change={(event) => (isRegex = event.detail === "regex")}
              />
              <CustomSelect
                className="fr-custom-select"
                value={searchScope}
                options={SEARCH_SCOPE_OPTIONS}
                on:change={(event) => (searchScope = event.detail as typeof searchScope)}
              />
              <CustomSelect
                className="fr-custom-select fr-select-xs"
                value={searchDirection}
                options={SEARCH_DIRECTION_OPTIONS}
                on:change={(event) => (searchDirection = event.detail as typeof searchDirection)}
              />
              <label class="fr-checkbox">
                <input type="checkbox" bind:checked={wrapAround} />循环
              </label>
              <label class="fr-checkbox">
                <input type="checkbox" bind:checked={textOnly} />纯文本
              </label>
              <span class="fr-message">{searchMessage}</span>
            </div>
          </div>
        {/if}
      </section>
      {/if}

      <aside class="right-panel">
        <section class="preview-pane">
          {#if readerMode}
            <div class="preview-header reader-preview-header">
              <div class="reader-preview-title">
                <strong>{currentPreviewTitle}</strong>
                <span>{previewNavLabel()}</span>
              </div>
              <div class="reader-preview-actions">
                <button type="button" class="reader-toc-toggle" class:active={!readerTocCollapsed} on:click={toggleReaderToc} aria-pressed={!readerTocCollapsed} title={readerTocCollapsed ? "显示目录" : "收起目录"}>目录</button>
                <button type="button" class="reader-font-btn" on:click={() => adjustReaderFont(-1)} disabled={!canAdjustReaderFont(-1) || busy} title="减小字号">A-</button>
                <button type="button" class="reader-font-btn" on:click={() => adjustReaderFont(1)} disabled={!canAdjustReaderFont(1) || busy} title="增大字号">A+</button>
                <button type="button" on:click={() => previewRelative(-1)} disabled={!canPreviewRelative(-1) || busy}>上一章</button>
                <button type="button" on:click={() => previewRelative(1)} disabled={!canPreviewRelative(1) || busy}>下一章</button>
              </div>
            </div>
          {:else}
            <div class="preview-header">
              <button type="button" class:active={rightTab === "preview"} on:click={() => (rightTab = "preview")}>预览</button>
              <button type="button" class:active={rightTab === "toc"} on:click={() => (rightTab = "toc")}>目录</button>
            </div>
          {/if}
          {#if readerMode || rightTab === "preview"}
            <div class="preview-body">
              {#if selectedFile?.kind === "xhtml" && previewHtml}
                <div class="mobile-frame" class:reader-frame={readerMode}>
                  <iframe class="chapter-preview" title={readerMode ? "章节阅读预览" : "章节预览"} srcdoc={previewHtml} sandbox="allow-same-origin allow-scripts"></iframe>
                </div>
              {:else if (!selectedFile && !selectedImage) || selectedFile?.kind === "xhtml"}
                <div class="preview-empty">{readerMode ? "选择目录中的章节后开始阅读。" : "选择 XHTML 章节后自动预览。"}</div>
              {/if}
            </div>
          {:else}
            <div class="toc-container">
              {#if tocTree.length === 0}
                <p class="panel-empty">未读取到 NAV / NCX 目录。</p>
              {:else}
                <div class="toc-list">
                  {#each tocTree as item (item.id)}
                    <TocNode {item} selectedSrc={selectedFile?.path || ""} onSelect={handleTocSelect} />
                  {/each}
                </div>
              {/if}
            </div>
          {/if}
        </section>
      </aside>
    </main>
  {/if}

  {#if fileMenu}
    <div
      class="file-menu-popover"
      style={`left: ${fileMenu.x}px; top: ${fileMenu.y}px;`}
      role="menu"
      tabindex="-1"
      on:click|stopPropagation
      on:keydown|stopPropagation
    >
      <button type="button" role="menuitem" on:click={() => renameResourcePath(fileMenu?.path || "")} disabled={busy}>重命名</button>
      <button type="button" role="menuitem" class="danger" on:click={() => deleteResourcePath(fileMenu?.path || "")} disabled={busy}>删除</button>
    </div>
  {/if}
</div>

<style>
  .web-epub-page {
    position: fixed;
    inset: 0;
    height: 100vh;
    --radius-sm: 6px;
    --font-code: Consolas, "Cascadia Mono", "Microsoft YaHei", monospace;
    --color-surface: #fffdf9;
    --color-surface-soft: #f8fafc;
    --color-text: #172033;
    --color-text-soft: #334155;
    --color-muted: #64748b;
    --color-border: #d8e0eb;
    --color-hover: #f1f5f9;
    --color-accent: #1677b8;
    --color-accent-deep: #155e96;
    --color-accent-soft: #e8f2f8;
    --shadow-xs: none;
    --transition-fast: 0.12s ease;
    background: #eef2f7;
    color: #172033;
    font-family: "Microsoft YaHei", "PingFang SC", system-ui, sans-serif;
    overflow: hidden;
  }

  .import-shell {
    height: 100%;
    min-height: 0;
    display: grid;
    grid-template-rows: minmax(0, 1fr);
    overflow: hidden;
  }

  .file-input {
    display: none;
  }

  h2,
  p {
    margin: 0;
  }

  h2 {
    font-size: 14px;
    line-height: 1.4;
  }

  button {
    height: 34px;
    padding: 0 12px;
    border: 1px solid #cbd5e1;
    border-radius: 6px;
    background: #ffffff;
    color: #172033;
    font: inherit;
    cursor: pointer;
  }

  button:hover:not(:disabled) {
    border-color: var(--color-accent);
  }

  button:disabled {
    cursor: not-allowed;
    opacity: 0.55;
  }

  button.primary {
    border-color: var(--color-accent);
    background: var(--color-accent);
    color: #ffffff;
    font-weight: 800;
  }

  button.danger {
    border-color: #fecaca;
    background: #fff1f2;
    color: #be123c;
    font-weight: 800;
  }

  button.danger:hover:not(:disabled) {
    border-color: #f43f5e;
  }

  button.active {
    border-color: var(--color-accent);
    background: var(--color-accent-soft);
    color: var(--color-accent-deep);
    font-weight: 800;
  }

  .workspace {
    height: 100vh;
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(240px, 280px) minmax(0, 1fr) minmax(320px, 380px);
    gap: 0;
    padding: 0;
    box-sizing: border-box;
    overflow: hidden;
  }

  .workspace.reader-workspace {
    grid-template-columns: minmax(260px, 340px) minmax(0, 1fr);
    background: #eef2f7;
  }

  .workspace.reader-workspace.reader-toc-collapsed {
    grid-template-columns: minmax(0, 1fr);
  }

  .left-panel,
  .right-panel,
  .editor-panel {
    min-height: 0;
    border: 0;
    border-radius: 0;
    background: #ffffff;
    overflow: hidden;
  }

  .left-panel {
    display: grid;
    grid-template-rows: auto auto auto minmax(0, 1fr);
    border-right: 1px solid #d8e0eb;
  }

  .reader-workspace .left-panel {
    grid-template-rows: auto auto minmax(0, 1fr);
  }

  .reader-workspace.reader-toc-collapsed .left-panel {
    display: none;
  }

  .right-panel {
    display: grid;
    grid-template-rows: minmax(0, 1fr);
    border-left: 1px solid #d8e0eb;
  }

  .reader-workspace .right-panel {
    border-left: 0;
  }

  .book-summary {
    display: grid;
    gap: 4px;
    min-height: 58px;
    padding: 10px 12px;
    border-bottom: 1px solid #e2e8f0;
    box-sizing: border-box;
    background: #f8fafc;
  }

  .book-summary strong {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .book-summary span,
  .book-summary small {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: #64748b;
    font-size: 12px;
  }

  .tree-header {
    height: 50px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 0 10px 0 12px;
    border-bottom: 1px solid #e2e8f0;
    background: #fafafa;
    box-sizing: border-box;
  }

  .tree-actions {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .tree-actions button {
    height: 26px;
    padding: 0 7px;
    font-size: 12px;
  }

  .tabs {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 0;
    padding: 0;
    border-bottom: 1px solid #e2e8f0;
  }

  .tabs button {
    height: 40px;
    border: 0;
    border-radius: 0;
    background: #f8fafc;
    font-weight: 700;
  }

  .tabs button.active {
    background: #ffffff;
    color: var(--color-accent);
    box-shadow: inset 0 -2px 0 var(--color-accent);
  }

  .tree-content,
  .toc-list {
    min-height: 0;
    max-height: none;
    display: grid;
    align-content: start;
    overflow: auto;
    padding: 8px;
  }

  .tree-content {
    padding: 8px;
    overflow-x: hidden;
  }

  .metadata-form {
    min-height: 0;
    overflow: auto;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
    padding: 12px;
  }

  .metadata-top {
    grid-column: 1 / -1;
    display: grid;
    grid-template-columns: minmax(0, 1fr) 96px;
    gap: 10px;
    align-items: start;
    min-width: 0;
  }

  .metadata-title-fields {
    display: grid;
    gap: 10px;
    min-width: 0;
  }

  .metadata-form label,
  .metadata-meta {
    min-width: 0;
    display: grid;
    gap: 5px;
  }

  .metadata-form label.full,
  .metadata-form label.metadata-identifier,
  .metadata-meta,
  .metadata-actions {
    grid-column: 1 / -1;
  }

  .metadata-form span {
    color: #64748b;
    font-size: 12px;
  }

  .metadata-form input,
  .metadata-form textarea {
    width: 100%;
    min-width: 0;
    box-sizing: border-box;
    border: 1px solid #cbd5e1;
    border-radius: 6px;
    background: #ffffff;
    color: #172033;
    font: inherit;
  }

  .metadata-form input {
    height: 34px;
    padding: 0 9px;
  }

  .metadata-form textarea {
    min-height: 98px;
    padding: 8px 9px;
    resize: vertical;
    line-height: 1.6;
  }

  .metadata-cover {
    width: 96px;
    height: 128px;
    aspect-ratio: 3 / 4;
    display: grid;
    place-items: center;
    border: 1px solid #cbd5e1;
    border-radius: 6px;
    background: #f8fafc;
    color: #64748b;
    font-size: 12px;
    overflow: hidden;
  }

  .metadata-cover img {
    width: 100%;
    height: 100%;
    object-fit: contain;
  }

  .metadata-meta {
    gap: 3px;
    overflow-wrap: anywhere;
  }

  .metadata-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .editor-panel {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr) auto;
    border-left: 1px solid #eef2f7;
    border-right: 1px solid #eef2f7;
  }

  .editor-head {
    height: 50px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 12px;
    border-bottom: 1px solid #e2e8f0;
    box-sizing: border-box;
  }

  .editor-head div {
    min-width: 0;
    display: grid;
    gap: 0;
  }

  .editor-head strong {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .editor-actions {
    flex: 0 0 auto;
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .editor-actions button {
    height: 28px;
    padding: 0 9px;
  }

  .editor-content {
    width: 100%;
    height: 100%;
    min-height: 0;
    box-sizing: border-box;
    background: #fffdf9;
    overflow: hidden;
  }

  .editor-content :global(.epub-code-editor) {
    height: 100%;
  }

  .editor-content :global(.cm-editor) {
    height: 100%;
    background: #fffdf9;
  }

  .editor-content :global(.cm-scroller) {
    font-family: var(--font-code);
  }

  .editor-content :global(.cm-content) {
    padding: 18px 20px;
    color: #172033;
  }

  .find-replace-panel {
    min-height: 0;
    background: #f0f0f0;
    border-top: 1px solid #cbd5e1;
    padding: 6px 10px;
    font-size: 13px;
    flex-shrink: 0;
    overflow: visible;
    box-sizing: border-box;
  }

  .fr-row {
    display: flex;
    align-items: center;
    gap: 4px;
    margin-bottom: 6px;
    overflow: visible;
  }

  .fr-row:last-child {
    margin-bottom: 0;
  }

  .fr-label {
    flex: 0 0 64px;
    font-weight: 600;
    color: #555;
    text-align: right;
    white-space: nowrap;
  }

  .fr-input-wrapper {
    flex: 1;
    min-width: 0;
    display: flex;
    position: relative;
  }

  .fr-input {
    flex: 1;
    min-width: 0;
    height: 30px;
    padding: 5px 10px;
    border: 1px solid #cbd5e1;
    border-radius: 4px;
    background: #ffffff;
    color: #172033;
    font: 13px Consolas, "Cascadia Mono", "Microsoft YaHei", monospace;
    box-sizing: border-box;
  }

  .fr-input:focus {
    outline: none;
    border-color: #2196f3;
    box-shadow: 0 0 0 2px rgba(33, 150, 243, 0.2);
  }

  .fr-input-wrapper .fr-input {
    border-top-right-radius: 0;
    border-bottom-right-radius: 0;
  }

  .fr-actions {
    width: 164px;
    display: flex;
    justify-content: flex-end;
    gap: 4px;
    flex-shrink: 0;
  }

  .fr-find-actions {
    width: 164px;
  }

  .fr-btn,
  .fr-dropdown-btn {
    height: 30px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px solid #bbb;
    border-radius: 4px;
    background: linear-gradient(to bottom, #fff, #e8e8e8);
    color: #172033;
    cursor: pointer;
    font-size: 12px;
    white-space: nowrap;
  }

  .fr-btn {
    padding: 0 8px;
  }

  .fr-btn-text {
    min-width: 44px;
  }

  .fr-btn-wide {
    min-width: 72px;
  }

  .fr-btn:hover,
  .fr-dropdown-btn:hover {
    background: linear-gradient(to bottom, #f5f5f5, #ddd);
    border-color: #999;
  }

  .fr-danger {
    border-color: #d99;
    background: linear-gradient(to bottom, #fee, #fcc);
    color: #be123c;
  }

  .fr-danger:hover {
    border-color: #c66;
    background: linear-gradient(to bottom, #fdd, #faa);
  }

  .fr-dropdown-btn {
    width: 30px;
    flex: 0 0 30px;
    border-left: 0;
    border-top-left-radius: 0;
    border-bottom-left-radius: 0;
    padding: 0;
  }

  .fr-options {
    font-size: 12px;
  }

  .fr-options :global(.fr-custom-select) {
    flex: 0 0 104px;
    width: 104px;
  }

  .fr-options :global(.fr-custom-select.fr-select-sm) {
    flex-basis: 96px;
    width: 96px;
  }

  .fr-options :global(.fr-custom-select.fr-select-xs) {
    flex-basis: 70px;
    width: 70px;
  }

  .fr-options :global(.fr-custom-select .custom-select-trigger) {
    height: 30px;
    min-height: 30px;
    padding: 0 8px;
    border-color: #cbd5e1;
    border-radius: 4px;
    background: #ffffff;
    color: #172033;
    box-shadow: none;
    font-size: 12px;
    font-weight: 600;
  }

  .fr-options :global(.fr-custom-select.open .custom-select-trigger),
  .fr-options :global(.fr-custom-select .custom-select-trigger:focus-visible) {
    border-color: #2196f3;
    box-shadow: 0 0 0 2px rgba(33, 150, 243, 0.2);
  }

  .fr-options :global(.fr-custom-select .custom-select-menu) {
    top: auto;
    bottom: calc(100% + 4px);
    border-color: #cbd5e1;
    border-radius: 4px;
    background: #ffffff;
    box-shadow: 0 4px 12px rgba(15, 23, 42, 0.14);
  }

  .fr-options :global(.fr-custom-select .custom-select-menu button) {
    min-height: 28px;
    padding: 5px 8px;
    border-radius: 3px;
    font-size: 12px;
    font-weight: 600;
  }

  .fr-checkbox {
    display: flex;
    align-items: center;
    gap: 4px;
    color: #555;
    cursor: pointer;
    white-space: nowrap;
  }

  .fr-checkbox input {
    margin: 0;
  }

  .fr-message {
    margin-left: auto;
    min-width: 84px;
    color: #1677b8;
    font-weight: 600;
    text-align: right;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .fr-history-dropdown {
    position: absolute;
    bottom: 100%;
    left: 0;
    right: 0;
    z-index: 20;
    max-height: 280px;
    margin-bottom: 2px;
    overflow-y: auto;
    border: 1px solid #cbd5e1;
    border-radius: 4px;
    background: #ffffff;
    box-shadow: 0 4px 8px rgba(15, 23, 42, 0.15);
  }

  .fr-history-item {
    display: flex;
    align-items: center;
    padding: 6px 10px;
    border-bottom: 1px solid #edf1f6;
  }

  .fr-history-item:last-child {
    border-bottom: 0;
  }

  .fr-history-item:hover {
    background: #f5f5f5;
  }

  .fr-history-text {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    cursor: pointer;
    font-family: Consolas, "Cascadia Mono", monospace;
    font-size: 12px;
  }

  .fr-history-del {
    width: 24px;
    height: 24px;
    min-height: 0;
    margin-left: 8px;
    padding: 0;
    border: 0;
    background: transparent;
    color: #999;
  }

  .fr-history-del:hover {
    background: #fdd;
    color: #d32f2f;
  }

  .editor-image-preview {
    min-height: 0;
    display: grid;
    place-items: center;
    padding: 18px;
    background: #f8fafc;
    box-sizing: border-box;
    overflow: auto;
  }

  .editor-image-preview img {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
    border: 1px solid #e2e8f0;
    background: #ffffff;
  }

  .editor-image-preview span {
    color: #64748b;
    font-size: 13px;
  }

  .preview-pane {
    min-height: 0;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    background: #f1f5f9;
  }

  .preview-header {
    position: relative;
    height: 50px;
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 0;
    padding: 0;
    border-bottom: 1px solid #d8e0eb;
    background: #f8fafc;
    box-sizing: border-box;
  }

  .preview-header button {
    height: 50px;
    border: 0;
    border-radius: 0;
    background: transparent;
    color: #64748b;
    font-weight: 800;
  }

  .preview-header button.active {
    color: var(--color-accent);
    background: #ffffff;
    box-shadow: inset 0 -2px 0 var(--color-accent);
  }

  .reader-preview-header {
    height: 56px;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 12px;
    padding: 0 14px 0 18px;
  }

  .reader-preview-title {
    min-width: 0;
    display: grid;
    gap: 2px;
  }

  .reader-preview-title strong {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: #172033;
    font-size: 15px;
  }

  .reader-preview-title span {
    color: #64748b;
    font-size: 12px;
    font-weight: 700;
  }

  .reader-preview-actions {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    flex-wrap: wrap;
    gap: 8px;
  }

  .reader-preview-actions button {
    height: 32px;
    padding: 0 12px;
    border: 1px solid #cbd5e1;
    border-radius: 4px;
    background: #ffffff;
    color: #334155;
    font-weight: 800;
  }

  .reader-preview-actions button:disabled {
    cursor: not-allowed;
    opacity: 0.45;
  }

  .reader-preview-actions .reader-font-btn {
    min-width: 38px;
    padding: 0 9px;
    font-size: 14px;
  }

  .reader-preview-actions .reader-toc-toggle {
    min-width: 54px;
    padding: 0 10px;
  }

  .reader-preview-actions .reader-toc-toggle.active {
    border-color: #334155;
    background: #172033;
    color: #ffffff;
  }

  .preview-body {
    min-height: 0;
    display: flex;
    align-items: stretch;
    justify-content: center;
    padding: 10px;
    background: #f0f0f0;
    overflow: hidden;
  }

  .reader-workspace .preview-body {
    padding: 18px clamp(16px, 3vw, 42px);
    background: #eef2f7;
  }

  .mobile-frame,
  .preview-empty {
    width: min(100%, 340px);
    height: 100%;
    min-height: 0;
    box-sizing: border-box;
  }

  .mobile-frame {
    display: flex;
    flex-direction: column;
    border: 1px solid #d8e0eb;
    background: #ffffff;
    box-shadow: 0 4px 10px rgba(15, 23, 42, 0.12);
    overflow: hidden;
  }

  .reader-frame {
    width: min(100%, 1080px);
    border-color: #cbd5e1;
    box-shadow: 0 10px 24px rgba(15, 23, 42, 0.14);
  }

  .chapter-preview {
    width: 100%;
    height: 100%;
    min-height: 0;
    border: 0;
    box-sizing: border-box;
    background: #ffffff;
  }

  .preview-empty {
    display: grid;
    place-items: center;
    border: 1px dashed #cbd5e1;
    background: #ffffff;
    color: #64748b;
    font-size: 13px;
  }

  .editor-empty {
    display: grid;
    place-items: center;
    color: #64748b;
  }

  .right-panel {
    box-sizing: border-box;
    overflow: hidden;
  }

  .panel-empty {
    color: #64748b;
    font-size: 13px;
    line-height: 1.6;
  }

  .toc-container {
    min-height: 0;
    background: #ffffff;
    overflow: auto;
  }

  .toc-list {
    max-height: none;
    padding: 0;
    border: 1px solid #e2e8f0;
    border-radius: 6px;
  }

  .toc-list {
    border: 0;
    border-radius: 0;
  }

  .toc-container .panel-empty {
    padding: 12px;
  }

  .file-menu-popover {
    position: fixed;
    z-index: 30;
    width: 104px;
    display: grid;
    gap: 4px;
    padding: 6px;
    border: 1px solid #cbd5e1;
    border-radius: 6px;
    background: #ffffff;
    box-shadow: 0 8px 24px rgba(15, 23, 42, 0.16);
  }

  .file-menu-popover button {
    width: 100%;
    height: 30px;
    padding: 0 8px;
    text-align: left;
  }

  @media (max-width: 760px) {
    .editor-head {
      align-items: stretch;
      flex-direction: column;
    }

    .editor-actions {
      width: 100%;
      justify-content: stretch;
    }

    .workspace {
      grid-template-columns: 1fr;
      grid-template-rows: 220px minmax(320px, 1fr) 360px;
    }

    .workspace.reader-workspace {
      grid-template-columns: 1fr;
      grid-template-rows: minmax(220px, 34vh) minmax(420px, 1fr);
    }

    .workspace.reader-workspace.reader-toc-collapsed {
      grid-template-rows: minmax(420px, 1fr);
    }

    .reader-preview-header {
      height: auto;
      min-height: 56px;
      grid-template-columns: minmax(0, 1fr);
      align-content: center;
      padding: 10px 12px;
    }

    .reader-preview-actions {
      justify-content: flex-start;
    }

    .left-panel,
    .editor-panel,
    .right-panel {
      border: 0;
      border-bottom: 1px solid #d8e0eb;
    }
  }

  :global(:root[data-tepub-client="web-mobile"]) .web-epub-page {
    height: 100dvh;
  }

  :global(:root[data-tepub-client="web-mobile"]) .import-shell {
    height: 100dvh;
    grid-template-rows: minmax(0, 1fr);
    background: #f4f5f8;
    overflow: auto;
  }

  :global(:root[data-tepub-client="web-mobile"]) .workspace {
    height: 100dvh;
    grid-template-columns: 1fr;
    grid-template-rows: minmax(180px, 24dvh) minmax(360px, 1fr) minmax(320px, 38dvh);
    overflow: auto;
  }

  :global(:root[data-tepub-client="web-mobile"]) .workspace.reader-workspace {
    grid-template-rows: minmax(220px, 34dvh) minmax(420px, 1fr);
  }

  :global(:root[data-tepub-client="web-mobile"]) .workspace.reader-workspace.reader-toc-collapsed {
    grid-template-rows: minmax(420px, 1fr);
  }

  :global(:root[data-tepub-client="web-mobile"]) .left-panel,
  :global(:root[data-tepub-client="web-mobile"]) .editor-panel,
  :global(:root[data-tepub-client="web-mobile"]) .right-panel {
    border: 0;
    border-bottom: 1px solid #d8e0eb;
  }

  :global(:root[data-tepub-client="web-mobile"]) .editor-head {
    height: auto;
    min-height: 50px;
    align-items: stretch;
    flex-direction: column;
  }

  :global(:root[data-tepub-client="web-mobile"]) .editor-actions {
    width: 100%;
    flex-wrap: wrap;
  }

  :global(:root[data-tepub-client="web-mobile"]) .editor-actions button {
    flex: 1 1 auto;
  }

  :global(:root[data-tepub-client="web-mobile"]) .metadata-form {
    grid-template-columns: 1fr;
  }

  :global(:root[data-tepub-client="web-mobile"]) .metadata-top {
    grid-template-columns: minmax(0, 1fr) 86px;
  }

  :global(:root[data-tepub-client="web-mobile"]) .metadata-cover {
    width: 86px;
    height: 116px;
  }

  :global(:root[data-tepub-client="web-mobile"]) .find-replace-panel {
    overflow: auto;
  }

  :global(:root[data-tepub-client="web-mobile"]) .fr-row {
    flex-wrap: wrap;
  }

  :global(:root[data-tepub-client="web-mobile"]) .fr-label {
    flex-basis: 100%;
    text-align: left;
  }

  :global(:root[data-tepub-client="web-mobile"]) .fr-actions,
  :global(:root[data-tepub-client="web-mobile"]) .fr-find-actions {
    width: auto;
    flex: 1 1 160px;
  }
</style>
