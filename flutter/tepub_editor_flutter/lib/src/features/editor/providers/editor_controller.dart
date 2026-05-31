import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../library/models/library_book.dart';
import '../models/editor_document.dart';
import '../services/editor_file_service.dart';

final editorControllerProvider =
    StateNotifierProvider<EditorController, EditorDocument>((ref) {
  return EditorController(ref);
});

class EditorController extends StateNotifier<EditorDocument> {
  EditorController(this._ref)
      : super(
          EditorDocument(
            title: '示例长篇小说',
            content: _sampleContent,
            chapters: _sampleChapters,
            selectedChapterId: _sampleChapters.first.id,
            dirty: false,
            search: EditorSearchState.empty,
            toc: EditorTocState.empty,
            history: const [],
            display: EditorDisplaySettings.defaults,
            checkReport: EditorCheckReport.empty,
          ),
        );

  final Ref _ref;
  static const EditorFileService _scanner = EditorFileService();
  Timer? _tocScanDebounce;
  Timer? _analysisDebounce;

  Future<EditorFileService> get _fileService async {
    return _ref.read(editorFileServiceProvider.future);
  }

  @override
  void dispose() {
    _tocScanDebounce?.cancel();
    _analysisDebounce?.cancel();
    super.dispose();
  }

  void updateContent(String value) {
    _setContent(value, dirty: true, rescanImmediately: false);
    _tocScanDebounce?.cancel();
    _tocScanDebounce = Timer(const Duration(milliseconds: 450), () {
      if (!mounted) {
        return;
      }
      rebuildToc(keepSelection: true);
    });
    _analysisDebounce?.cancel();
    _analysisDebounce = Timer(const Duration(milliseconds: 650), () {
      if (!mounted) {
        return;
      }
      _refreshTextAnalysis();
    });
  }

  void selectChapter(String id) {
    final chapter = state.chapters.firstWhere((item) => item.id == id);
    state = state.copyWith(
      selectedChapterId: id,
      pendingScrollOffset: chapter.offset,
    );
  }

