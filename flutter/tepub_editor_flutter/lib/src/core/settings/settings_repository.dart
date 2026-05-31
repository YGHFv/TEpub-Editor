import 'dart:convert';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../storage/app_storage_paths.dart';
import 'app_settings.dart';

final settingsRepositoryProvider =
    FutureProvider<SettingsRepository>((ref) async {
  final paths = await ref.watch(appStoragePathsProvider.future);
  return SettingsRepository(paths);
});

final appSettingsProvider = FutureProvider<AppSettings>((ref) async {
  final repository = await ref.watch(settingsRepositoryProvider.future);
  return repository.load();
});

class SettingsRepository {
  const SettingsRepository(this.paths);

  final AppStoragePaths paths;

  Future<AppSettings> load() async {
    final file = paths.settingsFile;
    if (!await file.exists()) {
      await save(AppSettings.defaults);
      return AppSettings.defaults;
    }

    final text = await file.readAsString();
    final json = jsonDecode(text) as Map<String, Object?>;
    return AppSettings.fromJson(json);
  }

  Future<void> save(AppSettings settings) async {
    await paths.ensureCreated();
    const encoder = JsonEncoder.withIndent('  ');
    await paths.settingsFile.writeAsString(encoder.convert(settings.toJson()));
  }
}
