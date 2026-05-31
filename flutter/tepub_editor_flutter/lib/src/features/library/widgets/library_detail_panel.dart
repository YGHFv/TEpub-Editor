import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../../ui/theme/app_theme.dart';
import '../../../ui/widgets/app_surface.dart';
import '../models/library_book.dart';
import '../providers/library_controller.dart';

class LibraryDetailPanel extends ConsumerWidget {
  const LibraryDetailPanel({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final book = ref.watch(libraryControllerProvider).selectedBook;

    if (book == null) {
      return const AppSurface(child: Center(child: Text('请选择图书')));
    }

    return AppSurface(
      child: ListView(
        padding: EdgeInsets.zero,
        children: [
          Text(
            book.title,
            style: Theme.of(context).textTheme.headlineSmall?.copyWith(
                  fontWeight: FontWeight.w800,
                  color: AppTheme.ink,
                ),
          ),
          const SizedBox(height: 8),
          Text(
            '${book.author} · ${book.formatLabel}',
            style: const TextStyle(color: AppTheme.muted),
          ),
          const SizedBox(height: 18),
          Wrap(
            spacing: 10,
            runSpacing: 10,
            children: [
              FilledButton.icon(
                onPressed: book.format == BookFormat.txt
                    ? () => context.go('/editor', extra: book)
                    : null,
                icon: const Icon(Icons.edit_note_outlined),
                label: const Text('打开编辑'),
              ),
              OutlinedButton.icon(
                onPressed: () => context.go('/proofing', extra: book),
                icon: const Icon(Icons.fact_check_outlined),
                label: const Text('智能校对'),
              ),
              OutlinedButton.icon(
                onPressed: () {},
                icon: const Icon(Icons.info_outline),
                label: const Text('元数据'),
              ),
            ],
          ),
          const SizedBox(height: 28),
          _Metric(label: '章节数', value: '${book.chapterCount}'),
          _Metric(label: '阅读进度', value: '${(book.progress * 100).round()}%'),
          _Metric(label: '更新时间', value: _formatDate(book.updatedAt)),
          if (book.fileSize != null)
            _Metric(label: '文件大小', value: _formatFileSize(book.fileSize!)),
          if (book.sourcePath != null)
            _Metric(label: '来源路径', value: book.sourcePath!),
        ],
      ),
    );
  }

  String _formatDate(DateTime value) {
    return '${value.year}-${value.month.toString().padLeft(2, '0')}-'
        '${value.day.toString().padLeft(2, '0')} '
        '${value.hour.toString().padLeft(2, '0')}:'
        '${value.minute.toString().padLeft(2, '0')}';
  }

  String _formatFileSize(int size) {
    if (size < 1024) {
      return '$size B';
    }
    if (size < 1024 * 1024) {
      return '${(size / 1024).toStringAsFixed(1)} KB';
    }
    return '${(size / 1024 / 1024).toStringAsFixed(1)} MB';
  }
}

class _Metric extends StatelessWidget {
  const _Metric({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 14),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(
            width: 82,
            child: Text(label, style: const TextStyle(color: AppTheme.muted)),
          ),
          Expanded(
            child: SelectableText(
              value,
              style: const TextStyle(fontWeight: FontWeight.w700),
            ),
          ),
        ],
      ),
    );
  }
}
