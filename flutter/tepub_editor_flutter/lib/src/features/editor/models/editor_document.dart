enum EditorSearchScope {
  content,
  toc,
}

enum EditorSearchMode {
  normal,
  regex,
  extended,
}

class EditorChapter {
  const EditorChapter({
    required this.id,
    required this.title,
    required this.preview,
    required this.offset,
    required this.lineNumber,
    this.level = 1,
    this.collapsed = false,
  });

  final String id;
  final String title;
  final String preview;
  final int offset;
  final int lineNumber;
  final int level;
  final bool collapsed;

  EditorChapter copyWith({
    String? id,
    String? title,
    String? preview,
    int? offset,
    int? lineNumber,
    int? level,
    bool? collapsed,
  }) {
    return EditorChapter(
      id: id ?? this.id,
      title: title ?? this.title,
      preview: preview ?? this.preview,
      offset: offset ?? this.offset,
      lineNumber: lineNumber ?? this.lineNumber,
      level: level ?? this.level,
      collapsed: collapsed ?? this.collapsed,
    );
  }
}

class EditorHistorySnapshot {
  const EditorHistorySnapshot({
    required this.id,
    required this.title,
    required this.createdAt,
    required this.contentLength,
    this.filePath,
  });

  final String id;
  final String title;
  final DateTime createdAt;
  final int contentLength;
  final String? filePath;

  factory EditorHistorySnapshot.fromJson(Map<String, Object?> json) {
    return EditorHistorySnapshot(
      id: json['id'] as String,
      title: json['title'] as String? ?? '未命名快照',
      createdAt: DateTime.tryParse(json['createdAt'] as String? ?? '') ??
          DateTime.fromMillisecondsSinceEpoch(0),
      contentLength: json['contentLength'] as int? ?? 0,
      filePath: json['filePath'] as String?,
    );
  }

  Map<String, Object?> toJson() {
    return {
      'id': id,
      'title': title,
      'createdAt': createdAt.toIso8601String(),
      'contentLength': contentLength,
      'filePath': filePath,
    };
  }
}

class EditorSearchMatch {
  const EditorSearchMatch({
    required this.offset,
    required this.length,
  });

  final int offset;
  final int length;
}

class EditorSearchState {
  const EditorSearchState({
    required this.query,
    required this.replacement,
    required this.matches,
    required this.currentIndex,
    required this.visible,
    required this.replaceVisible,
    required this.mode,
    required this.matchCase,
    required this.wholeWord,
    required this.scope,
    this.error,
  });

  final String query;
  final String replacement;
  final List<EditorSearchMatch> matches;
  final int currentIndex;
  final bool visible;
  final bool replaceVisible;
  final EditorSearchMode mode;
  final bool matchCase;
  final bool wholeWord;
  final EditorSearchScope scope;
  final String? error;

  static const empty = EditorSearchState(
    query: '',
    replacement: '',
    matches: [],
    currentIndex: -1,
    visible: false,
    replaceVisible: true,
    mode: EditorSearchMode.normal,
    matchCase: false,
    wholeWord: false,
    scope: EditorSearchScope.content,
  );

  bool get useRegex => mode == EditorSearchMode.regex;

  EditorSearchMatch? get currentMatch {
    if (currentIndex < 0 || currentIndex >= matches.length) {
      return null;
    }
    return matches[currentIndex];
  }

  int? get currentOffset => currentMatch?.offset;

  EditorSearchState copyWith({
    String? query,
    String? replacement,
    List<EditorSearchMatch>? matches,
    int? currentIndex,
    bool? visible,
    bool? replaceVisible,
    EditorSearchMode? mode,
    bool? matchCase,
    bool? wholeWord,
    EditorSearchScope? scope,
    String? error,
    bool clearError = false,
  }) {
    return EditorSearchState(
      query: query ?? this.query,
      replacement: replacement ?? this.replacement,
      matches: matches ?? this.matches,
      currentIndex: currentIndex ?? this.currentIndex,
      visible: visible ?? this.visible,
      replaceVisible: replaceVisible ?? this.replaceVisible,
      mode: mode ?? this.mode,
      matchCase: matchCase ?? this.matchCase,
      wholeWord: wholeWord ?? this.wholeWord,
      scope: scope ?? this.scope,
      error: clearError ? null : error ?? this.error,
    );
  }
}

class EditorTocState {
  const EditorTocState({
    required this.pattern,
    required this.useCustomPattern,
    this.error,
  });

  static const defaultPattern = r'^(?:第)?[0-9零〇一二两三四五六七八九十百千万]+[卷章]\s*.+$';

  static const empty = EditorTocState(
    pattern: defaultPattern,
    useCustomPattern: false,
  );

