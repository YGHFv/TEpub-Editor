import 'dart:convert';
import 'dart:io';

import 'package:archive/archive.dart';
import 'package:file_picker/file_picker.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:http/http.dart' as http;
import 'package:path/path.dart' as p;
import 'package:path_provider/path_provider.dart';

import '../../settings/models/font_asset.dart';
import '../models/epub_project.dart';

final epubMakeServiceProvider = Provider<EpubMakeService>((ref) {
  return const EpubMakeService();
});

class EpubMakeService {
  const EpubMakeService();

  static const defaultMainCss = _defaultCss;
  static const defaultFontCss = _defaultFontCss;

  Future<({String path, String name, String content})?> pickSource() async {
    final result = await FilePicker.platform.pickFiles(
      allowMultiple: false,
      type: FileType.custom,
      allowedExtensions: const ['txt', 'md', 'html', 'htm'],
    );
    final path = result?.files.single.path;
    if (path == null) {
      return null;
    }
    return (
      path: path,
      name: p.basename(path),
      content: await _readText(path),
    );
  }

  Future<String?> pickCover() async {
    final result = await FilePicker.platform.pickFiles(
      allowMultiple: false,
      type: FileType.image,
    );
    return result?.files.single.path;
  }

  Future<void> openFile(String path) async {
    if (path.trim().isEmpty) {
      return;
    }
    await Process.start(path, const [], runInShell: true);
  }

  Future<void> revealInExplorer(String path) async {
    if (path.trim().isEmpty) {
      return;
    }
    if (Platform.isWindows) {
      await Process.start('explorer.exe', ['/select,', path]);
      return;
    }
    await Process.start('open', [p.dirname(path)], runInShell: true);
  }

  Future<List<CoverSearchResult>> searchCovers({
    required String title,
    required String author,
  }) async {
    final cleanTitle = title.trim();
    if (cleanTitle.isEmpty) {
      throw StateError('请先填写书名');
    }

    final queries = <String>{
      cleanTitle,
      if (author.trim().isNotEmpty) '$cleanTitle ${author.trim()}',
      '$cleanTitle 小说封面',
    };
    final seen = <String>{};
    final results = <({int score, CoverSearchResult result})>[];
    final titleKey = _compactKey(cleanTitle);
    final client = http.Client();
    try {
      var queryIndex = 0;
      for (final query in queries) {
        final uri = Uri.https('cn.bing.com', '/images/search', {
          'q': query,
          'form': 'HDRSC2',
          'first': '1',
        });
        final response = await client.get(
          uri,
          headers: const {
            'User-Agent':
                'Mozilla/5.0 (Windows NT 10.0; Win64; x64) TEpub-Editor-Flutter/0.1',
            'Referer': 'https://cn.bing.com/images',
            'Accept-Language': 'zh-CN,zh;q=0.9',
          },
        );
        if (response.statusCode >= 400) {
          queryIndex += 1;
          continue;
        }
        final html = response.body;
        final matches = RegExp(r'm="([^"]+)"').allMatches(html).take(32);
        var itemIndex = 0;
        for (final match in matches) {
          final rawMeta = _htmlUnescape(match.group(1) ?? '');
          final meta = jsonDecode(rawMeta);
          if (meta is! Map<String, Object?>) {
            continue;
          }
          final imageUrl = _normalizeCoverUrl(
            (meta['murl'] ?? meta['turl'] ?? '').toString(),
          );
          if (imageUrl.isEmpty ||
              !(imageUrl.startsWith('http://') ||
                  imageUrl.startsWith('https://')) ||
              !seen.add(imageUrl)) {
            continue;
          }
          final pageUrl = _normalizeCoverUrl((meta['purl'] ?? '').toString());
          final resultTitle = _htmlUnescape(
            (meta['t'] ?? cleanTitle).toString(),
          );
          final source = _hostFromUrl(pageUrl).ifBlank(_hostFromUrl(imageUrl));
          final resultKey = _compactKey('$resultTitle $pageUrl $source');
          final preferred = _isPreferredCoverSource(imageUrl, pageUrl, source);
          var score = preferred ? 180 : 0;
          score += switch (queryIndex) {
            0 => 70,
            1 => 30,
            _ => 10,
          };
          if (resultKey.contains(titleKey)) {
            score += 100;
          } else {
            score -= 80;
          }
          score -= itemIndex;
          results.add((
            score: score,
            result: CoverSearchResult(
              id: (meta['md5'] ?? meta['cid'] ?? '$queryIndex-$itemIndex')
                  .toString(),
              title: resultTitle.trim().isEmpty ? cleanTitle : resultTitle,
              imageUrl: imageUrl,
              pageUrl: pageUrl,
              source: source,
              preferred: preferred,
            ),
          ));
          itemIndex += 1;
        }
        queryIndex += 1;
      }
    } finally {
      client.close();
    }

    results.sort((a, b) => b.score.compareTo(a.score));
    return results.take(12).map((item) => item.result).toList();
  }

