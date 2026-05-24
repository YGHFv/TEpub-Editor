import 'package:flutter/material.dart';

class EditorPage extends StatelessWidget {
  const EditorPage({super.key});

  @override
  Widget build(BuildContext context) {
    return Row(
      children: [
        SizedBox(
          width: 280,
          child: Card(
            elevation: 0,
            child: ListView(
              padding: const EdgeInsets.all(12),
              children: const [
                ListTile(title: Text('目录'), subtitle: Text('章节扫描待迁移')),
              ],
            ),
          ),
        ),
        const SizedBox(width: 16),
        Expanded(
          child: Card(
            elevation: 0,
            child: Padding(
              padding: const EdgeInsets.all(18),
              child: TextField(
                expands: true,
                maxLines: null,
                minLines: null,
                decoration: const InputDecoration(
                  border: InputBorder.none,
                  hintText: 'TXT 编辑器待迁移...',
                ),
                style: const TextStyle(fontSize: 17, height: 1.7),
              ),
            ),
          ),
        ),
      ],
    );
  }
}
