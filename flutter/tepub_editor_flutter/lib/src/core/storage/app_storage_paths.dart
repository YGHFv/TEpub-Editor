import 'dart:io';

import 'package:path/path.dart' as p;
import 'package:path_provider/path_provider.dart';

import '../app_constants.dart';

class AppStoragePaths {
  AppStoragePaths._({
    required this.root,
    required this.library,
    required this.cache,
    required this.history,
    required this.logs,
    required this.config,
  });

  final Directory root;
  final Directory library;
  final Directory cache;
  final Directory history;
  final Directory logs;
  final Directory config;

  File get settingsFile =>
      File(p.join(config.path, AppConstants.configFileName));

  static Future<AppStoragePaths> resolve() async {
    final supportDir = await getApplicationSupportDirectory();
    final root =
        Directory(p.join(supportDir.path, AppConstants.dataFolderName));

    final paths = AppStoragePaths._(
      root: root,
      library: Directory(p.join(root.path, 'library')),
      cache: Directory(p.join(root.path, 'cache')),
      history: Directory(p.join(root.path, 'history')),
      logs: Directory(p.join(root.path, 'logs')),
      config: Directory(p.join(root.path, 'config')),
    );

    await paths.ensureCreated();
    return paths;
  }

  Future<void> ensureCreated() async {
    for (final dir in [root, library, cache, history, logs, config]) {
      if (!await dir.exists()) {
        await dir.create(recursive: true);
      }
    }
  }
}
