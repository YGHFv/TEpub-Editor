import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/storage/app_storage_paths.dart';

final storagePathsProvider = FutureProvider<AppStoragePaths>((ref) {
  return AppStoragePaths.resolve();
});

class SettingsPage extends ConsumerWidget {
  const SettingsPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final paths = ref.watch(storagePathsProvider);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text('设置', style: Theme.of(context).textTheme.headlineMedium),
        const SizedBox(height: 16),
        Card(
          elevation: 0,
          child: Padding(
            padding: const EdgeInsets.all(18),
            child: paths.when(
              data: (value) => Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                mainAxisSize: MainAxisSize.min,
                children: [
                  const Text('独立数据目录'),
                  const SizedBox(height: 8),
                  SelectableText(value.root.path),
                  const Divider(height: 28),
                  SelectableText('配置: ${value.config.path}'),
                  SelectableText('书库: ${value.library.path}'),
                  SelectableText('缓存: ${value.cache.path}'),
                  SelectableText('历史: ${value.history.path}'),
                  SelectableText('日志: ${value.logs.path}'),
                ],
              ),
              error: (error, stackTrace) => Text('读取目录失败: $error'),
              loading: () => const LinearProgressIndicator(),
            ),
          ),
        ),
      ],
    );
  }
}
