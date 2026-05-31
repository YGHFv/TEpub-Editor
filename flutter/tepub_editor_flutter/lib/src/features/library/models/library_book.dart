enum BookFormat { txt, epub }

class LibraryBook {
  const LibraryBook({
    required this.id,
    required this.title,
    required this.author,
    required this.format,
    required this.updatedAt,
    required this.chapterCount,
    required this.progress,
    this.sourcePath,
    this.fileSize,
  });

  final String id;
  final String title;
  final String author;
  final BookFormat format;
  final DateTime updatedAt;
  final int chapterCount;
  final double progress;
  final String? sourcePath;
  final int? fileSize;

  String get formatLabel => switch (format) {
        BookFormat.txt => 'TXT',
        BookFormat.epub => 'EPUB',
      };

  factory LibraryBook.fromJson(Map<String, Object?> json) {
    return LibraryBook(
      id: json['id'] as String,
      title: json['title'] as String? ?? '未命名图书',
      author: json['author'] as String? ?? '未知作者',
      format: BookFormat.values.byName(json['format'] as String? ?? 'txt'),
      updatedAt: DateTime.tryParse(json['updatedAt'] as String? ?? '') ??
          DateTime.now(),
      chapterCount: json['chapterCount'] as int? ?? 0,
      progress: (json['progress'] as num?)?.toDouble() ?? 0,
      sourcePath: json['sourcePath'] as String?,
      fileSize: json['fileSize'] as int?,
    );
  }

  Map<String, Object?> toJson() {
    return {
      'id': id,
      'title': title,
      'author': author,
      'format': format.name,
      'updatedAt': updatedAt.toIso8601String(),
      'chapterCount': chapterCount,
      'progress': progress,
      'sourcePath': sourcePath,
      'fileSize': fileSize,
    };
  }
}