  Future<String> downloadCoverToCache(CoverSearchResult result) async {
    final uri = Uri.parse(result.imageUrl);
    final response = await http.get(
      uri,
      headers: {
        'User-Agent':
            'Mozilla/5.0 (Windows NT 10.0; Win64; x64) TEpub-Editor-Flutter/0.1',
        'Referer': _coverReferer(result.imageUrl),
      },
    );
    if (response.statusCode >= 400 || response.bodyBytes.isEmpty) {
      throw StateError('下载封面失败');
    }
    if (response.bodyBytes.length > 12 * 1024 * 1024) {
      throw StateError('封面图片过大');
    }
    final cacheDir = await getTemporaryDirectory();
    final dir = Directory(p.join(cacheDir.path, 'tepub_editor_flutter_covers'));
    if (!await dir.exists()) {
      await dir.create(recursive: true);
    }
    final ext = _detectImageExt(response.bodyBytes);
    final file = File(
      p.join(
        dir.path,
        '${_safeFileName(result.title.ifBlank('cover'))}-${DateTime.now().millisecondsSinceEpoch}.$ext',
      ),
    );
    await file.writeAsBytes(response.bodyBytes);
    return file.path;
  }

  Future<String?> pickOutputPath(String title) {
    return FilePicker.platform.saveFile(
      dialogTitle: '保存 EPUB',
      fileName: '${_safeFileName(title.ifBlank('untitled'))}.epub',
      type: FileType.custom,
      allowedExtensions: const ['epub'],
    );
  }

  Future<EpubBuildResult?> buildFromEditor({
    required String title,
    required String author,
    required String publisher,
    required String date,
    required String description,
    required List<String> tags,
    required String mainCss,
    required String fontCss,
    required String uuid,
    required String sourceName,
    required String content,
    required String coverPath,
    required List<EpubChapter> chapters,
    required List<FontAsset> fonts,
  }) async {
    final outputPath = await pickOutputPath(title);
    if (outputPath == null) {
      return null;
    }
    return buildEpub(
      project: EpubProject.empty().copyWith(
        title: title,
        author: author,
        publisher: publisher,
        date: date,
        description: description,
        tags: tags,
        mainCss: mainCss,
        fontCss: fontCss,
        uuid: uuid,
        sourcePath: sourceName,
        sourceName: sourceName,
        content: content,
        coverPath: coverPath,
        chapters: chapters,
        fonts: [
          for (final font in fonts)
            EpubFontAsset(
              family: font.family,
              fileName: font.fileName,
              path: font.path,
            ),
        ],
      ),
      outputPath: outputPath,
    );
  }

  List<EpubChapter> scanChapters(String content, List<EpubTocRule> rules) {
    final normalized = _normalizeText(content);
    final lines = normalized.split('\n');
    final compiledRules = <({RegExp regex, int level})>[];

    for (final rule in rules) {
      if (rule.pattern.trim().isEmpty) {
        continue;
      }
      compiledRules.add((
        regex: RegExp(rule.pattern, multiLine: true),
        level: rule.level,
      ));
    }

    final chapters = <EpubChapter>[];
    EpubChapter? current;
    var currentWordCount = 0;

    for (var index = 0; index < lines.length; index += 1) {
      final rawLine = lines[index];
      final line = rawLine.trim();
      final level = _matchLevel(rawLine, compiledRules);
      if (line.isNotEmpty && level != null) {
        if (current != null) {
          chapters.add(current.copyWith(wordCount: currentWordCount));
        }
        current = EpubChapter(
          id: 'chapter-${chapters.length + 1}',
          title: line,
          lineNumber: index + 1,
          level: level,
          isMeta: _isMetaTitle(line, level, chapters.isEmpty),
          wordCount: 0,
        );
        currentWordCount = 0;
      } else if (current != null && line.isNotEmpty) {
        currentWordCount += line.runes.length;
      }
    }

    if (current != null) {
      chapters.add(current.copyWith(wordCount: currentWordCount));
    }

    if (chapters.isEmpty && normalized.trim().isNotEmpty) {
      chapters.add(
        EpubChapter(
          id: 'chapter-1',
          title: '正文',
          lineNumber: 1,
          level: 3,
          isMeta: false,
          wordCount: normalized.trim().runes.length,
        ),
      );
    }

    return chapters;
  }

