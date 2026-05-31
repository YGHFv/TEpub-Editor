import 'package:flutter_test/flutter_test.dart';
import 'package:tepub_editor_flutter/src/features/editor/models/editor_document.dart';
import 'package:tepub_editor_flutter/src/features/editor/services/editor_file_service.dart';

void main() {
  test('scanChapters default rule only detects top chapter and volume titles',
      () {
    const service = EditorFileService();

    final chapters = service.scanChapters('''
序章
这里是序章，不应被默认规则识别。
第一卷 风起
第一章 开始
这里是第一章。
第2章 继续
这里是第二章。
3. 尾声之前
6.666666666....
Chapter 4 Finale
The end.
''');

    expect(chapters.map((chapter) => chapter.title), [
      '第一卷 风起',
      '第一章 开始',
      '第2章 继续',
    ]);
    expect(chapters.map((chapter) => chapter.level), [1, 2, 2]);
    expect(chapters.last.offset, greaterThan(chapters.first.offset));
  });

  test('scanChapters supports custom regular expressions', () {
    const service = EditorFileService();

    final chapters = service.scanChapters(
      '''
序章
这里是序章。
1. 起点
正文
1.1 小节
正文
''',
      toc: const EditorTocState(
        pattern: r'^(序章|[0-9]+(?:\.[0-9]+)?\.?\s+\S+.*)$',
        useCustomPattern: true,
      ),
    );

    expect(chapters.map((chapter) => chapter.title), [
      '序章',
      '1. 起点',
      '1.1 小节',
    ]);
  });

  test('scanChapters avoids common prose false positives', () {
    const service = EditorFileService();

    final chapters = service.scanChapters('''
第一章 真正的章节
第二节课开始之前，他先看了看窗外。
序列8时，系统仍然保持静默。
第3回的时候，故事没有真正换章。
第二章 真正继续
正文继续。
''');

    expect(chapters, hasLength(2));
    expect(chapters.first.title, '第一章 真正的章节');
    expect(chapters.last.title, '第二章 真正继续');
  });
}
