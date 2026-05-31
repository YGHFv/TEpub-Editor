import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:path/path.dart' as p;

import '../../settings/services/font_asset_service.dart';
import '../../settings/services/style_template_service.dart';
import '../models/epub_project.dart';
import '../services/epub_make_service.dart';

final epubMakeControllerProvider =
    StateNotifierProvider<EpubMakeController, EpubProject>((ref) {
  return EpubMakeController(
    ref.read(epubMakeServiceProvider),
    ref.read(fontAssetServiceProvider.future),
    ref.read(styleTemplateServiceProvider.future),
  )..loadFonts();
});

class EpubMakeController extends StateNotifier<EpubProject> {
  EpubMakeController(
    this._service,
    this._fontServiceFuture,
    this._styleTemplateServiceFuture,
  ) : super(EpubProject.empty());

  final EpubMakeService _service;
  final Future<FontAssetService> _fontServiceFuture;
  final Future<StyleTemplateService> _styleTemplateServiceFuture;

  Future<void> loadFonts() async {
    try {
      final service = await _fontServiceFuture;
      final fonts = await service.listFonts();
      if (!mounted) {
        return;
      }
      state = state.copyWith(
        fonts: [
          for (final font in fonts)
            EpubFontAsset(
              family: font.family,
              fileName: font.fileName,
              path: font.path,
            ),
        ],
      );
    } catch (_) {
      // Font listing should not block EPUB creation.
    }
  }

  Future<void> pickSource() async {
    state = state.copyWith(busy: true, clearError: true, clearResult: true);
    try {
      final source = await _service.pickSource();
      if (source == null) {
        state = state.copyWith(busy: false, status: '已取消导入。');
        return;
      }
      final title = p.basenameWithoutExtension(source.name);
      final chapters = _service.scanChapters(source.content, state.rules);
      state = state.copyWith(
        sourcePath: source.path,
        sourceName: source.name,
        content: source.content,
        title: title,
        chapters: chapters,
        status: '已导入 ${source.name}，识别到 ${chapters.length} 个目录项。',
        busy: false,
      );
    } catch (error) {
      state = state.copyWith(
        busy: false,
        error: '$error',
        status: '导入文本失败。',
      );
    }
  }

  Future<void> pickCover() async {
    final coverPath = await _service.pickCover();
    if (coverPath == null) {
      return;
    }
    state = state.copyWith(
      coverPath: coverPath,
      clearResult: true,
      status: '已选择封面：${p.basename(coverPath)}',
    );
  }

  Future<void> searchCovers() async {
    state = state.copyWith(
      coverSearching: true,
      coverResults: const [],
      status: '正在搜索封面...',
      clearError: true,
    );
    try {
      final results = await _service.searchCovers(
        title: state.title,
        author: state.author,
      );
      state = state.copyWith(
        coverSearching: false,
        coverResults: results,
        status: results.isEmpty ? '没有找到合适的封面。' : '找到 ${results.length} 个封面结果。',
      );
    } catch (error) {
      state = state.copyWith(
        coverSearching: false,
        error: '$error',
        status: '封面搜索失败。',
      );
    }
  }

  Future<void> applyRemoteCover(CoverSearchResult result) async {
    state = state.copyWith(coverSearching: true, status: '正在下载封面...');
    try {
      final coverPath = await _service.downloadCoverToCache(result);
      state = state.copyWith(
        coverSearching: false,
        coverPath: coverPath,
        status: '已应用封面：${result.title}',
        clearResult: true,
      );
    } catch (error) {
      state = state.copyWith(
        coverSearching: false,
        error: '$error',
        status: '封面下载失败。',
      );
    }
  }

  void updateTitle(String value) {
    state = state.copyWith(title: value, clearResult: true);
  }

  void updateAuthor(String value) {
    state = state.copyWith(author: value, clearResult: true);
  }

  void updatePublisher(String value) {
    state = state.copyWith(publisher: value, clearResult: true);
  }

  void updateDate(String value) {
    state = state.copyWith(date: value, clearResult: true);
  }

  void updateDescription(String value) {
    state = state.copyWith(description: value, clearResult: true);
  }

  void updateTags(String value) {
    final tags = value
        .split(RegExp(r'[,，;；\s]+'))
        .map((tag) => tag.trim())
        .where((tag) => tag.isNotEmpty)
        .toList();
    state = state.copyWith(tags: tags, clearResult: true);
  }

  void updateMainCss(String value) {
    state = state.copyWith(mainCss: value, clearResult: true);
  }

  Future<void> applyStyleTemplate(String id) async {
    try {
      final service = await _styleTemplateServiceFuture;
      final templates = await service.listTemplates();
      final template = templates.firstWhere((item) => item.id == id);
      state = state.copyWith(
        mainCss: template.css,
        status: '已应用样式模板：${template.name}',
        clearResult: true,
      );
    } catch (error) {
      state = state.copyWith(
        error: '$error',
        status: '应用样式模板失败。',
      );
    }
  }

  void updateFontCss(String value) {
    state = state.copyWith(fontCss: value, clearResult: true);
  }

  void updateUuid(String value) {
    state = state.copyWith(uuid: value, clearResult: true);
  }

  void updateRulePattern(int index, String value) {
    final rules = [...state.rules];
    if (index < 0 || index >= rules.length) {
      return;
    }
    rules[index] = rules[index].copyWith(pattern: value);
    state = state.copyWith(rules: rules, clearResult: true);
  }