  Future<EpubBuildResult> buildEpub({
    required EpubProject project,
    required String outputPath,
  }) async {
    final content = _normalizeText(project.content);
    final chapters = project.chapters.isEmpty
        ? scanChapters(content, project.rules)
        : project.chapters;
    final title = project.title.trim().ifBlank(
          p.basenameWithoutExtension(project.sourceName).ifBlank('未命名图书'),
        );
    final author = project.author.trim().ifBlank('未知作者');
    final uuid = project.uuid.trim().ifBlank(
          'tepub-${DateTime.now().microsecondsSinceEpoch}',
        );

    final archive = Archive();
    archive.addFile(
      ArchiveFile.noCompress(
        'mimetype',
        'application/epub+zip'.length,
        utf8.encode('application/epub+zip'),
      ),
    );
    _addTextFile(archive, 'META-INF/container.xml', _containerXml);
    final validFonts = <EpubFontAsset>[];
    for (final font in project.fonts) {
      final file = File(font.path);
      if (!await file.exists()) {
        continue;
      }
      final bytes = await file.readAsBytes();
      final safeName = _safeFileName(font.fileName).ifBlank(
        'font-${validFonts.length + 1}${p.extension(font.path)}',
      );
      validFonts.add(
        EpubFontAsset(
          family: font.family.ifBlank(p.basenameWithoutExtension(safeName)),
          fileName: safeName,
          path: font.path,
        ),
      );
      archive.addFile(
        ArchiveFile('OEBPS/Fonts/$safeName', bytes.length, bytes),
      );
    }

    _addTextFile(
      archive,
      'OEBPS/Styles/font.css',
      _fontCssWithAssets(
        project.fontCss.trim().isEmpty ? _defaultFontCss : project.fontCss,
        validFonts,
      ),
    );
    _addTextFile(
      archive,
      'OEBPS/Styles/main.css',
      project.mainCss.trim().isEmpty ? _defaultCss : project.mainCss,
    );

    String? coverHref;
    if (project.coverPath.trim().isNotEmpty &&
        await File(project.coverPath).exists()) {
      final ext = p.extension(project.coverPath).replaceFirst('.', '');
      final safeExt = ext.isEmpty ? 'jpg' : ext.toLowerCase();
      final bytes = await File(project.coverPath).readAsBytes();
      coverHref = 'Images/cover.$safeExt';
      archive.addFile(
        ArchiveFile('OEBPS/$coverHref', bytes.length, bytes),
      );
    }

    final lines = content.split('\n');
    final manifestItems = <String>[
      '<item id="style" href="Styles/main.css" media-type="text/css"/>',
      '<item id="font-style" href="Styles/font.css" media-type="text/css"/>',
      '<item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>',
      '<item id="toc" href="toc.ncx" media-type="application/x-dtbncx+xml"/>',
    ];
    for (var index = 0; index < validFonts.length; index += 1) {
      final font = validFonts[index];
      manifestItems.add(
        '<item id="font-${index + 1}" href="Fonts/${_escapeXml(font.fileName)}" media-type="${_mimeForFont(font.fileName)}"/>',
      );
    }
    if (coverHref != null) {
      manifestItems.add(
        '<item id="cover-image" href="$coverHref" media-type="${_mimeForPath(coverHref)}" properties="cover-image"/>',
      );
    }

    final spineRefs = <String>[];
    final navItems = <String>[];
    final ncxItems = <String>[];
    var wordCount = 0;

    for (var index = 0; index < chapters.length; index += 1) {
      final chapter = chapters[index];
      final filename = 'Text/chapter${index + 1}.xhtml';
      final id = 'chapter${index + 1}';
      final bodyLines = _chapterBodyLines(lines, chapters, index);
      final chapterWordCount = chapter.wordCount == 0
          ? bodyLines.join('\n').trim().runes.length
          : chapter.wordCount;
      wordCount += chapterWordCount;
      _addTextFile(
        archive,
        'OEBPS/$filename',
        _chapterXhtml(
          title: chapter.title,
          bodyLines: bodyLines,
          isMeta: chapter.isMeta,
          coverHref: index == 0 && chapter.isMeta ? coverHref : null,
          bookTitle: title,
        ),
      );
      manifestItems.add(
        '<item id="$id" href="$filename" media-type="application/xhtml+xml"/>',
      );
      spineRefs.add('<itemref idref="$id"/>');
      navItems.add(
        '<li><a href="$filename">${_escapeXml(chapter.title)}</a></li>',
      );
      ncxItems.add(
        '<navPoint id="navPoint-${index + 1}" playOrder="${index + 1}"><navLabel><text>${_escapeXml(chapter.title)}</text></navLabel><content src="$filename"/></navPoint>',
      );
    }

    _addTextFile(
      archive,
      'OEBPS/nav.xhtml',
      _navXhtml(title: title, items: navItems),
    );
    _addTextFile(
      archive,
      'OEBPS/toc.ncx',
      _tocNcx(uuid: uuid, title: title, author: author, items: ncxItems),
    );
    _addTextFile(
      archive,
      'OEBPS/content.opf',
      _opf(
        uuid: uuid,
        title: title,
        author: author,
        publisher: project.publisher,
        date: project.date,
        description: project.description,
        tags: project.tags,
        manifestItems: manifestItems,
        spineRefs: spineRefs,
        hasCover: coverHref != null,
      ),
    );

    final bytes = ZipEncoder().encode(archive);
    if (bytes == null) {
      throw StateError('EPUB 压缩失败');
    }
    await File(outputPath).writeAsBytes(bytes);

    return EpubBuildResult(
      outputPath: outputPath,
      title: title,
      chapterCount: chapters.length,
      wordCount: wordCount,
    );
  }

