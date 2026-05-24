import 'package:flutter/material.dart';

import '../../../core/storage/app_storage_paths.dart';

class StoragePathList extends StatelessWidget {
  const StoragePathList({required this.paths, super.key});

  final AppStoragePaths paths;

  @override
  Widget build(BuildContext context) {
    return ListView(
      children: [
        _PathTile(label: '数据根目录', path: paths.root.path),
        const Divider(height: 24),
        _PathTile(label: '配置', path: paths.config.path),
        _PathTile(label: '书库', path: paths.library.path),
        _PathTile(label: '缓存', path: paths.cache.path),
        _PathTile(label: '历史', path: paths.history.path),
        _PathTile(label: '日志', path: paths.logs.path),
      ],
    );
  }
}

class _PathTile extends StatelessWidget {
  const _PathTile({required this.label, required this.path});

  final String label;
  final String path;

  @override
  Widget build(BuildContext context) {
    return ListTile(
      contentPadding: EdgeInsets.zero,
      title: Text(label),
      subtitle: SelectableText(path),
    );
  }
}
