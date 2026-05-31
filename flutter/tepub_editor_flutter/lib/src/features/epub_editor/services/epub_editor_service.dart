import 'dart:convert';
import 'dart:io';
import 'dart:math' as math;

import 'package:archive/archive.dart';
import 'package:file_picker/file_picker.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:path/path.dart' as p;

import '../models/epub_editor_document.dart';

final epubEditorServiceProvider = Provider<EpubEditorService>((ref) {
  return const EpubEditorService();
});

final epubPreviewResourceServerProvider =
    StateNotifierProvider<EpubPreviewResourceServer, Uri?>((ref) {
  final server = EpubPreviewResourceServer(ref.read(epubEditorServiceProvider));
  ref.onDispose(server.dispose);
  return server;
});

class EpubEditorService {
  const EpubEditorService();

  Future<String?> pickEpub() async {
    final result = await FilePicker.platform.pickFiles(
      allowMultiple: false,
      type: FileType.custom,
      allowedExtensions: const ['epub'],
    );
    return result?.files.single.path;
  }

  Future<EpubEditorDocument> openEpub(String path) async {
    final file = File(path);
    if (!await file.exists()) {
      throw StateError('文件不存在：$path');
    }

    final files = await compute(_decodeEpubFilesFromPath, path);

    return EpubEditorDocument(
      epubPath: path,
      files: files,
      selectedPath: null,
      status: '已打开 ${p.basename(path)}，共 ${files.length} 个文件。',
      busy: false,
      searchQuery: '',
      replaceText: '',
      searchRegex: false,
      searchMatchCase: false,
      searchAllFiles: false,
      searchHits: const [],
      metadata: parseMetadata(files),
    );
  }

  Future<void> saveEpub(EpubEditorDocument document) async {
    if (!document.hasDocument) {
      throw StateError('请先打开 EPUB');
    }
    final archive = Archive();
    final mimetype = document.files
        .where((file) => file.path == 'mimetype' && !file.deleted)
        .firstOrNull;
    if (mimetype != null) {
      archive.addFile(
        ArchiveFile.noCompress(
          'mimetype',
          'application/epub+zip'.length,
          utf8.encode('application/epub+zip'),
        ),
      );
    }

    for (final file in document.files) {
      if (file.deleted || file.path == 'mimetype') {
        continue;
      }
      final bytes = file.modified
          ? (file.isText ? utf8.encode(file.content ?? '') : file.bytes ?? [])
          : await _readOriginalBytes(document.epubPath, file.path);
      archive.addFile(ArchiveFile(file.path, bytes.length, bytes));
    }

    final encoded = ZipEncoder().encode(archive);
    if (encoded == null) {
      throw StateError('EPUB 压缩失败');
    }

    final target = File(document.epubPath);
    final backup = File('${document.epubPath}.bak.tmp');
    if (await backup.exists()) {
      await backup.delete();
    }
    if (await target.exists()) {
      await target.copy(backup.path);
    }
    try {
      await target.writeAsBytes(encoded);
      if (await backup.exists()) {
        await backup.delete();
      }
    } catch (_) {
      if (await backup.exists()) {
        await backup.copy(target.path);
      }
      rethrow;
    }
  }

  Future<EpubEditorFile> loadFilePayload({
    required String epubPath,
    required EpubEditorFile file,
  }) async {
    if (file.modified || file.content != null || file.bytes != null) {
      return file;
    }
    if (file.isText) {
      final content = await compute(
        _readSingleEpubTextFile,
        _SingleFileReadRequest(epubPath, file.path),
      );
      return file.copyWith(content: content);
    }
    final bytes = await compute(
      _readSingleEpubFile,
      _SingleFileReadRequest(epubPath, file.path),
    );
    return file.copyWith(bytes: bytes);
  }

  Future<List<int>> loadRawFileBytes({
    required String epubPath,
    required String filePath,
  }) {
    return compute(
      _readSingleEpubFile,
      _SingleFileReadRequest(epubPath, filePath),
    );
  }

