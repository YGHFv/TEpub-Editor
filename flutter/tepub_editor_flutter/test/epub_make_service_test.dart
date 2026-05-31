import 'dart:io';

import 'package:archive/archive.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:tepub_editor_flutter/src/features/epub/models/epub_project.dart';
import 'package:tepub_editor_flutter/src/features/epub/services/epub_make_service.dart';

void main() {
  test('scanChapters detects volumes and chapters from rules', () {
    const service = EpubMakeService();

    final chapters = service.scanChapters('''
书名：测试
第一卷 风起
第一章 初见
正文
第二章 远行
正文
''', EpubProject.defaultRules);

    expect(chapters.map((chapter) => chapter.title), [
      '第一卷 风起',
      '第一章 初见',
      '第二章 远行',
    ]);
    expect(chapters.map((chapter) => chapter.level), [1, 3, 3]);
  });

  test('buildEpub writes a readable EPUB archive', () async {
    const service = EpubMakeService();
    final temp = await Directory.systemTemp.createTemp('tepub_epub_test_');
    addTearDown(() async {
      if (await temp.exists()) {
        await temp.delete(recursive: true);
      }
    });

    final content = '''
第一章 初见
这里是第一章正文。
第二章 远行
这里是第二章正文。
''';
    final project = EpubProject.empty().copyWith(
      title: '测试书',
      author: '测试作者',
      sourcePath: '${temp.path}/source.txt',
      sourceName: 'source.txt',
      content: content,
      chapters: service.scanChapters(content, EpubProject.defaultRules),
    );
    final output = '${temp.path}/out.epub';

    await service.buildEpub(project: project, outputPath: output);

    final archive = ZipDecoder().decodeBytes(await File(output).readAsBytes());
    final names = archive.files.map((file) => file.name).toSet();
    expect(names, contains('mimetype'));
    expect(names, contains('META-INF/container.xml'));
    expect(names, contains('OEBPS/content.opf'));
    expect(names, contains('OEBPS/nav.xhtml'));
    expect(names, contains('OEBPS/Text/chapter1.xhtml'));
    expect(names, contains('OEBPS/Text/chapter2.xhtml'));
  });
}
