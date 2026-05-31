import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/epub_editor_document.dart';
import '../services/epub_editor_service.dart';

final epubEditorControllerProvider =
    StateNotifierProvider<EpubEditorController, EpubEditorDocument>((ref) {
  return EpubEditorController(ref.read(epubEditorServiceProvider));
});

class EpubEditorController extends StateNotifier<EpubEditorDocument> {
  EpubEditorController(this._service) : super(EpubEditorDocument.empty());

  final EpubEditorService _service;

  Future<void> pickAndOpen() async {
    final path = await _service.pickEpub();
    if (path == null) {
      return;
    }
    await open(path);
  }

  Future<void> open(String path) async {
    state = state.copyWith(busy: true, clearError: true);
    try {
      state = await _service.openEpub(path);
    } catch (error) {
      state = state.copyWith(
        busy: false,
        error: '$error',
        status: '打开 EPUB 失败。',
      );
    }
  }

  Future<void> selectFile(String path) async {
    state = state.copyWith(selectedPath: path);
    final selected = state.selectedFile;
    if (selected != null &&
        selected.isText &&
        selected.content == null &&
        selected.previewContent != null) {
      state = state.copyWith(
        files: [
          for (final file in state.files)
            if (file.path == path)
              file.copyWith(content: selected.previewContent)
            else
              file,
        ],
        clearError: true,
      );
      return;
    }
    if (selected == null ||
        !selected.isText ||
        selected.modified ||
        selected.content != null ||
        selected.bytes != null ||
        !state.hasDocument) {
      return;
    }
    try {
      final loaded = await _service.loadFilePayload(
        epubPath: state.epubPath,
        file: selected,
      );
      if (state.selectedPath != path) {
        return;
      }
      state = state.copyWith(
        files: [
          for (final file in state.files)
            if (file.path == path) loaded else file,
        ],
        clearError: true,
      );
    } catch (error) {
      state = state.copyWith(error: '$error', status: '加载文件失败。');
    }
  }

  void selectSearchHit(EpubEditorSearchHit hit) {
    state = state.copyWith(
      selectedPath: hit.filePath,
      status: '已定位到 ${hit.filePath} @ ${hit.offset}',
    );
  }

  void updateSelectedContent(String content) {
    final selected = state.selectedFile;
    if (selected == null || !selected.isText) {
      return;
    }
    state = state.copyWith(
      files: [
        for (final file in state.files)
          if (file.path == selected.path)
            file.copyWith(
              content: content,
              size: content.length,
              modified: true,
            )
          else
            file,
      ],
      status: '已修改 ${selected.path}',
    );
  }

  void updateSearchQuery(String value) {
    state = state.copyWith(searchQuery: value);
  }

  void updateReplaceText(String value) {
    state = state.copyWith(replaceText: value);
  }

  void setSearchRegex(bool value) {
    state = state.copyWith(searchRegex: value);
  }

  void setSearchMatchCase(bool value) {
    state = state.copyWith(searchMatchCase: value);
  }

  void setSearchAllFiles(bool value) {
    state = state.copyWith(searchAllFiles: value);
  }

  void findNextInSelected() {
    final selected = state.selectedFile;
    final content = selected?.content;
    if (selected == null || content == null || state.searchQuery.isEmpty) {
      return;
    }
    try {
      final matcher = _searchMatcher();
      final found = matcher.firstMatch(content);
      state = state.copyWith(
        status: found == null
            ? '当前文件没有匹配项。'
            : '找到匹配：${selected.path} @ ${found.start}',
        clearError: true,
      );
    } catch (error) {
      state = state.copyWith(error: '$error', status: '查找失败。');
    }
  }

  void countMatches() {
    if (state.searchQuery.isEmpty) {
      return;
    }
    try {
      final matcher = _searchMatcher();
      final files = state.searchAllFiles
          ? state.visibleFiles.where((file) => file.isText)
          : [
              if (state.selectedFile?.isText == true) state.selectedFile!,
            ];
      var total = 0;
      var fileCount = 0;
      final hits = <EpubEditorSearchHit>[];
      for (final file in files) {
        final content = file.content ?? '';
        final matches = matcher.allMatches(content);
        final count = matches.length;
        if (count > 0) {
          fileCount += 1;
          total += count;
          for (final match in matches.take(8)) {
            hits.add(
              EpubEditorSearchHit(
                filePath: file.path,
                offset: match.start,
                preview: _previewAround(content, match.start, match.end),
              ),
            );
          }
        }
      }
      state = state.copyWith(
        status: state.searchAllFiles
            ? '共 $fileCount 个文件、$total 处匹配。'
            : '当前文件 $total 处匹配。',
        searchHits: hits.take(80).toList(),
        clearError: true,
      );
    } catch (error) {
      state = state.copyWith(error: '$error', status: '统计匹配失败。');
    }
  }