  Future<EpubEditorFile> importFile({
    required String targetPath,
    required FilePickerResult result,
  }) async {
    final sourcePath = result.files.single.path;
    if (sourcePath == null) {
      throw StateError('未选择文件');
    }
    final source = File(sourcePath);
    var normalizedTarget = _normalizeZipPath(targetPath);
    if (normalizedTarget.isEmpty || normalizedTarget.endsWith('/')) {
      normalizedTarget = '$normalizedTarget${p.basename(sourcePath)}';
    }
    final bytes = await source.readAsBytes();
    final kind = _kindForPath(normalizedTarget);
    return EpubEditorFile(
      path: normalizedTarget,
      size: bytes.length,
      kind: kind,
      content: kind == EpubEditorFileKind.text
          ? utf8.decode(bytes, allowMalformed: true)
          : null,
      bytes: kind == EpubEditorFileKind.text ? null : bytes,
      modified: true,
    );
  }

  Future<FilePickerResult?> pickAnyFile() {
    return FilePicker.platform.pickFiles(allowMultiple: false);
  }

  Future<FilePickerResult?> pickImageFile() {
    return FilePicker.platform.pickFiles(
      allowMultiple: false,
      type: FileType.image,
    );
  }

  EpubEditorFile newTextFile(String path) {
    final normalized = _normalizeZipPath(path);
    return EpubEditorFile(
      path: normalized,
      size: 0,
      kind: _kindForPath(normalized),
      content: '',
      modified: true,
    );
  }

  EpubEditorMetadata parseMetadata(List<EpubEditorFile> files) {
    final opf = files.where((file) {
      return !file.deleted && file.path.toLowerCase().endsWith('.opf');
    }).firstOrNull;
    final content = opf?.content ?? opf?.previewContent ?? '';
    if (opf == null || content.isEmpty) {
      return EpubEditorMetadata.empty();
    }
    final coverId = _firstMetaContent(content, 'cover');
    var coverHref = '';
    if (coverId.isNotEmpty) {
      final manifestItem = RegExp(
        '<item[^>]+id=["\']${RegExp.escape(coverId)}["\'][^>]*>',
        caseSensitive: false,
      ).firstMatch(content)?.group(0);
      coverHref = _attr(manifestItem ?? '', 'href');
    }
    coverHref = coverHref.isEmpty ? _firstCoverHref(content) : coverHref;
    final coverPath = coverHref.isEmpty
        ? ''
        : p.posix.normalize(
            p.posix.join(p.posix.dirname(opf.path), coverHref),
          );
    return EpubEditorMetadata(
      opfPath: opf.path,
      title: _firstTagText(content, 'dc:title'),
      author: _firstTagText(content, 'dc:creator'),
      publisher: _firstTagText(content, 'dc:publisher'),
      description: _firstTagText(content, 'dc:description'),
      uuid: _firstTagText(content, 'dc:identifier'),
      subjects: [
        for (final match in RegExp(
          r'<dc:subject[^>]*>([\s\S]*?)</dc:subject>',
          caseSensitive: false,
        ).allMatches(content))
          _xmlUnescape(match.group(1) ?? '').trim(),
      ].where((item) => item.isNotEmpty).toList(),
      coverPath: coverPath,
    );
  }

  String updateMetadataInOpf(String opf, EpubEditorMetadata metadata) {
    var next = opf;
    next = _upsertTag(next, 'dc:title', metadata.title);
    next = _upsertTag(next, 'dc:creator', metadata.author);
    next = _upsertTag(next, 'dc:publisher', metadata.publisher);
    next = _upsertTag(next, 'dc:description', metadata.description);
    next = _upsertTag(next, 'dc:identifier', metadata.uuid, keepAttrs: true);
    next = _replaceSubjects(next, metadata.subjects);
    return next;
  }

  Future<List<int>> _readOriginalBytes(String epubPath, String filePath) async {
    final archive =
        ZipDecoder().decodeBytes(await File(epubPath).readAsBytes());
    for (final entry in archive.files) {
      if (entry.name == filePath) {
        return _entryBytes(entry);
      }
    }
    throw StateError('原 EPUB 中不存在文件：$filePath');
  }

  List<int> _entryBytes(ArchiveFile entry) {
    final content = entry.content;
    if (content is List<int>) {
      return content;
    }
    return content as List<int>;
  }

