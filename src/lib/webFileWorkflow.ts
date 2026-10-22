export type FileValidationOptions = {
  extensions?: string[];
  mimeTypes?: string[];
  multiple?: boolean;
};

export type FileValidationResult = {
  accepted: File[];
  rejected: File[];
  message: string;
};

export function validateBrowserFiles(
  files: Iterable<File>,
  options: FileValidationOptions = {},
): FileValidationResult {
  const source = Array.from(files);
  const extensions = (options.extensions || []).map((item) =>
    item.toLowerCase().replace(/^\./, ""),
  );
  const mimeTypes = (options.mimeTypes || []).map((item) => item.toLowerCase());
  const matches = (file: File) => {
    if (!extensions.length && !mimeTypes.length) return true;
    const extension = file.name.split(".").pop()?.toLowerCase() || "";
    return (
      extensions.includes(extension) ||
      mimeTypes.includes(file.type.toLowerCase())
    );
  };
  const accepted = source.filter(matches);
  const rejected = source.filter((file) => !matches(file));
  const limited = options.multiple === false ? accepted.slice(0, 1) : accepted;
  const overflow = accepted.slice(limited.length);
  return {
    accepted: limited,
    rejected: [...rejected, ...overflow],
    message: rejected.length
      ? `已忽略 ${rejected.length} 个不支持的文件。`
      : overflow.length
        ? "此工具一次只能处理一个文件。"
        : "",
  };
}

export class ObjectUrlRegistry {
  private urls = new Set<string>();

  create(blob: Blob) {
    const url = URL.createObjectURL(blob);
    this.urls.add(url);
    return url;
  }

  revoke(url: string | null | undefined) {
    if (!url || !this.urls.delete(url)) return;
    URL.revokeObjectURL(url);
  }

  clear() {
    for (const url of this.urls) URL.revokeObjectURL(url);
    this.urls.clear();
  }

  get size() {
    return this.urls.size;
  }
}

/**
 * 用一次隐藏的 <a download> 触发浏览器下载。
 *
 * 两个移动端细节，桌面浏览器都能容忍缺失，所以以前一直是"桌面能用就行"：
 * 1. 锚点必须挂进 DOM 再点 —— 游离节点上的 click() 在部分 WebView 里被忽略；
 * 2. 调用方不能紧接着回收 URL —— 移动端下载是异步接管的，URL 提前失效就是
 *    "点了导出没反应"。
 *
 * revoke 由调用方决定（blob URL 的所有权在调用方），这里只负责触发。
 */
export function triggerBrowserDownload(url: string, fileName: string) {
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = fileName;
  anchor.style.display = "none";
  document.body.appendChild(anchor);
  anchor.click();
  anchor.remove();
}

/** blob → 立即下载；URL 延后 30 秒回收，理由见 triggerBrowserDownload */
export function downloadBrowserBlob(blob: Blob, fileName: string) {
  const url = URL.createObjectURL(blob);
  triggerBrowserDownload(url, fileName);
  setTimeout(() => URL.revokeObjectURL(url), 30_000);
}