  void replaceAllInSelected() {
    if (state.searchAllFiles) {
      replaceAllInFiles();
      return;
    }
    final selected = state.selectedFile;
    final content = selected?.content;
    if (selected == null || content == null || state.searchQuery.isEmpty) {
      return;
    }
    try {
      final matcher = _searchMatcher();
      var count = 0;
      final nextContent = content.replaceAllMapped(matcher, (match) {
        count += 1;
        return state.replaceText;
      });
      if (count == 0) {
        state = state.copyWith(status: '当前文件没有可替换内容。');
        return;
      }
      state = state.copyWith(
        files: [
          for (final file in state.files)
            if (file.path == selected.path)
              file.copyWith(
                content: nextContent,
                size: nextContent.length,
                modified: true,
              )
            else
              file,
        ],
        status: '已替换 $count 处：${selected.path}',
        clearError: true,
      );
    } catch (error) {
      state = state.copyWith(error: '$error', status: '替换失败。');
    }
  }

  void replaceAllInFiles() {
    if (state.searchQuery.isEmpty) {
      return;
    }
    try {
      final matcher = _searchMatcher();
      var total = 0;
      var touchedFiles = 0;
      final files = [
        for (final file in state.files)
          if (!file.deleted && file.isText)
            _replaceInFile(file, matcher, onCount: (count) {
              if (count > 0) {
                touchedFiles += 1;
                total += count;
              }
            })
          else
            file,
      ];
      state = state.copyWith(
        files: files,
        status: total == 0 ? '没有可替换内容。' : '已跨文件替换 $touchedFiles 个文件、$total 处。',
        clearError: true,
      );
    } catch (error) {
      state = state.copyWith(error: '$error', status: '跨文件替换失败。');
    }
  }

  Future<void> save() async {
    state = state.copyWith(busy: true, clearError: true);
    try {
      await _service.saveEpub(state);
      final reopened = await _service.openEpub(state.epubPath);
      state = reopened.copyWith(status: '已保存 EPUB：${state.epubPath}');
    } catch (error) {
      state = state.copyWith(
        busy: false,
        error: '$error',
        status: '保存 EPUB 失败。',
      );
    }
  }

  Future<void> reload() async {
    if (!state.hasDocument) {
      return;
    }
    await open(state.epubPath);
  }

  void updateMetadata(EpubEditorMetadata metadata) {
    if (metadata.opfPath.isEmpty) {
      state = state.copyWith(error: '没有找到 OPF 元数据文件。');
      return;
    }
    final opf = state.files.firstWhere(
      (file) => file.path == metadata.opfPath,
      orElse: () => const EpubEditorFile(
        path: '',
        size: 0,
        kind: EpubEditorFileKind.text,
      ),
    );
    if (opf.path.isEmpty || opf.content == null) {
      state = state.copyWith(error: 'OPF 文件不可编辑。');
      return;
    }
    final nextContent = _service.updateMetadataInOpf(opf.content!, metadata);
    state = state.copyWith(
      metadata: metadata,
      files: [
        for (final file in state.files)
          if (file.path == opf.path)
            file.copyWith(
              content: nextContent,
              size: nextContent.length,
              modified: true,
            )
          else
            file,
      ],
      selectedPath: opf.path,
      status: '元数据已写入 OPF，保存 EPUB 后生效。',
      clearError: true,
    );
  }

  Future<void> replaceCover() async {
    final metadata = state.metadata;
    if (metadata.coverPath.isEmpty) {
      state = state.copyWith(error: '没有在 OPF 中识别到封面资源路径。');
      return;
    }
    try {
      final result = await _service.pickImageFile();
      if (result == null) {
        return;
      }
      final file = await _service.importFile(
        targetPath: metadata.coverPath,
        result: result,
      );
      state = state.copyWith(
        files: [
          for (final item in state.files)
            if (item.path == metadata.coverPath) file else item,
        ],
        selectedPath: metadata.coverPath,
        status: '封面已替换为本地图片，保存 EPUB 后生效。',
        clearError: true,
      );
    } catch (error) {
      state = state.copyWith(error: '$error', status: '替换封面失败。');
    }
  }