  EpubEditorFileKind _kindForPath(String path) {
    return switch (p.extension(path).toLowerCase()) {
      '.html' ||
      '.htm' ||
      '.xhtml' ||
      '.css' ||
      '.xml' ||
      '.opf' ||
      '.ncx' =>
        EpubEditorFileKind.text,
      '.txt' || '.md' || '.svg' || '.js' || '.json' => EpubEditorFileKind.text,
      '.png' ||
      '.jpg' ||
      '.jpeg' ||
      '.gif' ||
      '.webp' =>
        EpubEditorFileKind.image,
      '.ttf' || '.otf' || '.woff' || '.woff2' => EpubEditorFileKind.font,
      _ => EpubEditorFileKind.binary,
    };
  }

  String _firstTagText(String content, String tag) {
    final match = RegExp(
      '<$tag[^>]*>([\\s\\S]*?)</$tag>',
      caseSensitive: false,
    ).firstMatch(content);
    return _xmlUnescape(match?.group(1) ?? '').trim();
  }

  String _firstMetaContent(String content, String name) {
    final match = RegExp(
      '<meta[^>]+name=["\']${RegExp.escape(name)}["\'][^>]*>',
      caseSensitive: false,
    ).firstMatch(content);
    return _attr(match?.group(0) ?? '', 'content');
  }

  String _firstCoverHref(String content) {
    final match = RegExp(
      '''<item[^>]+(?:properties=["'][^"']*cover-image[^"']*["']|id=["']cover[^"']*["'])[^>]*>''',
      caseSensitive: false,
    ).firstMatch(content);
    return _attr(match?.group(0) ?? '', 'href');
  }

  String _attr(String tag, String name) {
    final match = RegExp(
      '$name=["\']([^"\']+)["\']',
      caseSensitive: false,
    ).firstMatch(tag);
    return _xmlUnescape(match?.group(1) ?? '').trim();
  }

  String _upsertTag(
    String content,
    String tag,
    String value, {
    bool keepAttrs = false,
  }) {
    final escaped = _xmlEscape(value.trim());
    final pattern = RegExp(
      '<$tag([^>]*)>[\\s\\S]*?</$tag>',
      caseSensitive: false,
    );
    final existing = pattern.firstMatch(content);
    if (existing != null) {
      final attrs = keepAttrs ? existing.group(1) ?? '' : '';
      return content.replaceRange(
        existing.start,
        existing.end,
        '<$tag$attrs>$escaped</$tag>',
      );
    }
    final metadataClose =
        RegExp(r'</metadata>', caseSensitive: false).firstMatch(content);
    if (metadataClose == null) {
      return content;
    }
    return content.replaceRange(
      metadataClose.start,
      metadataClose.start,
      '    <$tag>$escaped</$tag>\n',
    );
  }

  String _replaceSubjects(String content, List<String> subjects) {
    final withoutSubjects = content.replaceAll(
      RegExp(
        r'\s*<dc:subject[^>]*>[\s\S]*?</dc:subject>\s*',
        caseSensitive: false,
      ),
      '\n',
    );
    final metadataClose = RegExp(r'</metadata>', caseSensitive: false)
        .firstMatch(withoutSubjects);
    if (metadataClose == null || subjects.isEmpty) {
      return withoutSubjects;
    }
    final block = subjects
        .where((subject) => subject.trim().isNotEmpty)
        .map((subject) =>
            '    <dc:subject>${_xmlEscape(subject.trim())}</dc:subject>')
        .join('\n');
    return withoutSubjects.replaceRange(
      metadataClose.start,
      metadataClose.start,
      '$block\n',
    );
  }

  String _xmlUnescape(String value) {
    return value
        .replaceAll('&quot;', '"')
        .replaceAll('&apos;', "'")
        .replaceAll('&lt;', '<')
        .replaceAll('&gt;', '>')
        .replaceAll('&amp;', '&');
  }

  String _xmlEscape(String value) {
    return value
        .replaceAll('&', '&amp;')
        .replaceAll('<', '&lt;')
        .replaceAll('>', '&gt;')
        .replaceAll('"', '&quot;')
        .replaceAll("'", '&apos;');
  }