  void updateRuleLevel(int index, int value) {
    final rules = [...state.rules];
    if (index < 0 || index >= rules.length) {
      return;
    }
    rules[index] = rules[index].copyWith(level: value);
    state = state.copyWith(rules: rules, clearResult: true);
  }

  void addRule(int level) {
    state = state.copyWith(
      rules: [
        ...state.rules,
        EpubTocRule(pattern: r'^.+$', level: level),
      ],
      clearResult: true,
    );
  }

  void removeRule(int index) {
    if (state.rules.length <= 1 || index < 0 || index >= state.rules.length) {
      return;
    }
    final rules = [...state.rules]..removeAt(index);
    state = state.copyWith(rules: rules, clearResult: true);
  }

  void scanToc() {
    try {
      final chapters = _service.scanChapters(state.content, state.rules);
      state = state.copyWith(
        chapters: chapters,
        status: chapters.isEmpty
            ? '没有识别到目录，将按单章正文生成。'
            : '已重新扫描目录：${chapters.length} 项。',
        clearError: true,
        clearResult: true,
      );
    } catch (error) {
      state = state.copyWith(
        error: '$error',
        status: '目录扫描失败，请检查正则。',
      );
    }
  }

  void toggleChapterCollapsed(String id) {
    state = state.copyWith(
      chapters: [
        for (final chapter in state.chapters)
          if (chapter.id == id)
            chapter.copyWith(collapsed: !chapter.collapsed)
          else
            chapter,
      ],
    );
  }

  void renameChapter(int index, String title) {
    if (index < 0 || index >= state.chapters.length || title.trim().isEmpty) {
      return;
    }
    final lines = state.content.split('\n');
    final chapter = state.chapters[index];
    final titleLineIndex = chapter.lineNumber - 1;
    if (titleLineIndex < 0 || titleLineIndex >= lines.length) {
      return;
    }
    lines[titleLineIndex] = title.trim();
    final nextContent = lines.join('\n');
    final chapters = _service.scanChapters(nextContent, state.rules);
    state = state.copyWith(
      content: nextContent,
      chapters: chapters,
      status: '已重命名目录：${title.trim()}',
      clearResult: true,
    );
  }

  void updateChapterBody(int index, String body) {
    if (index < 0 || index >= state.chapters.length) {
      return;
    }
    final lines = state.content.split('\n');
    final chapter = state.chapters[index];
    final start = chapter.lineNumber;
    final endExclusive = index + 1 < state.chapters.length
        ? state.chapters[index + 1].lineNumber - 1
        : lines.length;
    if (start < 0 || start > lines.length || endExclusive < start) {
      return;
    }
    final nextLines = [
      ...lines.take(start),
      ...body.replaceAll('\r\n', '\n').replaceAll('\r', '\n').split('\n'),
      ...lines.skip(endExclusive),
    ];
    final nextContent = nextLines.join('\n');
    final chapters = _service.scanChapters(nextContent, state.rules);
    state = state.copyWith(
      content: nextContent,
      chapters: chapters,
      status: '已更新章节正文：${chapter.title}',
      clearResult: true,
    );
  }

  void saveChapterEdit(int index, String title, String body) {
    if (index < 0 || index >= state.chapters.length || title.trim().isEmpty) {
      return;
    }
    final lines = state.content.split('\n');
    final chapter = state.chapters[index];
    final titleLineIndex = chapter.lineNumber - 1;
    final bodyStart = chapter.lineNumber;
    final bodyEndExclusive = index + 1 < state.chapters.length
        ? state.chapters[index + 1].lineNumber - 1
        : lines.length;
    if (titleLineIndex < 0 ||
        titleLineIndex >= lines.length ||
        bodyStart < 0 ||
        bodyStart > lines.length ||
        bodyEndExclusive < bodyStart) {
      return;
    }
    final nextLines = [
      ...lines.take(titleLineIndex),
      title.trim(),
      ...body.replaceAll('\r\n', '\n').replaceAll('\r', '\n').split('\n'),
      ...lines.skip(bodyEndExclusive),
    ];
    final nextContent = nextLines.join('\n');
    final chapters = _service.scanChapters(nextContent, state.rules);
    state = state.copyWith(
      content: nextContent,
      chapters: chapters,
      status: '已保存章节：${title.trim()}',
      clearResult: true,
    );
  }

  Future<void> buildEpub() async {
    if (!state.hasSource) {
      state = state.copyWith(status: '请先导入文本。');
      return;
    }

    final outputPath = await _service.pickOutputPath(state.title);
    if (outputPath == null) {
      state = state.copyWith(status: '已取消生成。');
      return;
    }

    state = state.copyWith(busy: true, clearError: true, clearResult: true);
    try {
      final chapters = state.chapters.isEmpty
          ? _service.scanChapters(state.content, state.rules)
          : state.chapters;
      final nextState = state.copyWith(chapters: chapters);
      final result = await _service.buildEpub(
        project: nextState,
        outputPath: outputPath,
      );
      state = nextState.copyWith(
        busy: false,
        result: result,
        status:
            '已生成《${result.title}》，${result.chapterCount} 个目录项，约 ${result.wordCount} 字。',
      );
    } catch (error) {
      state = state.copyWith(
        busy: false,
        error: '$error',
        status: '生成 EPUB 失败。',
      );
    }
  }

  Future<void> openGeneratedFile() async {
    final outputPath = state.result?.outputPath;
    if (outputPath == null) {
      return;
    }
    await _service.openFile(outputPath);
  }

  Future<void> revealGeneratedFile() async {
    final outputPath = state.result?.outputPath;
    if (outputPath == null) {
      return;
    }
    await _service.revealInExplorer(outputPath);
  }
}
