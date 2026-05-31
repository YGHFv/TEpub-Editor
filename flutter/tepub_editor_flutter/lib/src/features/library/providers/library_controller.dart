import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/library_book.dart';
import '../services/library_import_service.dart';
import '../services/library_repository.dart';

final libraryControllerProvider =
    StateNotifierProvider<LibraryController, LibraryState>((ref) {
  return LibraryController(
    ref.read(libraryImportServiceProvider),
    ref.read(libraryRepositoryProvider.future),
  );
});

class LibraryState {
  const LibraryState({
    required this.books,
    required this.query,
    required this.selectedBookId,
    required this.loading,
    this.error,
  });

  final List<LibraryBook> books;
  final String query;
  final String? selectedBookId;
  final bool loading;
  final String? error;

  List<LibraryBook> get filteredBooks {
    final keyword = query.trim().toLowerCase();
    if (keyword.isEmpty) {
      return books;
    }
    return books.where((book) {
      return book.title.toLowerCase().contains(keyword) ||
          book.author.toLowerCase().contains(keyword) ||
          book.formatLabel.toLowerCase().contains(keyword);
    }).toList();
  }

  LibraryBook? get selectedBook {
    for (final book in books) {
      if (book.id == selectedBookId) {
        return book;
      }
    }
    return books.isEmpty ? null : books.first;
  }

  LibraryState copyWith({
    List<LibraryBook>? books,
    String? query,
    String? selectedBookId,
    bool? loading,
    String? error,
  }) {
    return LibraryState(
      books: books ?? this.books,
      query: query ?? this.query,
      selectedBookId: selectedBookId ?? this.selectedBookId,
      loading: loading ?? this.loading,
      error: error,
    );
  }
}

class LibraryController extends StateNotifier<LibraryState> {
  LibraryController(this._importService, this._repositoryFuture)
      : super(
          const LibraryState(
            books: [],
            query: '',
            selectedBookId: null,
            loading: true,
          ),
        ) {
    _load();
  }

  final LibraryImportService _importService;
  final Future<LibraryRepository> _repositoryFuture;

  void selectBook(String id) {
    state = state.copyWith(selectedBookId: id);
  }

  void setQuery(String query) {
    state = state.copyWith(query: query);
  }

  Future<void> importBooks() async {
    final books = await _importService.pickBooks();
    if (books.isEmpty) {
      return;
    }
    final nextBooks = [...books, ...state.books];
    state = state.copyWith(
      books: nextBooks,
      selectedBookId: books.first.id,
    );
    await _save(nextBooks);
  }

  Future<void> addPlaceholderBook() async {
    final now = DateTime.now();
    final book = LibraryBook(
      id: 'draft-${now.microsecondsSinceEpoch}',
      title: '新导入图书',
      author: '未知作者',
      format: BookFormat.txt,
      updatedAt: now,
      chapterCount: 0,
      progress: 0,
    );
    final nextBooks = [book, ...state.books];
    state = state.copyWith(
      books: nextBooks,
      selectedBookId: book.id,
    );
    await _save(nextBooks);
  }

  Future<void> _load() async {
    try {
      final repository = await _repositoryFuture;
      final books = await repository.loadBooks();
      state = state.copyWith(
        books: books,
        selectedBookId: books.isEmpty ? null : books.first.id,
        loading: false,
      );
    } catch (error) {
      state = state.copyWith(
        books: defaultLibraryBooks,
        selectedBookId: defaultLibraryBooks.first.id,
        loading: false,
        error: '$error',
      );
    }
  }

  Future<void> _save(List<LibraryBook> books) async {
    try {
      final repository = await _repositoryFuture;
      await repository.saveBooks(books);
    } catch (error) {
      state = state.copyWith(error: '$error');
    }
  }
}
