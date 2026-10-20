/**
 * Shared file-type icon mapping for the EPUB editor UI
 * (file tree, tabs bar). Pure function, no side effects.
 *
 * 返回统一图标集里的名字，而不是 emoji：emoji 由系统字体提供，
 * Windows / macOS / Linux 画出来粗细和配色都不一样，也没法跟随
 * 深色主题的 currentColor。
 */
import type { IconName } from "$lib/icons/Icon.svelte";

export function getFileIcon(type: string): IconName {
  switch (type) {
    case "folder":
      return "folder";
    case "css":
      return "file-css";
    case "xml":
      return "file-xml";
    case "image":
      return "image";
    case "font":
      return "font";
    case "html":
    default:
      return "file";
  }
}
