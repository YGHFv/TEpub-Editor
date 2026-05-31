class EpubTocRule {
  const EpubTocRule({
    required this.pattern,
    required this.level,
  });

  final String pattern;
  final int level;

  EpubTocRule copyWith({
    String? pattern,
    int? level,
  }) {
    return EpubTocRule(
      pattern: pattern ?? this.pattern,
      level: level ?? this.level,
    );
  }
}

class EpubChapter {
  const EpubChapter({
    required this.id,
    required this.title,
    required this.lineNumber,
    required this.level,
    required this.isMeta,
    required this.wordCount,
    this.collapsed = false,
  });

  final String id;
  final String title;
  final int lineNumber;
  final int level;
  final bool isMeta;
  final int wordCount;
  final bool collapsed;

  EpubChapter copyWith({
    String? id,
    String? title,
    int? lineNumber,
    int? level,
    bool? isMeta,
    int? wordCount,
    bool? collapsed,
  }) {
    return EpubChapter(
      id: id ?? this.id,
      title: title ?? this.title,
      lineNumber: lineNumber ?? this.lineNumber,
      level: level ?? this.level,
      isMeta: isMeta ?? this.isMeta,
      wordCount: wordCount ?? this.wordCount,
      collapsed: collapsed ?? this.collapsed,
    );
  }
}

class EpubBuildResult {
  const EpubBuildResult({
    required this.outputPath,
    required this.title,
    required this.chapterCount,
    required this.wordCount,
  });

  final String outputPath;
  final String title;
  final int chapterCount;
  final int wordCount;
}

class CoverSearchResult {
  const CoverSearchResult({
    required this.id,
    required this.title,
    required this.imageUrl,
    required this.pageUrl,
    required this.source,
    required this.preferred,
  });

  final String id;
  final String title;
  final String imageUrl;
  final String pageUrl;
  final String source;
  final bool preferred;
}

class EpubFontAsset {
  const EpubFontAsset({
    required this.family,
    required this.fileName,
    required this.path,
  });

  final String family;
  final String fileName;
  final String path;
}

class EpubProject {
  const EpubProject({
    required this.title,
    required this.author,
    required this.publisher,
    required this.date,
    required this.description,
    required this.tags,
    required this.mainCss,
    required this.fontCss,
    required this.uuid,
    required this.sourcePath,
    required this.sourceName,
    required this.content,
    required this.coverPath,
    required this.rules,
    required this.chapters,
    required this.status,
    required this.busy,
    required this.coverSearching,
    required this.coverResults,
    required this.fonts,
    this.result,
    this.error,
  });

  static const defaultRules = [
    EpubTocRule(
      pattern: r'^(?:第)?[0-9零〇一二两三四五六七八九十百千万]+卷\s*.+$',
      level: 1,
    ),
    EpubTocRule(
      pattern: r'^(?:第)?[0-9零〇一二两三四五六七八九十百千万]+章\s*.+$',
      level: 3,
    ),
  ];

  static EpubProject empty() {
    return EpubProject(
      title: '',
      author: '',
      publisher: '',
      date: '',
      description: '',
      tags: const [],
      mainCss: '',
      fontCss: '',
      uuid: _newUuid(),
      sourcePath: '',
      sourceName: '',
      content: '',
      coverPath: '',
      rules: defaultRules,
      chapters: const [],
      status: '选择 TXT、MD 或 HTML 文件后预览目录。',
      busy: false,
      coverSearching: false,
      coverResults: const [],
      fonts: const [],
    );
  }

  final String title;
  final String author;
  final String publisher;
  final String date;
  final String description;
  final List<String> tags;
  final String mainCss;
  final String fontCss;
  final String uuid;
  final String sourcePath;
  final String sourceName;
  final String content;
  final String coverPath;
  final List<EpubTocRule> rules;
  final List<EpubChapter> chapters;
  final String status;
  final bool busy;
  final bool coverSearching;
  final List<CoverSearchResult> coverResults;
  final List<EpubFontAsset> fonts;
  final EpubBuildResult? result;
  final String? error;

  int get wordCount => content.trim().runes.length;
  bool get hasSource => sourcePath.isNotEmpty && content.trim().isNotEmpty;

  EpubProject copyWith({
    String? title,
    String? author,
    String? publisher,
    String? date,
    String? description,
    List<String>? tags,
    String? mainCss,
    String? fontCss,
    String? uuid,
    String? sourcePath,
    String? sourceName,
    String? content,
    String? coverPath,
    List<EpubTocRule>? rules,
    List<EpubChapter>? chapters,
    String? status,
    bool? busy,
    bool? coverSearching,
    List<CoverSearchResult>? coverResults,
    List<EpubFontAsset>? fonts,
    EpubBuildResult? result,
    String? error,
    bool clearResult = false,
    bool clearError = false,
  }) {
    return EpubProject(
      title: title ?? this.title,
      author: author ?? this.author,
      publisher: publisher ?? this.publisher,
      date: date ?? this.date,
      description: description ?? this.description,
      tags: tags ?? this.tags,
      mainCss: mainCss ?? this.mainCss,
      fontCss: fontCss ?? this.fontCss,
      uuid: uuid ?? this.uuid,
      sourcePath: sourcePath ?? this.sourcePath,
      sourceName: sourceName ?? this.sourceName,
      content: content ?? this.content,
      coverPath: coverPath ?? this.coverPath,
      rules: rules ?? this.rules,
      chapters: chapters ?? this.chapters,
      status: status ?? this.status,
      busy: busy ?? this.busy,
      coverSearching: coverSearching ?? this.coverSearching,
      coverResults: coverResults ?? this.coverResults,
      fonts: fonts ?? this.fonts,
      result: clearResult ? null : result ?? this.result,
      error: clearError ? null : error ?? this.error,
    );
  }

  static String _newUuid() {
    final now = DateTime.now().microsecondsSinceEpoch;
    return 'tepub-${now.toRadixString(16)}';
  }
}