  String _normalizeZipPath(String path) {
    final raw = path.replaceAll('\\', '/').trim();
    final keepTrailingSlash = raw.endsWith('/');
    final parts = <String>[];
    for (final part in raw.split('/')) {
      final clean = part.trim();
      if (clean.isEmpty || clean == '.' || clean == '..') {
        continue;
      }
      parts.add(clean.replaceAll(RegExp(r'[<>:"|?*]+'), '_'));
    }
    final normalized = parts.join('/');
    return keepTrailingSlash && normalized.isNotEmpty
        ? '$normalized/'
        : normalized;
  }
}

class EpubPreviewResourceServer extends StateNotifier<Uri?> {
  EpubPreviewResourceServer(this._service) : super(null);

  final EpubEditorService _service;
  HttpServer? _server;
  String _epubPath = '';
  final Map<String, String> _pathLookup = {};

  Future<Uri?> ensureStarted({
    required String epubPath,
    required List<EpubEditorFile> files,
  }) async {
    if (_server != null && _epubPath == epubPath) {
      return state;
    }
    await stop();
    _epubPath = epubPath;
    _pathLookup
      ..clear()
      ..addEntries(files.map((file) {
        return MapEntry(
            _normalizeResourcePath(file.path).toLowerCase(), file.path);
      }));
    _server = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
    state = Uri.parse('http://127.0.0.1:${_server!.port}/');
    _server!.listen(_handleRequest);
    return state;
  }

  Future<void> stop() async {
    final server = _server;
    _server = null;
    state = null;
    _epubPath = '';
    _pathLookup.clear();
    await server?.close(force: true);
  }

  @override
  void dispose() {
    final server = _server;
    _server = null;
    _epubPath = '';
    _pathLookup.clear();
    server?.close(force: true);
    super.dispose();
  }

  Future<void> _handleRequest(HttpRequest request) async {
    try {
      final requested = Uri.decodeComponent(request.uri.path);
      final normalized = _normalizeResourcePath(requested);
      final zipPath = _pathLookup[normalized.toLowerCase()];
      if (zipPath == null || _epubPath.isEmpty) {
        request.response.statusCode = HttpStatus.notFound;
        await request.response.close();
        return;
      }
      final bytes = await _service.loadRawFileBytes(
        epubPath: _epubPath,
        filePath: zipPath,
      );
      request.response.headers.contentType =
          ContentType.parse(_mimeTypeForPath(zipPath));
      request.response.headers.set(
        HttpHeaders.cacheControlHeader,
        'max-age=3600',
      );
      request.response.add(bytes);
      await request.response.close();
    } catch (_) {
      request.response.statusCode = HttpStatus.internalServerError;
      await request.response.close();
    }
  }
}

String _normalizeResourcePath(String path) {
  final parts = <String>[];
  for (final part
      in path.replaceAll('\\', '/').replaceAll(RegExp(r'/+'), '/').split('/')) {
    if (part.isEmpty || part == '.') {
      continue;
    }
    if (part == '..') {
      if (parts.isNotEmpty) {
        parts.removeLast();
      }
    } else {
      parts.add(part);
    }
  }
  return parts.join('/');
}

String _mimeTypeForPath(String path) {
  final lower = path.toLowerCase();
  if (lower.endsWith('.jpg') || lower.endsWith('.jpeg')) {
    return 'image/jpeg';
  }
  if (lower.endsWith('.png')) {
    return 'image/png';
  }
  if (lower.endsWith('.gif')) {
    return 'image/gif';
  }
  if (lower.endsWith('.svg')) {
    return 'image/svg+xml';
  }
  if (lower.endsWith('.webp')) {
    return 'image/webp';
  }
  if (lower.endsWith('.css')) {
    return 'text/css; charset=utf-8';
  }
  if (lower.endsWith('.ttf')) {
    return 'font/ttf';
  }
  if (lower.endsWith('.otf')) {
    return 'font/otf';
  }
  if (lower.endsWith('.woff')) {
    return 'font/woff';
  }
  if (lower.endsWith('.woff2')) {
    return 'font/woff2';
  }
  return 'application/octet-stream';
}

