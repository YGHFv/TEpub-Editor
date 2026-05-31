import 'dart:convert';
import 'dart:io';

import 'package:archive/archive.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:tepub_editor_flutter/src/features/epub_editor/models/epub_editor_document.dart';
import 'package:tepub_editor_flutter/src/features/epub_editor/services/epub_editor_service.dart';

void main() {
  test('openEpub and saveEpub persist edited text files', () async {
    const service = EpubEditorService();
    final temp = await Directory.systemTemp.createTemp('tepub_epub_edit_');
    addTearDown(() async {
      if (await temp.exists()) {
        await temp.delete(recursive: true);
      }
    });

    final path = '${temp.path}/book.epub';
    final archive = Archive()
      ..addFile(
        ArchiveFile.noCompress(
          'mimetype',
          'application/epub+zip'.length,
          utf8.encode('application/epub+zip'),
        ),
      )
      ..addFile(
        ArchiveFile(
          'OEBPS/Text/chapter1.xhtml',
          '<p>旧内容</p>'.length,
          utf8.encode('<p>旧内容</p>'),
        ),
      );
    await File(path).writeAsBytes(ZipEncoder().encode(archive)!);

    final opened = await service.openEpub(path);
    final edited = opened.copyWith(
      files: [
        for (final file in opened.files)
          if (file.path == 'OEBPS/Text/chapter1.xhtml')
            file.copyWith(content: '<p>新内容</p>', modified: true)
          else
            file,
      ],
    );

    await service.saveEpub(edited);

    final saved = await service.openEpub(path);
    final chapterMeta = saved.files.firstWhere(
      (file) => file.path == 'OEBPS/Text/chapter1.xhtml',
    );
    final chapter = await service.loadFilePayload(
      epubPath: saved.epubPath,
      file: chapterMeta,
    );
    expect(chapter.content, '<p>新内容</p>');
  });

  test('saveEpub keeps modified binary bytes', () async {
    const service = EpubEditorService();
    final temp = await Directory.systemTemp.createTemp('tepub_epub_bin_');
    addTearDown(() async {
      if (await temp.exists()) {
        await temp.delete(recursive: true);
      }
    });

    final path = '${temp.path}/book.epub';
    final archive = Archive()
      ..addFile(
        ArchiveFile.noCompress(
          'mimetype',
          'application/epub+zip'.length,
          utf8.encode('application/epub+zip'),
        ),
      )
      ..addFile(
        ArchiveFile('OEBPS/Images/a.bin', 3, [1, 2, 3]),
      );
    await File(path).writeAsBytes(ZipEncoder().encode(archive)!);

    final opened = await service.openEpub(path);
    final edited = opened.copyWith(
      files: [
        for (final file in opened.files)
          if (file.path == 'OEBPS/Images/a.bin')
            file.copyWith(bytes: [7, 8, 9], size: 3, modified: true)
          else
            file,
      ],
    );

    await service.saveEpub(edited);

    final savedArchive =
        ZipDecoder().decodeBytes(await File(path).readAsBytes());
    final binary = savedArchive.files.firstWhere(
      (file) => file.name == 'OEBPS/Images/a.bin',
    );
    expect(binary.content as List<int>, [7, 8, 9]);
  });

  test('parseMetadata and updateMetadataInOpf handle common OPF fields', () {
    const service = EpubEditorService();
    const opf = '''
<package>
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:identifier id="BookId">old-uuid</dc:identifier>
    <dc:title>旧书名</dc:title>
    <dc:creator>旧作者</dc:creator>
    <dc:publisher>旧出版社</dc:publisher>
    <dc:description>旧简介</dc:description>
    <dc:subject>奇幻</dc:subject>
    <meta name="cover" content="cover-image"/>
  </metadata>
  <manifest>
    <item id="cover-image" href="Images/cover.jpg" media-type="image/jpeg"/>
  </manifest>
</package>
''';
    final metadata = service.parseMetadata([
      const EpubEditorFile(
        path: 'OEBPS/content.opf',
        size: opf.length,
        kind: EpubEditorFileKind.text,
        content: opf,
      ),
    ]);

    expect(metadata.title, '旧书名');
    expect(metadata.author, '旧作者');
    expect(metadata.coverPath, 'OEBPS/Images/cover.jpg');

    final updated = service.updateMetadataInOpf(
      opf,
      metadata.copyWith(
        title: '新书名',
        author: '新作者',
        subjects: const ['科幻', '长篇'],
      ),
    );

    expect(updated, contains('<dc:title>新书名</dc:title>'));
    expect(updated, contains('<dc:creator>新作者</dc:creator>'));
    expect(updated, contains('<dc:subject>科幻</dc:subject>'));
    expect(updated, contains('<dc:subject>长篇</dc:subject>'));
  });
  test('loads toc from the reported large EPUB without hanging', () async {
    const sourcePath = r'C:\Users\YGHF\Desktop\1\H-0001 修罗场玩家1.4.epub';
    final source = File(sourcePath);
    if (!await source.exists()) {
      return;
    }

    const service = EpubEditorService();
    final opened = await service.openEpub(sourcePath).timeout(
      const Duration(seconds: 5),
    );
    final toc = opened.files.firstWhere(
      (file) => file.path == 'OEBPS/toc.ncx',
    );
    final loaded = await service
        .loadFilePayload(epubPath: opened.epubPath, file: toc)
        .timeout(const Duration(seconds: 5));

    expect(loaded.content, isNotNull);
    expect(loaded.content, contains('<ncx'));
  });
}
