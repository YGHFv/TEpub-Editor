export function hasUnsavedTextChanges(hasLoadedText: boolean, content: string, savedContent: string) {
  return hasLoadedText && content !== savedContent;
}

export function hasUnsavedEpubChanges(readerMode: boolean, hasDocument: boolean, ...dirtyFlags: boolean[]) {
  return !readerMode && hasDocument && dirtyFlags.some(Boolean);
}

export function hasUnsavedMakeChanges(selectedPath: string, exportPath: string) {
  return Boolean(selectedPath) && !exportPath;
}
