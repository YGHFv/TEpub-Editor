enum EpubEditorFileKind {
  text,
  image,
  font,
  binary,
}

class EpubEditorMetadata {
  const EpubEditorMetadata({
    required this.opfPath,
    required this.title,
    required this.author,
    required this.publisher,
    required this.description,
    required this.uuid,
    required this.subjects,
    required this.coverPath,
  });

  factory EpubEditorMetadata.empty() {
    return const EpubEditorMetadata(
      opfPath: '',
      title: '',
      author: '',
      publisher: '',
      description: '',
      uuid: '',
      subjects: [],
      coverPath: '',
    );
  }

  final String opfPath;
  final String title;
  final String author;
  final String publisher;
  final String description;
  final String uuid;
  final List<String> subjects;
  final String coverPath;

  EpubEditorMetadata copyWith({
    String? opfPath,
    String? title,
    String? author,
    String? publisher,
    String? description,
    String? uuid,
    List<String>? subjects,
    String? coverPath,
  }) {
    return EpubEditorMetadata(
      opfPath: opfPath ?? this.opfPath,
      title: title ?? this.title,
      author: author ?? this.author,
      publisher: publisher ?? this.publisher,
      description: description ?? this.description,
      uuid: uuid ?? this.uuid,
      subjects: subjects ?? this.subjects,
      coverPath: coverPath ?? this.coverPath,
    );
  }
}

class EpubEditorSearchHit {
  const EpubEditorSearchHit({
    required this.filePath,
    required this.offset,
    required this.preview,
  });

  final String filePath;
  final int offset;
  final String preview;
}

class EpubEditorFile {
  const EpubEditorFile({
    required this.path,
    required this.size,
    required this.kind,
    this.content,
    this.previewContent,
    this.titleHint,
    this.bytes,
    this.modified = false,
    this.deleted = false,
  });

  final String path;
  final int size;
  final EpubEditorFileKind kind;
  final String? content;
  final String? previewContent;
  final String? titleHint;
  final List<int>? bytes;
  final bool modified;
  final bool deleted;

  String get name {
    final normalized = path.replaceAll('\\', '/');
    final index = normalized.lastIndexOf('/');
    return index < 0 ? normalized : normalized.substring(index + 1);
  }

  bool get isText => kind == EpubEditorFileKind.text;

  EpubEditorFile copyWith({
    String? path,
    int? size,
    EpubEditorFileKind? kind,
    String? content,
    String? previewContent,
    String? titleHint,
    List<int>? bytes,
    bool? modified,
    bool? deleted,
  }) {
    return EpubEditorFile(
      path: path ?? this.path,
      size: size ?? this.size,
      kind: kind ?? this.kind,
      content: content ?? this.content,
      previewContent: previewContent ?? this.previewContent,
      titleHint: titleHint ?? this.titleHint,
      bytes: bytes ?? this.bytes,
      modified: modified ?? this.modified,
      deleted: deleted ?? this.deleted,
    );
  }
}

class EpubEditorDocument {
  const EpubEditorDocument({
    required this.epubPath,
    required this.files,
    required this.selectedPath,
    required this.status,
    required this.busy,
    required this.searchQuery,
    required this.replaceText,
    required this.searchRegex,
    required this.searchMatchCase,
    required this.searchAllFiles,
    required this.searchHits,
    required this.metadata,
    this.error,
  });

  factory EpubEditorDocument.empty() {
    return const EpubEditorDocument(
      epubPath: '',
      files: [],
      selectedPath: null,
      status: '选择 EPUB 后可编辑内部 HTML/CSS/XML 文件。',
      busy: false,
      searchQuery: '',
      replaceText: '',
      searchRegex: false,
      searchMatchCase: false,
      searchAllFiles: false,
      searchHits: [],
      metadata: EpubEditorMetadata(
        opfPath: '',
        title: '',
        author: '',
        publisher: '',
        description: '',
        uuid: '',
        subjects: [],
        coverPath: '',
      ),
    );
  }

  final String epubPath;
  final List<EpubEditorFile> files;
  final String? selectedPath;
  final String status;
  final bool busy;
  final String searchQuery;
  final String replaceText;
  final bool searchRegex;
  final bool searchMatchCase;
  final bool searchAllFiles;
  final List<EpubEditorSearchHit> searchHits;
  final EpubEditorMetadata metadata;
  final String? error;

  bool get hasDocument => epubPath.trim().isNotEmpty;
  bool get dirty => files.any((file) => file.modified || file.deleted);

  EpubEditorFile? get selectedFile {
    final path = selectedPath;
    if (path == null) {
      return null;
    }
    for (final file in files) {
      if (file.path == path && !file.deleted) {
        return file;
      }
    }
    return null;
  }

  List<EpubEditorFile> get visibleFiles =>
      files.where((file) => !file.deleted).toList(growable: false);

  EpubEditorDocument copyWith({
    String? epubPath,
    List<EpubEditorFile>? files,
    String? selectedPath,
    String? status,
    bool? busy,
    String? searchQuery,
    String? replaceText,
    bool? searchRegex,
    bool? searchMatchCase,
    bool? searchAllFiles,
    List<EpubEditorSearchHit>? searchHits,
    EpubEditorMetadata? metadata,
    String? error,
    bool clearError = false,
  }) {
    return EpubEditorDocument(
      epubPath: epubPath ?? this.epubPath,
      files: files ?? this.files,
      selectedPath: selectedPath ?? this.selectedPath,
      status: status ?? this.status,
      busy: busy ?? this.busy,
      searchQuery: searchQuery ?? this.searchQuery,
      replaceText: replaceText ?? this.replaceText,
      searchRegex: searchRegex ?? this.searchRegex,
      searchMatchCase: searchMatchCase ?? this.searchMatchCase,
      searchAllFiles: searchAllFiles ?? this.searchAllFiles,
      searchHits: searchHits ?? this.searchHits,
      metadata: metadata ?? this.metadata,
      error: clearError ? null : error ?? this.error,
    );
  }
}