  final String pattern;
  final bool useCustomPattern;
  final String? error;

  EditorTocState copyWith({
    String? pattern,
    bool? useCustomPattern,
    String? error,
    bool clearError = false,
  }) {
    return EditorTocState(
      pattern: pattern ?? this.pattern,
      useCustomPattern: useCustomPattern ?? this.useCustomPattern,
      error: clearError ? null : error ?? this.error,
    );
  }
}

class EditorDisplaySettings {
  const EditorDisplaySettings({
    required this.fontSize,
    required this.lineHeight,
    required this.wordWrap,
    required this.showWhitespace,
    required this.showLineBreaks,
    required this.wordCountMin,
    required this.wordCountMax,
  });

  static const defaults = EditorDisplaySettings(
    fontSize: 17,
    lineHeight: 1.7,
    wordWrap: true,
    showWhitespace: false,
    showLineBreaks: false,
    wordCountMin: 1200,
    wordCountMax: 8000,
  );

  final double fontSize;
  final double lineHeight;
  final bool wordWrap;
  final bool showWhitespace;
  final bool showLineBreaks;
  final int wordCountMin;
  final int wordCountMax;

  EditorDisplaySettings copyWith({
    double? fontSize,
    double? lineHeight,
    bool? wordWrap,
    bool? showWhitespace,
    bool? showLineBreaks,
    int? wordCountMin,
    int? wordCountMax,
  }) {
    return EditorDisplaySettings(
      fontSize: fontSize ?? this.fontSize,
      lineHeight: lineHeight ?? this.lineHeight,
      wordWrap: wordWrap ?? this.wordWrap,
      showWhitespace: showWhitespace ?? this.showWhitespace,
      showLineBreaks: showLineBreaks ?? this.showLineBreaks,
      wordCountMin: wordCountMin ?? this.wordCountMin,
      wordCountMax: wordCountMax ?? this.wordCountMax,
    );
  }
}

class EditorCheckIssue {
  const EditorCheckIssue({
    required this.title,
    required this.message,
    required this.lineNumber,
    required this.offset,
  });

  final String title;
  final String message;
  final int lineNumber;
  final int offset;
}

class EditorCheckReport {
  const EditorCheckReport({
    required this.sequenceIssues,
    required this.titleIssues,
    required this.wordCountIssues,
  });

  static const empty = EditorCheckReport(
    sequenceIssues: [],
    titleIssues: [],
    wordCountIssues: [],
  );

  final List<EditorCheckIssue> sequenceIssues;
  final List<EditorCheckIssue> titleIssues;
  final List<EditorCheckIssue> wordCountIssues;

  int get totalIssues =>
      sequenceIssues.length + titleIssues.length + wordCountIssues.length;
}

class EditorDocument {
  const EditorDocument({
    required this.title,
    required this.content,
    required this.chapters,
    required this.selectedChapterId,
    required this.dirty,
    required this.search,
    required this.toc,
    required this.history,
    required this.display,
    required this.checkReport,
    this.filePath,
    this.pendingScrollOffset,
  });

  final String title;
  final String content;
  final List<EditorChapter> chapters;
  final String? selectedChapterId;
  final bool dirty;
  final String? filePath;
  final EditorSearchState search;
  final EditorTocState toc;
  final List<EditorHistorySnapshot> history;
  final EditorDisplaySettings display;
  final EditorCheckReport checkReport;
  final int? pendingScrollOffset;

  EditorChapter? get selectedChapter {
    for (final chapter in chapters) {
      if (chapter.id == selectedChapterId) {
        return chapter;
      }
    }
    return chapters.isEmpty ? null : chapters.first;
  }

  EditorDocument copyWith({
    String? title,
    String? content,
    List<EditorChapter>? chapters,
    String? selectedChapterId,
    bool? dirty,
    String? filePath,
    EditorSearchState? search,
    EditorTocState? toc,
    List<EditorHistorySnapshot>? history,
    EditorDisplaySettings? display,
    EditorCheckReport? checkReport,
    int? pendingScrollOffset,
    bool clearPendingScrollOffset = false,
  }) {
    return EditorDocument(
      title: title ?? this.title,
      content: content ?? this.content,
      chapters: chapters ?? this.chapters,
      selectedChapterId: selectedChapterId ?? this.selectedChapterId,
      dirty: dirty ?? this.dirty,
      filePath: filePath ?? this.filePath,
      search: search ?? this.search,
      toc: toc ?? this.toc,
      history: history ?? this.history,
      display: display ?? this.display,
      checkReport: checkReport ?? this.checkReport,
      pendingScrollOffset: clearPendingScrollOffset
          ? null
          : pendingScrollOffset ?? this.pendingScrollOffset,
    );
  }
}
