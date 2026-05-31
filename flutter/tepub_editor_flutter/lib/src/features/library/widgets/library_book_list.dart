import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../ui/theme/app_theme.dart';
import '../models/library_book.dart';
import '../providers/library_controller.dart';

class LibraryBookList extends ConsumerWidget {
  const LibraryBookList({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final state = ref.watch(libraryControllerProvider);
    final controller = ref.read(libraryControllerProvider.notifier);
    final books = state.filteredBooks;

    if (state.loading) {
      return const Center(child: CircularProgressIndicator());
    }

    if (books.isEmpty) {
      return Center(
        child: Text(
          state.books.isEmpty ? '还没有图书，点击右上角添加 TXT / EPUB。' : '没有匹配的图书',
        ),
      );
    }

    return ListView.separated(
      padding: EdgeInsets.zero,
      itemCount: books.length,
      separatorBuilder: (context, index) => const SizedBox(height: 10),
      itemBuilder: (context, index) {
        final book = books[index];
        final selected = book.id == state.selectedBook?.id;
        return _BookTile(
          book: book,
          selected: selected,
          onTap: () => controller.selectBook(book.id),
        );
      },
    );
  }
}

class _BookTile extends StatelessWidget {
  const _BookTile({
    required this.book,
    required this.selected,
    required this.onTap,
  });

  final LibraryBook book;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return Material(
      color: selected ? AppTheme.panelSoft : Colors.transparent,
      borderRadius: BorderRadius.circular(18),
      child: InkWell(
        borderRadius: BorderRadius.circular(18),
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.all(12),
          child: Row(
            children: [
              Container(
                width: 54,
                height: 72,
                decoration: BoxDecoration(
                  color: selected ? AppTheme.brand : AppTheme.border,
                  borderRadius: BorderRadius.circular(12),
                ),
                child: Center(
                  child: Text(
                    book.formatLabel,
                    style: TextStyle(
                      color: selected ? Colors.white : AppTheme.muted,
                      fontWeight: FontWeight.w800,
                    ),
                  ),
                ),
              ),
              const SizedBox(width: 14),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      book.title,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: Theme.of(context).textTheme.titleMedium?.copyWith(
                            color: AppTheme.ink,
                            fontWeight: FontWeight.w800,
                          ),
                    ),
                    const SizedBox(height: 4),
                    Text(
                      '${book.author} · ${book.chapterCount} 章',
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: const TextStyle(color: AppTheme.muted),
                    ),
                    const SizedBox(height: 10),
                    LinearProgressIndicator(
                      value: book.progress,
                      minHeight: 5,
                      borderRadius: BorderRadius.circular(999),
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
