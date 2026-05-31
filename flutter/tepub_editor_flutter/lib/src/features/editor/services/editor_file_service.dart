import 'dart:convert';
import 'dart:io';

import 'package:file_picker/file_picker.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:path/path.dart' as p;

import '../../../core/storage/app_storage_paths.dart';
import '../../library/models/library_book.dart';
import '../models/editor_document.dart';

final editorFileServiceProvider =
    FutureProvider<EditorFileService>((ref) async {
  final paths = await ref.watch(appStoragePathsProvider.future);
  return EditorFileService(paths: paths);
});

class EditorFileService {
  const EditorFileService({this.paths});

  final AppStoragePaths? paths;

  Future<EditorDocument?> pickTextDocument() async {
    final result = await FilePicker.platform.pickFiles(
      allowMultiple: false,
      type: FileType.custom,
      allowedExtensions: const ['txt'],
    );
    final path = result?.files.single.path;
    if (path == null) {
      return null;
    }

    return readTextDocument(path, title: p.basenameWithoutExtension(path));
  }

  Future<EditorDocument> readTextDocument(
    String path, {
    required String title,
  }) async {
    final content = await _readTextFile(path);
    final toc = EditorTocState.empty;
    final chapters = scanChapters(content, toc: toc);
    final history = await listHistorySnapshots(title: title, filePath: path);

    return EditorDocument(
      title: title,
      content: content,
      chapters: chapters,
      selectedChapterId: chapters.isEmpty ? null : chapters.first.id,
      dirty: false,
      filePath: path,
      search: EditorSearchState.empty,
      toc: toc,
      history: history,
      display: EditorDisplaySettings.defaults,
      checkReport: EditorCheckReport.empty,
    );
  }

  Future<String?> saveDocument(EditorDocument document) async {
    var path = document.filePath;
    path ??= await FilePicker.platform.saveFile(
      dialogTitle: '保存 TXT 文件',
      fileName: '${document.title}.txt',
      type: FileType.custom,
      allowedExtensions: const ['txt'],
    );

    if (path == null) {
      return null;
    }

    await File(path).writeAsString(document.content);
    return path;
  }

  Future<EditorDocument> openLibraryBook(LibraryBook book) async {
    final path = book.sourcePath;
    if (path == null || !await File(path).exists()) {
      return EditorDocument(
        title: book.title,
        content: '《${book.title}》\n\n这本书还没有可读取的本地 TXT 路径。',
        chapters: const [
          EditorChapter(
            id: 'book-placeholder',
            title: '全文',
            preview: '等待文件内容迁移',
            offset: 0,
            lineNumber: 1,
          ),
        ],
        selectedChapterId: 'book-placeholder',
        dirty: false,
        filePath: path,
        search: EditorSearchState.empty,
        toc: EditorTocState.empty,
        history: const [],
        display: EditorDisplaySettings.defaults,
        checkReport: EditorCheckReport.empty,
      );
    }

    return readTextDocument(path, title: book.title);
  }

  Future<List<EditorHistorySnapshot>> listHistorySnapshots({
    required String title,
    String? filePath,
  }) async {
    final historyDir = await _historyDir();
    if (historyDir == null || !await historyDir.exists()) {
      return const [];
    }

    final prefix = _historyKey(title: title, filePath: filePath);
    final snapshots = <EditorHistorySnapshot>[];
    await for (final entity in historyDir.list()) {
      if (entity is! File || !p.basename(entity.path).startsWith(prefix)) {
        continue;
      }
      try {
        final json =
            jsonDecode(await entity.readAsString()) as Map<String, Object?>;
        snapshots.add(EditorHistorySnapshot.fromJson(json));
      } catch (_) {
        // Ignore corrupt snapshots; they should not block editor startup.
      }
    }

    snapshots.sort((a, b) => b.createdAt.compareTo(a.createdAt));
    return snapshots;
  }

  Future<List<EditorHistorySnapshot>> createHistorySnapshot(
    EditorDocument document,
  ) async {
    final historyDir = await _historyDir();
    if (historyDir == null) {
      return document.history;
    }

    final now = DateTime.now();
    final prefix = _historyKey(
      title: document.title,
      filePath: document.filePath,
    );
    final id = '${prefix}_${now.microsecondsSinceEpoch}';
    final snapshot = EditorHistorySnapshot(
      id: id,
      title: '${document.title} ${_formatSnapshotTime(now)}',
      createdAt: now,
      contentLength: document.content.length,
      filePath: document.filePath,
    );

    await File(p.join(historyDir.path, '$id.json')).writeAsString(
      const JsonEncoder.withIndent('  ').convert({
        ...snapshot.toJson(),
        'content': document.content,
      }),
    );

    return listHistorySnapshots(
      title: document.title,
      filePath: document.filePath,
    );
  }

