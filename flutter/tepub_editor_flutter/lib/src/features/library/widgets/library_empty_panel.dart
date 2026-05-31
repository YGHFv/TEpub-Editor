import 'package:flutter/material.dart';

import '../../../ui/widgets/app_surface.dart';
import '../../../ui/widgets/empty_state.dart';

class LibraryEmptyPanel extends StatelessWidget {
  const LibraryEmptyPanel({super.key});

  @override
  Widget build(BuildContext context) {
    return const AppSurface(
      child: EmptyState(
        icon: Icons.auto_stories_outlined,
        title: '书库暂无图书',
        message: '点击右上角“添加图书”导入 TXT / EPUB。重构版会写入独立书库索引，不影响现有打包版本。',
      ),
    );
  }
}
