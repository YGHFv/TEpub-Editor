import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../ui/widgets/app_page.dart';
import '../../ui/widgets/app_surface.dart';
import '../../ui/widgets/responsive_two_pane.dart';
import 'providers/library_controller.dart';
import 'widgets/library_book_list.dart';
import 'widgets/library_detail_panel.dart';

class LibraryPage extends ConsumerWidget {
  const LibraryPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final controller = ref.read(libraryControllerProvider.notifier);

    return AppPage(
      title: '书库',
      subtitle: '使用重构版独立书库索引、缓存和配置目录，可导入 TXT / EPUB，不影响现有打包版本数据。',
      actions: [
        FilledButton.icon(
          onPressed: controller.importBooks,
          icon: const Icon(Icons.add),
          label: const Text('添加图书'),
        ),
      ],
      child: ResponsiveTwoPane(
        sideWidth: 420,
        breakpoint: 980,
        side: AppSurface(
          child: Column(
            children: [
              TextField(
                onChanged: controller.setQuery,
                decoration: const InputDecoration(
                  prefixIcon: Icon(Icons.search),
                  hintText: '搜索书名、作者或格式',
                ),
              ),
              const SizedBox(height: 14),
              const Expanded(child: LibraryBookList()),
            ],
          ),
        ),
        body: const LibraryDetailPanel(),
      ),
    );
  }
}
