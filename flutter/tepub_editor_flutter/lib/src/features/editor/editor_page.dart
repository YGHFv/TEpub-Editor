import 'package:flutter/material.dart';

import 'widgets/editor_sidebar.dart';
import 'widgets/editor_workspace.dart';
import '../../ui/widgets/app_page.dart';
import '../../ui/widgets/responsive_two_pane.dart';

class EditorPage extends StatelessWidget {
  const EditorPage({super.key});

  @override
  Widget build(BuildContext context) {
    return AppPage(
      title: '编辑器',
      subtitle: '先搭好目录、正文和工具区的桌面布局，后续逐步迁移 TXT/EPUB 编辑能力。',
      actions: [
        OutlinedButton.icon(
          onPressed: () {},
          icon: const Icon(Icons.search),
          label: const Text('搜索'),
        ),
        FilledButton.icon(
          onPressed: () {},
          icon: const Icon(Icons.save_outlined),
          label: const Text('保存'),
        ),
      ],
      child: ResponsiveTwoPane(
        side: const EditorSidebar(),
        body: const EditorWorkspace(),
      ),
    );
  }
}