  Future<String> _readText(String path) async {
    final bytes = await File(path).readAsBytes();
    final text = bytes.length >= 3 &&
            bytes[0] == 0xEF &&
            bytes[1] == 0xBB &&
            bytes[2] == 0xBF
        ? utf8.decode(bytes.sublist(3), allowMalformed: true)
        : utf8.decode(bytes, allowMalformed: true);
    return _normalizeText(text);
  }

  String _normalizeText(String content) {
    return content
        .replaceAll('\r\n', '\n')
        .replaceAll('\r', '\n')
        .replaceAll('\u2028', '\n')
        .replaceAll('\u2029', '\n')
        .replaceAll('\t', '    ');
  }

  int? _matchLevel(String line, List<({RegExp regex, int level})> rules) {
    for (final rule in rules) {
      if (rule.regex.hasMatch(line)) {
        return rule.level;
      }
    }
    return null;
  }

  bool _isMetaTitle(String title, int level, bool isFirstHeading) {
    if (title.contains('卷') || title.contains('部')) {
      return false;
    }
    final isMeta =
        RegExp(r'^(书名|作者|简介|序章|楔子|引子|前言|后记|尾声)$').hasMatch(title.trim());
    return isMeta && (level == 1 || isFirstHeading);
  }

  List<String> _chapterBodyLines(
    List<String> lines,
    List<EpubChapter> chapters,
    int index,
  ) {
    final isSingleBodyChapter =
        chapters.length == 1 && chapters[index].title == '正文';
    final start = isSingleBodyChapter
        ? 0
        : chapters[index].lineNumber.clamp(0, lines.length);
    final end = index + 1 < chapters.length
        ? chapters[index + 1].lineNumber - 2
        : lines.length - 1;
    if (start > end || start >= lines.length) {
      return const [];
    }
    return lines.sublist(start, end.clamp(start, lines.length - 1) + 1);
  }

  void _addTextFile(Archive archive, String path, String content) {
    final bytes = utf8.encode(content);
    archive.addFile(ArchiveFile(path, bytes.length, bytes));
  }

