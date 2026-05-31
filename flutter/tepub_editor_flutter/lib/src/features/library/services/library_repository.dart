import 'dart:convert';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../core/storage/app_storage_paths.dart';
import '../models/library_book.dart';

final libraryRepositoryProvider =
    FutureProvider<LibraryRepository>((ref) async {
  final paths = await ref.watch(appStoragePathsProvider.future);
  return LibraryRepository(paths);
});

class LibraryRepository {
  const LibraryRepository(this.paths);

  final AppStoragePaths paths;

  Future<List<LibraryBook>> loadBooks() async {
    final file = paths.libraryIndexFile;
    if (!await file.exists()) {
      await saveBooks(defaultLibraryBooks);
      return defaultLibraryBooks;
    }

    final text = await file.readAsString();
    final json = jsonDecode(text) as Map<String, Object?>;
    final books = json['books'] as List<dynamic>? ?? const [];
    return [
      for (final item in books)
        LibraryBook.fromJson(item as Map<String, Object?>),
    ];
  }

  Future<void> saveBooks(List<LibraryBook> books) async {
    await paths.ensureCreated();
    const encoder = JsonEncoder.withIndent('  ');
    await paths.libraryIndexFile.writeAsString(
      encoder.convert({
        'version': 1,
        'books': [for (final book in books) book.toJson()],
      }),
    );
  }
}

final defaultLibraryBooks = [
  LibraryBook(
    id: 'book-001',
    title: '示例长篇小说',
    author: '本地书库',
    format: BookFormat.txt,
    updatedAt: DateTime(2026, 5, 24, 14, 12),
    chapterCount: 128,
    progress: 0.42,
  ),
  LibraryBook(
    id: 'book-002',
    title: 'EPUB 排版测试集',
    author: '迁移样例',
    format: BookFormat.epub,
    updatedAt: DateTime(2026, 5, 21, 9, 30),
    chapterCount: 36,
    progress: 0.76,
  ),
  LibraryBook(
    id: 'book-003',
    title: '待校对文稿',
    author: 'AI 校对样例',
    format: BookFormat.txt,
    updatedAt: DateTime(2026, 5, 18, 22, 8),
    chapterCount: 12,
    progress: 0.18,
  ),
];