  void renameSelected(String nextPath) {
    final selected = state.selectedFile;
    if (selected == null) {
      return;
    }
    renameFile(selected.path, nextPath);
  }

  void renameFile(String currentPath, String nextPath) {
    final selected = state.files
        .where((file) => !file.deleted && file.path == currentPath)
        .firstOrNull;
    if (selected == null || nextPath.trim().isEmpty) {
      return;
    }
    final normalized = nextPath.replaceAll('\\', '/').trim();
    if (state.files.any(
      (file) => !file.deleted && file.path == normalized && file != selected,
    )) {
      state = state.copyWith(error: '目标路径已存在：$normalized');
      return;
    }
    state = state.copyWith(
      selectedPath: normalized,
      files: [
        for (final file in state.files)
          if (file.path == selected.path)
            file.copyWith(path: normalized, modified: true)
          else
            file,
      ],
      status: '已重命名为 $normalized',
    );
  }

  void deleteSelected() {
    final selected = state.selectedFile;
    if (selected == null) {
      return;
    }
    deleteFile(selected.path);
  }

  void deleteFile(String path) {
    final selected = state.files
        .where((file) => !file.deleted && file.path == path)
        .firstOrNull;
    if (selected == null || selected.path == 'mimetype') {
      return;
    }
    final files = [
      for (final file in state.files)
        if (file.path == selected.path)
          file.copyWith(deleted: true, modified: true)
        else
          file,
    ];
    final nextSelected = files
        .where((file) => !file.deleted)
        .map((file) => file.path)
        .firstOrNull;
    state = state.copyWith(
      files: files,
      selectedPath: nextSelected,
      status: '已标记删除 ${selected.path}，保存后生效。',
    );
  }

  void createTextFile(String path) {
    if (path.trim().isEmpty) {
      return;
    }
    final file = _service.newTextFile(path);
    if (state.files.any((item) => !item.deleted && item.path == file.path)) {
      state = state.copyWith(error: '文件已存在：${file.path}');
      return;
    }
    state = state.copyWith(
      files: [...state.files, file]..sort((a, b) => a.path.compareTo(b.path)),
      selectedPath: file.path,
      status: '已创建 ${file.path}，保存后写入 EPUB。',
    );
  }

  Future<void> importFile(String targetPath) async {
    try {
      final result = await _service.pickAnyFile();
      if (result == null) {
        return;
      }
      final file = await _service.importFile(
        targetPath: targetPath,
        result: result,
      );
      final files = state.files
          .where((item) => item.path != file.path)
          .toList(growable: true)
        ..add(file)
        ..sort((a, b) => a.path.compareTo(b.path));
      state = state.copyWith(
        files: files,
        selectedPath: file.path,
        status: '已导入 ${file.path}，保存后写入 EPUB。',
        clearError: true,
      );
    } catch (error) {
      state = state.copyWith(error: '$error', status: '导入文件失败。');
    }
  }

  RegExp _searchMatcher() {
    final source = state.searchRegex
        ? state.searchQuery
        : RegExp.escape(state.searchQuery);
    return RegExp(
      source,
      multiLine: true,
      caseSensitive: state.searchMatchCase,
    );
  }

  EpubEditorFile _replaceInFile(
    EpubEditorFile file,
    RegExp matcher, {
    required void Function(int count) onCount,
  }) {
    var count = 0;
    final nextContent = (file.content ?? '').replaceAllMapped(matcher, (match) {
      count += 1;
      return state.replaceText;
    });
    onCount(count);
    return count == 0
        ? file
        : file.copyWith(
            content: nextContent,
            size: nextContent.length,
            modified: true,
          );
  }

  String _previewAround(String content, int start, int end) {
    final left = (start - 24).clamp(0, content.length);
    final right = (end + 42).clamp(0, content.length);
    return content
        .substring(left, right)
        .replaceAll(RegExp(r'\s+'), ' ')
        .trim();
  }
}

extension _FirstOrNull<T> on Iterable<T> {
  T? get firstOrNull {
    final iterator = this.iterator;
    return iterator.moveNext() ? iterator.current : null;
  }
}