  String _chapterXhtml({
    required String title,
    required List<String> bodyLines,
    required bool isMeta,
    required String? coverHref,
    required String bookTitle,
  }) {
    final body = StringBuffer();
    if (coverHref != null) {
      body.writeln(
        '<div class="cover"><img src="../$coverHref" alt="${_escapeXml(bookTitle)}"/></div>',
      );
    }
    body.writeln('<h1>${_escapeXml(title)}</h1>');
    for (final line in bodyLines) {
      final trimmed = line.trimRight();
      if (trimmed.trim().isEmpty) {
        continue;
      }
      body.writeln('<p>${_escapeXml(trimmed)}</p>');
    }
    return '''
<?xml version="1.0" encoding="utf-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml" xml:lang="zh-CN">
<head>
  <title>${_escapeXml(title)}</title>
  <link rel="stylesheet" type="text/css" href="../Styles/main.css"/>
  <link rel="stylesheet" type="text/css" href="../Styles/font.css"/>
</head>
<body class="${isMeta ? 'intro' : 'chapter'}">
$body
</body>
</html>
''';
  }

  String _navXhtml({required String title, required List<String> items}) {
    return '''
<?xml version="1.0" encoding="utf-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">
<head><title>${_escapeXml(title)}</title></head>
<body>
  <nav epub:type="toc" id="toc">
    <h1>${_escapeXml(title)}</h1>
    <ol>
      ${items.join('\n      ')}
    </ol>
  </nav>
</body>
</html>
''';
  }

  String _tocNcx({
    required String uuid,
    required String title,
    required String author,
    required List<String> items,
  }) {
    return '''
<?xml version="1.0" encoding="utf-8"?>
<ncx xmlns="http://www.daisy.org/z3986/2005/ncx/" version="2005-1">
  <head>
    <meta name="dtb:uid" content="${_escapeXml(uuid)}"/>
    <meta name="dtb:depth" content="1"/>
    <meta name="dtb:totalPageCount" content="0"/>
    <meta name="dtb:maxPageNumber" content="0"/>
  </head>
  <docTitle><text>${_escapeXml(title)}</text></docTitle>
  <docAuthor><text>${_escapeXml(author)}</text></docAuthor>
  <navMap>
    ${items.join('\n    ')}
  </navMap>
</ncx>
''';
  }

  String _opf({
    required String uuid,
    required String title,
    required String author,
    required String publisher,
    required String date,
    required String description,
    required List<String> tags,
    required List<String> manifestItems,
    required List<String> spineRefs,
    required bool hasCover,
  }) {
    return '''
<?xml version="1.0" encoding="utf-8"?>
<package xmlns="http://www.idpf.org/2007/opf" unique-identifier="BookId" version="3.0">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:identifier id="BookId">${_escapeXml(uuid)}</dc:identifier>
    <dc:title>${_escapeXml(title)}</dc:title>
    <dc:creator>${_escapeXml(author)}</dc:creator>
    ${publisher.trim().isEmpty ? '' : '<dc:publisher>${_escapeXml(publisher.trim())}</dc:publisher>'}
    ${date.trim().isEmpty ? '' : '<dc:date>${_escapeXml(date.trim())}</dc:date>'}
    ${description.trim().isEmpty ? '' : '<dc:description>${_escapeXml(description.trim())}</dc:description>'}
    ${tags.map((tag) => '<dc:subject>${_escapeXml(tag)}</dc:subject>').join('\n    ')}
    <dc:language>zh-CN</dc:language>
    <meta property="dcterms:modified">${DateTime.now().toUtc().toIso8601String().split('.').first}Z</meta>
    ${hasCover ? '<meta name="cover" content="cover-image"/>' : ''}
  </metadata>
  <manifest>
    ${manifestItems.join('\n    ')}
  </manifest>
  <spine toc="toc">
    ${spineRefs.join('\n    ')}
  </spine>
</package>
''';
  }

  String _mimeForPath(String path) {
    return switch (p.extension(path).toLowerCase()) {
      '.png' => 'image/png',
      '.gif' => 'image/gif',
      '.webp' => 'image/webp',
      '.svg' => 'image/svg+xml',
      _ => 'image/jpeg',
    };
  }

  String _mimeForFont(String path) {
    return switch (p.extension(path).toLowerCase()) {
      '.otf' => 'font/otf',
      '.woff' => 'font/woff',
      '.woff2' => 'font/woff2',
      _ => 'font/ttf',
    };
  }

