const SHARED_ADVANCED = new Set([
  "epub-to-txt", "epub-version", "epub-chinese", "epub-ad-clean", "epub-phonetic",
  "epub-footnote", "image-compress", "image-watermark", "epub-merge", "epub-split",
]);
const PROCESS_TOOLS = new Set(["file-encrypt", "file-decrypt", "epub-reformat", "image-convert"]);
const DIRECT_ROUTES: Record<string, string> = {
  library: "/library",
  "image-tools": "/toolbox/image-tools",
  "txt-epub": "/toolbox/make-epub",
  "epub-style-library": "/toolbox/epub-style-library",
  "txt-edit": "/toolbox/text-editor",
  "epub-edit": "/toolbox/epub-editor",
  "epub-read": "/toolbox/epub-editor?mode=reader",
  "epub-diagnose": "/toolbox/epub-diagnose",
};

export function isSharedWebTool(id: string) {
  return id === "image-convert" || id === "font-subset" || SHARED_ADVANCED.has(id);
}

export function isWebRouteToolId(id: string) {
  return Boolean(DIRECT_ROUTES[id])
    || id === "font-encrypt"
    || id === "font-decrypt"
    || PROCESS_TOOLS.has(id)
    || isSharedWebTool(id);
}

export function webToolRoute(id: string) {
  if (id === "send-to-kindle") return "https://www.amazon.com/sendtokindle";
  if (DIRECT_ROUTES[id]) return DIRECT_ROUTES[id];
  if (id === "font-encrypt" || id === "font-decrypt") return `/toolbox/font-process?tool=${id}`;
  if (id === "font-subset") return "/toolbox/font-process?tool=font-subset";
  if (SHARED_ADVANCED.has(id)) return `/toolbox/epub-advanced?tool=${id}`;
  if (PROCESS_TOOLS.has(id)) return `/toolbox/epub-process?tool=${id}`;
  return "#";
}
