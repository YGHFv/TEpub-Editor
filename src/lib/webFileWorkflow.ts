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

export function validateBrowserFiles(files: Iterable<File>, options: FileValidationOptions = {}): FileValidationResult {
  const source = Array.from(files);
  const extensions = (options.extensions || []).map((item) => item.toLowerCase().replace(/^\./, ""));
  const mimeTypes = (options.mimeTypes || []).map((item) => item.toLowerCase());
  const matches = (file: File) => {
    if (!extensions.length && !mimeTypes.length) return true;
    const extension = file.name.split(".").pop()?.toLowerCase() || "";
    return extensions.includes(extension) || mimeTypes.includes(file.type.toLowerCase());
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

export function downloadBrowserBlob(blob: Blob, fileName: string) {
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = fileName;
  anchor.click();
  queueMicrotask(() => URL.revokeObjectURL(url));
}
