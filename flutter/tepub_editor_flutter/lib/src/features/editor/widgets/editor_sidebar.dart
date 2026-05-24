import 'package:flutter/material.dart';

import '../../../ui/widgets/app_surface.dart';

class EditorSidebar extends StatelessWidget {
  const EditorSidebar({super.key});

  @override
  Widget build(BuildContext context) {
    return AppSurface(
      padding: const EdgeInsets.all(8),
      child: ListView(
        children: const [
          ListTile(
            leading: Icon(Icons.menu_book_outlined),
            title: Text('目录'),
            subtitle: Text('章节扫描待迁移'),
          ),
          ListTile(
            leading: Icon(Icons.history_outlined),
            title: Text('历史版本'),
            subtitle: Text('快照与回退待迁移'),
          ),
        ],
      ),
    );
  }
}
