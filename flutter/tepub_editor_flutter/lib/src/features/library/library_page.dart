import 'package:flutter/material.dart';

import 'widgets/library_empty_panel.dart';
import '../../ui/widgets/app_page.dart';

class LibraryPage extends StatelessWidget {
  const LibraryPage({super.key});

  @override
  Widget build(BuildContext context) {
    return AppPage(
      title: '书库',
      subtitle: '重构版会使用全新的独立书库索引、缓存和配置目录，先不影响现有打包版本。',
      actions: [
        FilledButton.icon(
          onPressed: () {},
          icon: const Icon(Icons.add),
          label: const Text('添加图书'),
        ),
        OutlinedButton.icon(
          onPressed: () {},
          icon: const Icon(Icons.folder_open),
          label: const Text('打开文件'),
        ),
      ],
      child: const LibraryEmptyPanel(),
    );
  }
}
