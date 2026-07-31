/**
 * Shared file-type icon mapping for the EPUB editor UI
 * (file tree, tabs bar). Pure function, no side effects.
 */
export function getFileIcon(type: string): string {
  switch (type) {
    case "folder":
      return "\u{1F4C1}";
    case "html":
      return "\u{1F4C4}";
    case "css":
      return "\u{1F3A8}";
    case "xml":
      return "⚙️";
    case "image":
      return "\u{1F5BC}️";
    case "font":
      return "\u{1F524}";
    default:
      return "\u{1F4C4}";
  }
}