extension _FirstOrNull<T> on Iterable<T> {
  T? get firstOrNull {
    final iterator = this.iterator;
    return iterator.moveNext() ? iterator.current : null;
  }
}

Future<List<EpubEditorFile>> _decodeEpubFilesFromPath(String path) async {
  final entries = await _scanZipCentralDirectory(path);
  final files = <EpubEditorFile>[];
  for (final entry in entries) {
    if (entry.name.endsWith('/')) {
      continue;
    }
    final kind = _epubKindForPath(entry.name);
    final loadText = kind == EpubEditorFileKind.text &&
        _shouldPreloadText(entry.name, entry.size);
    final loadPreview = kind == EpubEditorFileKind.text &&
        _shouldPreloadPreview(entry.name, entry.size);
    final loadBytes = !loadText &&
        !loadPreview &&
        _shouldPreloadResourceBytes(entry.name, entry.size);
    final entryBytes = loadText
        ? await _readSingleEpubFile(_SingleFileReadRequest(path, entry.name))
        : loadPreview || loadBytes
            ? await _readSingleEpubFile(
                _SingleFileReadRequest(path, entry.name))
            : null;
    final decoded = entryBytes == null || loadBytes
        ? null
        : utf8.decode(entryBytes, allowMalformed: true);
    files.add(
      EpubEditorFile(
        path: entry.name,
        size: entry.size,
        kind: kind,
        content: loadText ? decoded : null,
        previewContent: loadPreview ? decoded : null,
        titleHint:
            decoded == null ? null : _titleHintForFile(entry.name, decoded),
        bytes: loadBytes ? entryBytes : null,
      ),
    );
  }
  files.sort(
      (a, b) => _epubFileSortKey(a.path).compareTo(_epubFileSortKey(b.path)));
  return files;
}

bool _shouldPreloadText(String path, int size) {
  return false;
}

bool _shouldPreloadResourceBytes(String path, int size) {
  final lower = path.toLowerCase();
  if (size > 512 * 1024) {
    return false;
  }
  return lower.endsWith('.ttf') ||
      lower.endsWith('.otf') ||
      lower.endsWith('.woff') ||
      lower.endsWith('.woff2');
}

bool _shouldPreloadPreview(String path, int size) {
  final lower = path.toLowerCase();
  if (lower.endsWith('.opf') || lower.endsWith('.ncx')) {
    return size <= 768 * 1024;
  }
  if (lower.endsWith('.xhtml') ||
      lower.endsWith('.html') ||
      lower.endsWith('.htm')) {
    return size <= 12 * 1024;
  }
  if (lower.endsWith('.css')) {
    return size <= 256 * 1024;
  }
  return false;
}

String? _titleHintForFile(String path, String content) {
  final lower = path.toLowerCase();
  if (lower.endsWith('.opf')) {
    return '元数据';
  }
  if (lower.endsWith('.ncx')) {
    return '目录结构';
  }
  if (!lower.endsWith('.xhtml') &&
      !lower.endsWith('.html') &&
      !lower.endsWith('.htm')) {
    return null;
  }
  final title = RegExp(
        r'<h[1-6][^>]*>([\s\S]*?)</h[1-6]>',
        caseSensitive: false,
      ).firstMatch(content)?.group(1) ??
      RegExp(
        r'<title[^>]*>([\s\S]*?)</title>',
        caseSensitive: false,
      ).firstMatch(content)?.group(1) ??
      '';
  final cleaned = title
      .replaceAll(RegExp(r'<[^>]+>'), '')
      .replaceAll('&nbsp;', ' ')
      .replaceAll('&amp;', '&')
      .replaceAll('&lt;', '<')
      .replaceAll('&gt;', '>')
      .replaceAll('&quot;', '"')
      .replaceAll('&apos;', "'")
      .replaceAll(RegExp(r'\s+'), ' ')
      .trim();
  return cleaned.isEmpty ? null : cleaned;
}

