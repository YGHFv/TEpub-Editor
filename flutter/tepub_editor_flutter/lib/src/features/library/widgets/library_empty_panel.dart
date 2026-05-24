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
        title: '书库组件准备迁移',
        message: '下一步会接入图书模型、封面列表、排序筛选和导入流程。当前先保留稳定的空态布局。',
      ),
    );
  }
}