  String _fontCssWithAssets(String baseCss, List<EpubFontAsset> fonts) {
    if (fonts.isEmpty) {
      return baseCss;
    }
    final buffer = StringBuffer();
    for (final font in fonts) {
      buffer.writeln('@font-face {');
      buffer.writeln('  font-family: "${_escapeCssString(font.family)}";');
      buffer.writeln(
        '  src: url("../Fonts/${_escapeCssUrl(font.fileName)}");',
      );
      buffer.writeln('}');
      buffer.writeln();
    }
    buffer.writeln(baseCss);
    return buffer.toString();
  }

  String _escapeCssString(String value) {
    return value.replaceAll(r'\', r'\\').replaceAll('"', r'\"');
  }

  String _escapeCssUrl(String value) {
    return Uri.encodeComponent(value).replaceAll('%2F', '/');
  }

  String _safeFileName(String input) {
    return input.replaceAll(RegExp(r'[\\/:*?"<>|]+'), '_').trim();
  }

  String _normalizeCoverUrl(String raw) {
    var url = raw
        .replaceAll(r'\/', '/')
        .replaceAll('&amp;', '&')
        .replaceAll('&quot;', '"')
        .trim();
    if (url.startsWith('//')) {
      url = 'https:$url';
    }
    return url;
  }

  String _htmlUnescape(String value) {
    return value
        .replaceAll('&quot;', '"')
        .replaceAll('&#34;', '"')
        .replaceAll('&#x22;', '"')
        .replaceAll('&#39;', "'")
        .replaceAll('&#x27;', "'")
        .replaceAll('&lt;', '<')
        .replaceAll('&gt;', '>')
        .replaceAll('&amp;', '&');
  }

  String _compactKey(String value) {
    return value.replaceAll(RegExp(r'[\s《》]+'), '').toLowerCase();
  }

  String _hostFromUrl(String url) {
    return Uri.tryParse(url)?.host ?? '';
  }

  bool _isPreferredCoverSource(String imageUrl, String pageUrl, String source) {
    final text = '$imageUrl $pageUrl $source'.toLowerCase();
    return text.contains('bookcover.yuewen.com') ||
        text.contains('icode.qq.com') ||
        text.contains('fanqienovel.com') ||
        text.contains('byteimg.com') ||
        text.contains('m.qidian.com');
  }

  String _coverReferer(String url) {
    final lower = url.toLowerCase();
    if (lower.contains('byteimg.com') || lower.contains('fanqienovel.com')) {
      return 'https://fanqienovel.com/';
    }
    return 'https://m.qidian.com/';
  }

  String _detectImageExt(List<int> bytes) {
    if (bytes.length >= 8 &&
        bytes[0] == 0x89 &&
        bytes[1] == 0x50 &&
        bytes[2] == 0x4E &&
        bytes[3] == 0x47) {
      return 'png';
    }
    if (bytes.length >= 6 &&
        bytes[0] == 0x47 &&
        bytes[1] == 0x49 &&
        bytes[2] == 0x46) {
      return 'gif';
    }
    if (bytes.length >= 12 &&
        bytes[8] == 0x57 &&
        bytes[9] == 0x45 &&
        bytes[10] == 0x42 &&
        bytes[11] == 0x50) {
      return 'webp';
    }
    return 'jpg';
  }

  String _escapeXml(String value) {
    return value
        .replaceAll('&', '&amp;')
        .replaceAll('<', '&lt;')
        .replaceAll('>', '&gt;')
        .replaceAll('"', '&quot;')
        .replaceAll("'", '&apos;');
  }
}

extension _BlankString on String {
  String ifBlank(String fallback) {
    return trim().isEmpty ? fallback : this;
  }
}

const _containerXml = '''
<?xml version="1.0"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>
''';

const _defaultFontCss = '''
@font-face {
  font-family: "TEpubSerif";
  src: local("Noto Serif CJK SC"), local("Source Han Serif SC"), local("SimSun");
}
''';

const _defaultCss = '''
html, body {
  margin: 0;
  padding: 0;
}
body {
  font-family: "Noto Serif CJK SC", "Source Han Serif SC", serif;
  line-height: 1.85;
  color: #1f2933;
}
h1 {
  font-size: 1.35em;
  line-height: 1.5;
  margin: 1.4em 0 1em;
  text-align: center;
}
p {
  margin: 0.45em 0;
  text-indent: 2em;
}
.cover {
  text-align: center;
  margin: 1em 0 2em;
}
.cover img {
  max-width: 90%;
  max-height: 90vh;
}
''';