Future<List<_ZipEntryMeta>> _scanZipCentralDirectory(String path) async {
  final raf = await File(path).open();
  try {
    final length = await raf.length();
    final tailLength = math.min(length, 1024 * 1024);
    await raf.setPosition(length - tailLength);
    final tail = await raf.read(tailLength);
    final eocdOffset = _findEocd(tail);
    if (eocdOffset < 0) {
      throw StateError('Could not find ZIP central directory.');
    }

    var directorySize = _u32(tail, eocdOffset + 12);
    var directoryOffset = _u32(tail, eocdOffset + 16);
    final entryCount = _u16(tail, eocdOffset + 10);
    if (directoryOffset == 0xffffffff ||
        directorySize == 0xffffffff ||
        entryCount == 0xffff) {
      final zip64 = await _readZip64DirectoryInfo(raf, tail, eocdOffset);
      directorySize = zip64.size;
      directoryOffset = zip64.offset;
    }

    await raf.setPosition(directoryOffset);
    final central = await raf.read(directorySize);
    final entries = <_ZipEntryMeta>[];
    var offset = 0;
    while (
        offset + 46 <= central.length && _u32(central, offset) == 0x02014b50) {
      final compressionMethod = _u16(central, offset + 10);
      var compressedSize = _u32(central, offset + 20);
      var uncompressedSize = _u32(central, offset + 24);
      final nameLength = _u16(central, offset + 28);
      final extraLength = _u16(central, offset + 30);
      final commentLength = _u16(central, offset + 32);
      var localHeaderOffset = _u32(central, offset + 42);
      final nameStart = offset + 46;
      final extraStart = nameStart + nameLength;
      final next = extraStart + extraLength + commentLength;
      if (next > central.length) {
        break;
      }
      final name = utf8.decode(
        central.sublist(nameStart, extraStart),
        allowMalformed: true,
      );
      if ((compressedSize == 0xffffffff || uncompressedSize == 0xffffffff) &&
          extraLength > 0) {
        final sizes = _readZip64EntrySizes(
          central.sublist(extraStart, extraStart + extraLength),
          compressedSize,
          uncompressedSize,
          localHeaderOffset,
        );
        compressedSize = sizes.compressed;
        uncompressedSize = sizes.uncompressed;
        localHeaderOffset = sizes.localHeaderOffset;
      }
      entries.add(_ZipEntryMeta(
        name: name,
        size: uncompressedSize,
        compressedSize: compressedSize,
        compressionMethod: compressionMethod,
        localHeaderOffset: localHeaderOffset,
      ));
      offset = next;
    }
    return entries;
  } finally {
    await raf.close();
  }
}

int _findEocd(List<int> bytes) {
  for (var i = bytes.length - 22; i >= 0; i -= 1) {
    if (_u32(bytes, i) == 0x06054b50) {
      return i;
    }
  }
  return -1;
}

Future<_ZipDirectoryInfo> _readZip64DirectoryInfo(
  RandomAccessFile raf,
  List<int> tail,
  int eocdOffset,
) async {
  final locatorOffset = eocdOffset - 20;
  if (locatorOffset < 0 || _u32(tail, locatorOffset) != 0x07064b50) {
    throw StateError('ZIP64 archive is missing locator.');
  }
  final zip64Offset = _u64(tail, locatorOffset + 8);
  await raf.setPosition(zip64Offset);
  final record = await raf.read(56);
  if (_u32(record, 0) != 0x06064b50) {
    throw StateError('Invalid ZIP64 central directory.');
  }
  return _ZipDirectoryInfo(
    size: _u64(record, 40),
    offset: _u64(record, 48),
  );
}

_Zip64EntrySizes _readZip64EntrySizes(
  List<int> extra,
  int compressedSize,
  int uncompressedSize,
  int localHeaderOffset,
) {
  var cursor = 0;
  var compressed = compressedSize;
  var uncompressed = uncompressedSize;
  var headerOffset = localHeaderOffset;
  while (cursor + 4 <= extra.length) {
    final id = _u16(extra, cursor);
    final size = _u16(extra, cursor + 2);
    final dataStart = cursor + 4;
    final dataEnd = dataStart + size;
    if (dataEnd > extra.length) {
      break;
    }
    var dataOffset = dataStart;
    if (id == 0x0001) {
      if (uncompressed == 0xffffffff && dataOffset + 8 <= dataEnd) {
        uncompressed = _u64(extra, dataOffset);
        dataOffset += 8;
      }
      if (compressed == 0xffffffff && dataOffset + 8 <= dataEnd) {
        compressed = _u64(extra, dataOffset);
        dataOffset += 8;
      }
      if (headerOffset == 0xffffffff && dataOffset + 8 <= dataEnd) {
        headerOffset = _u64(extra, dataOffset);
      }
      break;
    }
    cursor = dataEnd;
  }
  return _Zip64EntrySizes(
    compressed: compressed,
    uncompressed: uncompressed,
    localHeaderOffset: headerOffset,
  );
}