  Future<String?> readHistorySnapshot(EditorHistorySnapshot snapshot) async {
    final historyDir = await _historyDir();
    if (historyDir == null) {
      return null;
    }

    final file = File(p.join(historyDir.path, '${snapshot.id}.json'));
    if (!await file.exists()) {
      return null;
    }

    final json = jsonDecode(await file.readAsString()) as Map<String, Object?>;
    return json['content'] as String?;
  }

  List<EditorChapter> scanChapters(
    String content, {
    EditorTocState toc = EditorTocState.empty,
  }) {
    final pattern = _compileTocPattern(toc);
    final lines = _normalizeLineEndings(content).split('\n');
    final chapters = <EditorChapter>[];
    var offset = 0;

    for (var index = 0; index < lines.length; index += 1) {
      final rawLine = lines[index];
      final line = rawLine.trim();
      final match = line.isEmpty ? null : pattern.firstMatch(line);
      if (match != null && _isLikelyTocTitle(line, toc)) {
        final title = _titleFromMatch(match, line);
        chapters.add(
          EditorChapter(
            id: 'chapter-${chapters.length + 1}',
            title: title,
            preview: _nextPreview(lines, index + 1),
            offset: offset + rawLine.indexOf(rawLine.trimLeft()),
            lineNumber: index + 1,
            level: _chapterLevel(title),
          ),
        );
      }
      offset += rawLine.length + 1;
    }

    if (chapters.isEmpty && content.trim().isNotEmpty) {
      chapters.add(
        const EditorChapter(
          id: 'chapter-1',
          title: '全文',
          preview: '未扫描到章节标题',
          offset: 0,
          lineNumber: 1,
        ),
      );
    }

    return chapters;
  }

  String? validateTocPattern(String pattern) {
    try {
      RegExp(pattern, multiLine: true);
      return null;
    } on FormatException catch (error) {
      return error.message;
    }
  }

  Future<String> _readTextFile(String path) async {
    final bytes = await File(path).readAsBytes();
    if (bytes.length >= 3 &&
        bytes[0] == 0xEF &&
        bytes[1] == 0xBB &&
        bytes[2] == 0xBF) {
      return _normalizeTextContent(
        utf8.decode(bytes.sublist(3), allowMalformed: true),
      );
    }
    return _normalizeTextContent(utf8.decode(bytes, allowMalformed: true));
  }

  String _normalizeTextContent(String content) {
    return _normalizeLineEndings(content).replaceAll('\t', '    ');
  }

  String _normalizeLineEndings(String content) {
    return content
        .replaceAll('\r\n', '\n')
        .replaceAll('\r', '\n')
        .replaceAll('\u2028', '\n')
        .replaceAll('\u2029', '\n');
  }

  RegExp _compileTocPattern(EditorTocState toc) {
    final source =
        toc.useCustomPattern ? toc.pattern : EditorTocState.defaultPattern;
    return RegExp(source, multiLine: true);
  }

  String _titleFromMatch(RegExpMatch match, String line) {
    if (match.groupCount >= 1) {
      final captured = match.group(1)?.trim();
      if (captured != null && captured.isNotEmpty) {
        return captured;
      }
    }
    return line;
  }

  bool _isLikelyTocTitle(String title, EditorTocState toc) {
    if (toc.useCustomPattern) {
      return true;
    }

    final trimmed = title.trim();
    if (trimmed.isEmpty) {
      return false;
    }

    if (trimmed.length > 64) {
      return false;
    }

    return RegExp(
      r'^(?:第)?[0-9零〇一二两三四五六七八九十百千万]+(?:卷\s+\S.+|章\s+\S.+)$',
    ).hasMatch(trimmed);
  }

  int _chapterLevel(String title) {
    if (RegExp(r'^(?:第)?[0-9零〇一二两三四五六七八九十百千万]+卷').hasMatch(title)) {
      return 1;
    }
    return 2;
  }

  String _nextPreview(List<String> lines, int start) {
    for (var index = start; index < lines.length; index += 1) {
      final text = lines[index].trim();
      if (text.isNotEmpty) {
        return text;
      }
    }
    return '暂无预览';
  }

  Future<Directory?> _historyDir() async {
    final resolved = paths;
    if (resolved == null) {
      return null;
    }
    await resolved.ensureCreated();
    return resolved.history;
  }

  String _historyKey({required String title, String? filePath}) {
    final source = filePath?.trim().isNotEmpty == true ? filePath! : title;
    final bytes = utf8.encode(source);
    var hash = 0x811c9dc5;
    for (final byte in bytes) {
      hash ^= byte;
      hash = (hash * 0x01000193) & 0xffffffff;
    }
    return 'editor_${hash.toRadixString(16)}';
  }

  String _formatSnapshotTime(DateTime time) {
    String two(int value) => value.toString().padLeft(2, '0');
    return '${time.year}-${two(time.month)}-${two(time.day)} '
        '${two(time.hour)}:${two(time.minute)}';
  }
}
