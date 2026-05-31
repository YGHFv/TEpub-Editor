import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/settings/settings_repository.dart';
import '../../core/storage/app_storage_paths.dart';
import '../../ui/widgets/app_page.dart';
import '../../ui/widgets/app_surface.dart';
import 'widgets/font_asset_panel.dart';
import 'widgets/settings_summary.dart';
import 'widgets/storage_path_list.dart';
import 'widgets/style_template_panel.dart';

class SettingsPage extends ConsumerWidget {
  const SettingsPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final paths = ref.watch(appStoragePathsProvider);
    final settings = ref.watch(appSettingsProvider);

    return AppPage(
      title: '设置',
      subtitle: '管理重构版独立数据目录、字体资源和 CSS 样式模板，配置与现有打包版本隔离。',
      child: Column(
        children: [
          SizedBox(
            height: 180,
            child: AppSurface(
              child: settings.when(
                data: (value) => SettingsSummary(settings: value),
                error: (error, stackTrace) => Text('读取设置失败: $error'),
                loading: () => const LinearProgressIndicator(),
              ),
            ),
          ),
          const SizedBox(height: 16),
          Expanded(
            child: Row(
              children: [
                Expanded(
                  child: AppSurface(
                    child: paths.when(
                      data: (value) => StoragePathList(paths: value),
                      error: (error, stackTrace) => Text('读取目录失败: $error'),
                      loading: () => const LinearProgressIndicator(),
                    ),
                  ),
                ),
                const SizedBox(width: 16),
                const Expanded(
                  child: AppSurface(
                    child: FontAssetPanel(),
                  ),
                ),
                const SizedBox(width: 16),
                const Expanded(
                  child: AppSurface(
                    child: StyleTemplatePanel(),
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