int _u16(List<int> bytes, int offset) {
  return bytes[offset] | (bytes[offset + 1] << 8);
}

int _u32(List<int> bytes, int offset) {
  return _u16(bytes, offset) | (_u16(bytes, offset + 2) << 16);
}

int _u64(List<int> bytes, int offset) {
  return _u32(bytes, offset) | (_u32(bytes, offset + 4) << 32);
}

EpubEditorFileKind _epubKindForPath(String path) {
  return switch (p.extension(path).toLowerCase()) {
    '.html' ||
    '.htm' ||
    '.xhtml' ||
    '.css' ||
    '.xml' ||
    '.opf' ||
    '.ncx' =>
      EpubEditorFileKind.text,
    '.txt' || '.md' || '.svg' || '.js' || '.json' => EpubEditorFileKind.text,
    '.png' ||
    '.jpg' ||
    '.jpeg' ||
    '.gif' ||
    '.webp' =>
      EpubEditorFileKind.image,
    '.ttf' || '.otf' || '.woff' || '.woff2' => EpubEditorFileKind.font,
    _ => EpubEditorFileKind.binary,
  };
}

String _epubFileSortKey(String path) {
  if (path == 'mimetype') {
    return '0000/$path';
  }
  if (path.startsWith('META-INF/')) {
    return '0001/$path';
  }
  return '0002/$path';
}

class _ZipEntryMeta {
  const _ZipEntryMeta({
    required this.name,
    required this.size,
    required this.compressedSize,
    required this.compressionMethod,
    required this.localHeaderOffset,
  });

  final String name;
  final int size;
  final int compressedSize;
  final int compressionMethod;
  final int localHeaderOffset;
}

class _ZipDirectoryInfo {
  const _ZipDirectoryInfo({
    required this.size,
    required this.offset,
  });

  final int size;
  final int offset;
}

class _Zip64EntrySizes {
  const _Zip64EntrySizes({
    required this.compressed,
    required this.uncompressed,
    required this.localHeaderOffset,
  });

  final int compressed;
  final int uncompressed;
  final int localHeaderOffset;
}

class _SingleFileReadRequest {
  const _SingleFileReadRequest(this.epubPath, this.filePath);

  final String epubPath;
  final String filePath;
}

Future<List<int>> _readSingleEpubFile(_SingleFileReadRequest request) async {
  final entries = await _scanZipCentralDirectory(request.epubPath);
  final entry =
      entries.where((item) => item.name == request.filePath).firstOrNull;
  if (entry == null) {
    throw StateError('EPUB file not found: ${request.filePath}');
  }
  final raf = await File(request.epubPath).open();
  try {
    await raf.setPosition(entry.localHeaderOffset);
    final header = await raf.read(30);
    if (_u32(header, 0) != 0x04034b50) {
      throw StateError('Invalid ZIP local file header: ${request.filePath}');
    }
    final nameLength = _u16(header, 26);
    final extraLength = _u16(header, 28);
    await raf
        .setPosition(entry.localHeaderOffset + 30 + nameLength + extraLength);
    final compressed = await raf.read(entry.compressedSize);
    if (entry.compressionMethod == 0) {
      return compressed;
    }
    if (entry.compressionMethod == 8) {
      return inflateBuffer(compressed) ?? const [];
    }
    throw StateError(
      'Unsupported ZIP compression ${entry.compressionMethod}: ${request.filePath}',
    );
  } finally {
    await raf.close();
  }
}

Future<String> _readSingleEpubTextFile(_SingleFileReadRequest request) async {
  final bytes = await _readSingleEpubFile(request);
  return utf8.decode(bytes, allowMalformed: true);
}
