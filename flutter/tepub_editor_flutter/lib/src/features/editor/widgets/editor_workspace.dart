import 'package:flutter/material.dart';

import '../../../ui/widgets/app_surface.dart';

class EditorWorkspace extends StatelessWidget {
  const EditorWorkspace({super.key});

  @override
  Widget build(BuildContext context) {
    return const AppSurface(
      padding: EdgeInsets.all(18),
      child: TextField(
        expands: true,
        maxLines: null,
        minLines: null,
        decoration: InputDecoration(
          border: InputBorder.none,
          enabledBorder: InputBorder.none,
          focusedBorder: InputBorder.none,
          filled: false,
          hintText: 'TXT 编辑器待迁移...',
        ),
        style: TextStyle(fontSize: 17, height: 1.7),
      ),
    );
  }
}