  void updateActiveChapterByOffset(int offset) {
    final chapters = state.chapters;
    if (chapters.isEmpty) {
      return;
    }
    var active = chapters.first;
    for (final chapter in chapters) {
      if (chapter.offset <= offset) {
        active = chapter;
      } else {
        break;
      }
    }
    if (active.id != state.selectedChapterId) {
      state = state.copyWith(selectedChapterId: active.id);
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

  void collapseAllChapters() {
    state = state.copyWith(
      chapters: [
        for (final chapter in state.chapters)
          chapter.copyWith(collapsed: _chapterHasChildren(chapter)),
      ],
    );
  }

  void expandAllChapters() {
    state = state.copyWith(
      chapters: [
        for (final chapter in state.chapters)
          chapter.copyWith(collapsed: false),
      ],
    );
  }

  void toggleAllChapters() {
    if (state.chapters.any((chapter) => chapter.collapsed)) {
      expandAllChapters();
    } else {
      collapseAllChapters();
    }
  }

  Future<void> openTextFile() async {
    final service = await _fileService;
    final document = await service.pickTextDocument();
    if (document != null) {
      state = document;
    }
  }

  Future<void> openLibraryBook(LibraryBook book) async {
    if (book.format != BookFormat.txt) {
      return;
    }
    final service = await _fileService;
    state = await service.openLibraryBook(book);
  }

  Future<void> save() async {
    final service = await _fileService;
    final path = await service.saveDocument(state);
    if (path != null) {
      final history = await service.listHistorySnapshots(
        title: state.title,
        filePath: path,
      );
      state = state.copyWith(filePath: path, dirty: false, history: history);
    }
  }

  Future<void> createHistorySnapshot() async {
    final service = await _fileService;
    final history = await service.createHistorySnapshot(state);
    state = state.copyWith(history: history);
  }

  Future<void> restoreHistorySnapshot(EditorHistorySnapshot snapshot) async {
    final service = await _fileService;
    final content = await service.readHistorySnapshot(snapshot);
    if (content == null) {
      return;
    }
    _setContent(content, dirty: true, pendingOffset: 0);
  }

  void toggleSearch() {
    state = state.copyWith(
      search: state.search.copyWith(visible: !state.search.visible),
    );
  }

  void showSearch() {
    state = state.copyWith(search: state.search.copyWith(visible: true));
  }

  void updateSearchQuery(String query) {
    state = state.copyWith(search: _buildSearch(query: query));
  }

  void updateReplacement(String replacement) {
    state = state.copyWith(
      search: state.search.copyWith(replacement: replacement),
    );
  }

  void setSearchMode(EditorSearchMode mode) {
    state = state.copyWith(search: _buildSearch(mode: mode));
  }

  void setSearchRegex(bool value) {
    setSearchMode(value ? EditorSearchMode.regex : EditorSearchMode.normal);
  }

  void setSearchMatchCase(bool value) {
    state = state.copyWith(search: _buildSearch(matchCase: value));
  }

  void setSearchWholeWord(bool value) {
    state = state.copyWith(search: _buildSearch(wholeWord: value));
  }

  void setSearchScope(EditorSearchScope scope) {
    state = state.copyWith(search: _buildSearch(scope: scope));
  }

  void nextSearchMatch() {
    final search = state.search;
    if (search.matches.isEmpty) {
      return;
    }
    final next = (search.currentIndex + 1) % search.matches.length;
    state = state.copyWith(
      search: search.copyWith(currentIndex: next),
      pendingScrollOffset: search.matches[next].offset,
    );
  }

  void previousSearchMatch() {
    final search = state.search;
    if (search.matches.isEmpty) {
      return;
    }
    final previous = search.currentIndex <= 0
        ? search.matches.length - 1
        : search.currentIndex - 1;
    state = state.copyWith(
      search: search.copyWith(currentIndex: previous),
      pendingScrollOffset: search.matches[previous].offset,
    );
  }

  void replaceCurrent() {
    final search = state.search;
    final match = search.currentMatch;
    if (match == null ||
        search.query.isEmpty ||
        search.scope == EditorSearchScope.toc) {
      return;
    }
    final nextContent = state.content.replaceRange(
      match.offset,
      match.offset + match.length,
      _preparedReplacement(search),
    );
    _setContent(nextContent, dirty: true, pendingOffset: match.offset);
  }

  void replaceAll() {
    final search = state.search;
    if (search.query.isEmpty ||
        search.matches.isEmpty ||
        search.scope == EditorSearchScope.toc) {
      return;
    }

    final matcher = _buildMatcher(search);
    final nextContent = state.content.replaceAll(
      matcher,
      _preparedReplacement(search),
    );
    _setContent(nextContent, dirty: true);
  }

  EditorDocument rebuildToc({bool keepSelection = false}) {
    final chapters = _safeScanChapters(state.content, state.toc);
    final selected = state.selectedChapterId;
    state = state.copyWith(
      chapters: chapters,
      selectedChapterId:
          keepSelection && chapters.any((chapter) => chapter.id == selected)
              ? selected
              : chapters.isEmpty
                  ? null
                  : chapters.first.id,
    );
    return state;
  }

  void updateTocPattern(String pattern) {
    final toc = state.toc.copyWith(pattern: pattern, useCustomPattern: true);
    state = state.copyWith(toc: toc);
  }

  void setCustomTocPattern(bool value) {
    final toc = state.toc.copyWith(useCustomPattern: value, clearError: true);
    state = state.copyWith(toc: toc);
    rebuildToc();
  }

  void applyTocPattern() {
    final error = _scanner.validateTocPattern(state.toc.pattern);
    if (error != null) {
      state = state.copyWith(toc: state.toc.copyWith(error: error));
      return;
    }
    final toc = state.toc.copyWith(clearError: true);
    final chapters = _safeScanChapters(state.content, toc);
    state = state.copyWith(
      toc: toc,
      chapters: chapters,
      selectedChapterId: chapters.isEmpty ? null : chapters.first.id,
    );
  }

  void clearPendingScrollOffset() {
    state = state.copyWith(clearPendingScrollOffset: true);
  }

  void markSaved() {
    state = state.copyWith(dirty: false);
  }

  void updateFontSize(double value) {
    state = state.copyWith(
      display: state.display.copyWith(fontSize: value.clamp(12, 32)),
    );
  }

  void updateLineHeight(double value) {
    state = state.copyWith(
      display: state.display.copyWith(lineHeight: value.clamp(1.2, 2.4)),
    );
  }

  void setWordWrap(bool value) {
    state = state.copyWith(display: state.display.copyWith(wordWrap: value));
  }

  void setShowWhitespace(bool value) {
    state = state.copyWith(
      display: state.display.copyWith(showWhitespace: value),
    );
  }

  void setShowLineBreaks(bool value) {
    state = state.copyWith(
      display: state.display.copyWith(showLineBreaks: value),
    );
  }

  void updateWordCountMin(double value) {
    state = state.copyWith(
      display: state.display.copyWith(wordCountMin: value.round()),
    );
    runChapterCheck();
  }

  void updateWordCountMax(double value) {
    state = state.copyWith(
      display: state.display.copyWith(wordCountMax: value.round()),
    );
    runChapterCheck();
  }

  void runChapterCheck() {
    final report = _buildCheckReport();
    state = state.copyWith(checkReport: report);
  }

  void jumpToCheckIssue(EditorCheckIssue issue) {
    state = state.copyWith(pendingScrollOffset: issue.offset);
  }

  void _setContent(
    String value, {
    required bool dirty,
    int? pendingOffset,
    bool rescanImmediately = true,
  }) {
    final chapters = rescanImmediately
        ? _safeScanChapters(value, state.toc)
        : state.chapters;
    final selected = state.selectedChapterId;
    state = state.copyWith(
      content: value,
      dirty: dirty,
      chapters: chapters,
      selectedChapterId: chapters.any((chapter) => chapter.id == selected)
          ? selected
          : chapters.isEmpty
              ? null
              : chapters.first.id,
      pendingScrollOffset: pendingOffset,
      search: state.search.query.trim().isEmpty
          ? state.search
          : _buildSearch(content: value),
      checkReport: rescanImmediately
          ? _buildCheckReport(content: value, chapters: chapters)
          : state.checkReport,
    );
  }

  void _refreshTextAnalysis() {
    state = state.copyWith(
      search: state.search.query.trim().isEmpty
          ? state.search
          : _buildSearch(content: state.content),
      checkReport: _buildCheckReport(),
    );
  }

  List<EditorChapter> _safeScanChapters(String content, EditorTocState toc) {
    try {
      return _scanner.scanChapters(content, toc: toc);
    } on FormatException catch (error) {
      state = state.copyWith(toc: toc.copyWith(error: error.message));
      return state.chapters;
    }
  }

  bool _chapterHasChildren(EditorChapter chapter) {
    final index = state.chapters.indexWhere((item) => item.id == chapter.id);
    if (index < 0 || index == state.chapters.length - 1) {
      return false;
    }
    return state.chapters[index + 1].level > chapter.level;
  }

  RegExp _buildMatcher(EditorSearchState search) {
    var pattern = switch (search.mode) {
      EditorSearchMode.regex => search.query,
      EditorSearchMode.extended => RegExp.escape(_decodeExtended(search.query)),
      EditorSearchMode.normal => RegExp.escape(search.query),
    };

    if (search.wholeWord && search.mode != EditorSearchMode.regex) {
      pattern = r'\b' + pattern + r'\b';
    }

    return RegExp(
      pattern,
      caseSensitive: search.matchCase,
      multiLine: true,
    );
  }

  String _preparedReplacement(EditorSearchState search) {
    return search.mode == EditorSearchMode.extended
        ? _decodeExtended(search.replacement)
        : search.replacement;
  }

  String _decodeExtended(String value) {
    return value
        .replaceAll(r'\n', '\n')
        .replaceAll(r'\t', '\t')
        .replaceAll(r'\r', '\r');
  }

  EditorSearchState _buildSearch({
    String? content,
    String? query,
    EditorSearchMode? mode,
    bool? matchCase,
    bool? wholeWord,
    EditorSearchScope? scope,
  }) {
    final nextQuery = query ?? state.search.query;
    final nextMode = mode ?? state.search.mode;
    final nextMatchCase = matchCase ?? state.search.matchCase;
    final nextWholeWord = wholeWord ?? state.search.wholeWord;
    final nextScope = scope ?? state.search.scope;
    final trimmed = nextQuery.trim();
    if (trimmed.isEmpty) {
      return EditorSearchState.empty.copyWith(
        query: nextQuery,
        replacement: state.search.replacement,
        visible: state.search.visible,
        mode: nextMode,
        matchCase: nextMatchCase,
        wholeWord: nextWholeWord,
        scope: nextScope,
      );
    }

    final probe = state.search.copyWith(
      query: nextQuery,
      mode: nextMode,
      matchCase: nextMatchCase,
      wholeWord: nextWholeWord,
      scope: nextScope,
      clearError: true,
    );

    try {
      final matches = nextScope == EditorSearchScope.toc
          ? _searchToc(probe)
          : _searchContent(content ?? state.content, probe);
      return EditorSearchState(
        query: nextQuery,
        replacement: state.search.replacement,
        matches: matches,
        currentIndex: matches.isEmpty ? -1 : 0,
        visible: state.search.visible,
        replaceVisible: true,
        mode: nextMode,
        matchCase: nextMatchCase,
        wholeWord: nextWholeWord,
        scope: nextScope,
      );
    } on FormatException catch (error) {
      return state.search.copyWith(
        query: nextQuery,
        matches: const [],
        currentIndex: -1,
        visible: true,
        mode: nextMode,
        matchCase: nextMatchCase,
        wholeWord: nextWholeWord,
        scope: nextScope,
        error: error.message,
      );
    }
  }

  List<EditorSearchMatch> _searchContent(
    String content,
    EditorSearchState search,
  ) {
    final matcher = _buildMatcher(search);
    return [
      for (final match in matcher.allMatches(content))
        EditorSearchMatch(
          offset: match.start,
          length: match.end - match.start,
        ),
    ];
  }

  List<EditorSearchMatch> _searchToc(EditorSearchState search) {
    final matcher = _buildMatcher(search);
    return [
      for (final chapter in state.chapters)
        if (matcher.hasMatch(chapter.title))
          EditorSearchMatch(
            offset: chapter.offset,
            length: chapter.title.length,
          ),
    ];
  }

  EditorCheckReport _buildCheckReport({
    String? content,
    List<EditorChapter>? chapters,
  }) {
    final text = content ?? state.content;
    final toc = chapters ?? state.chapters;
    if (toc.isEmpty) {
      return EditorCheckReport.empty;
    }

    final sequenceIssues = <EditorCheckIssue>[];
    final titleIssues = <EditorCheckIssue>[];
    final wordCountIssues = <EditorCheckIssue>[];
    int? lastChapterNumber;

    for (var index = 0; index < toc.length; index += 1) {
      final chapter = toc[index];
      final title = chapter.title.trim();
      if (title.length <= 2 ||
          RegExp(r'^[第\d零〇一二两三四五六七八九十百千万]+[卷章]$').hasMatch(title)) {
        titleIssues.add(
          EditorCheckIssue(
            title: title,
            message: '标题缺少正文描述',
            lineNumber: chapter.lineNumber,
            offset: chapter.offset,
          ),
        );
      }

      final chapterNumber = _extractChapterNumber(title);
      if (chapter.level > 1 && chapterNumber != null) {
        if (lastChapterNumber != null &&
            chapterNumber != lastChapterNumber + 1) {
          sequenceIssues.add(
            EditorCheckIssue(
              title: title,
              message: '章节序号可能不连续：上一章 $lastChapterNumber，当前 $chapterNumber',
              lineNumber: chapter.lineNumber,
              offset: chapter.offset,
            ),
          );
        }
        lastChapterNumber = chapterNumber;
      }

      if (chapter.level > 1) {
        final start = chapter.offset;
        final end =
            index + 1 < toc.length ? toc[index + 1].offset : text.length;
        final wordCount = text
            .substring(start.clamp(0, text.length), end.clamp(0, text.length))
            .replaceAll(RegExp(r'\s+'), '')
            .runes
            .length;
        if (wordCount < state.display.wordCountMin ||
            wordCount > state.display.wordCountMax) {
          wordCountIssues.add(
            EditorCheckIssue(
              title: title,
              message:
                  '字数 $wordCount，不在 ${state.display.wordCountMin}-${state.display.wordCountMax}',
              lineNumber: chapter.lineNumber,
              offset: chapter.offset,
            ),
          );
        }
      }
    }

    return EditorCheckReport(
      sequenceIssues: sequenceIssues,
      titleIssues: titleIssues,
      wordCountIssues: wordCountIssues,
    );
  }

  int? _extractChapterNumber(String title) {
    final match = RegExp(r'^(?:第)?([0-9零〇一二两三四五六七八九十百千万]+)章').firstMatch(title);
    if (match == null) {
      return null;
    }
    final raw = match.group(1)!;
    return int.tryParse(raw) ?? _chineseNumberToInt(raw);
  }

  int? _chineseNumberToInt(String value) {
    const digits = {
      '零': 0,
      '〇': 0,
      '一': 1,
      '二': 2,
      '两': 2,
      '三': 3,
      '四': 4,
      '五': 5,
      '六': 6,
      '七': 7,
      '八': 8,
      '九': 9,
    };
    const units = {
      '十': 10,
      '百': 100,
      '千': 1000,
      '万': 10000,
    };
    var result = 0;
    var section = 0;
    var number = 0;
    for (final codePoint in value.runes) {
      final char = String.fromCharCode(codePoint);
      final digit = digits[char];
      if (digit != null) {
        number = digit;
        continue;
      }
      final unit = units[char];
      if (unit == null) {
        return null;
      }
      if (unit == 10000) {
        section = (section + number) * unit;
        result += section;
        section = 0;
      } else {
        section += (number == 0 ? 1 : number) * unit;
      }
      number = 0;
    }
    return result + section + number;
  }
}

final _sampleChapters = const EditorFileService().scanChapters(_sampleContent);

const _sampleContent = '''
第一卷 风起
这里是 Flutter 重构版的编辑器工作区。当前内容来自内存状态，后续会替换为真实 TXT/EPUB 文件读取。
第一章 编辑器布局

目录、正文、搜索、历史版本会保持独立组件，避免像旧项目后期那样所有逻辑挤在单个页面里。
第二章 校对流程

错别字自动应用，没有自动通过的建议进入人工审核。审批结果、建议列表和日志会共享同一份校对任务状态。
''';
