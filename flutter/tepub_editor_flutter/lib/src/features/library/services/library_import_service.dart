import 'dart:io';

import 'package:file_picker/file_picker.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:path/path.dart' as p;

import '../models/library_book.dart';

final libraryImportServiceProvider = Provider<LibraryImportService>((ref) {
  return const LibraryImportService();
});

class LibraryImportService {
  const LibraryImportService();

  Future<List<LibraryBook>> pickBooks() async {
    final result = await FilePicker.platform.pickFiles(
      allowMultiple: true,
      type: FileType.custom,
      allowedExtensions: const ['txt', 'epub'],
    );

    if (result == null) {
      return const [];
    }

    final now = DateTime.now();
    final books = <LibraryBook>[];
    for (final file in result.files) {
      final path = file.path;
      if (path == null) {
        continue;
      }

      final stat = await File(path).stat();
      final extension = p.extension(path).toLowerCase();
      final format = extension == '.epub' ? BookFormat.epub : BookFormat.txt;

      books.add(
        LibraryBook(
          id: 'file-${now.microsecondsSinceEpoch}-${books.length}',
          title: p.basenameWithoutExtension(path),
          author: '本地文件',
          format: format,
          updatedAt: stat.modified,
          chapterCount: 0,
          progress: 0,
          sourcePath: path,
          fileSize: stat.size,
        ),
      );
    }

    return books;
  }
}
