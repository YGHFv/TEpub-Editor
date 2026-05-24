import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'widgets/storage_path_list.dart';
import '../../core/storage/app_storage_paths.dart';
import '../../ui/widgets/app_page.dart';
import '../../ui/widgets/app_surface.dart';

final storagePathsProvider = FutureProvider<AppStoragePaths>((ref) {
  return AppStoragePaths.resolve();
});

class SettingsPage extends ConsumerWidget {
  const SettingsPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final paths = ref.watch(storagePathsProvider);

    return AppPage(
      title: '设置',
      subtitle: '这里先展示 Flutter 重构版的独立数据目录，后续迁移阅读、编辑、AI 与样式设置。',
      child: AppSurface(
        child: paths.when(
          data: (value) => StoragePathList(paths: value),
          error: (error, stackTrace) => Text('读取目录失败: $error'),
          loading: () => const LinearProgressIndicator(),
        ),
      ),
    );
  }
}
