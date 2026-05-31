import 'dart:io';

import 'package:file_picker/file_picker.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:path/path.dart' as p;

import '../../../core/storage/app_storage_paths.dart';
import '../models/font_asset.dart';

final fontAssetServiceProvider = FutureProvider<FontAssetService>((ref) async {
  final paths = await ref.watch(appStoragePathsProvider.future);
  return FontAssetService(paths);
});

class FontAssetService {
  const FontAssetService(this.paths);

  final AppStoragePaths paths;

  Future<List<FontAsset>> listFonts() async {
    await paths.ensureCreated();
    final fonts = <FontAsset>[];
    await for (final entity in paths.fonts.list()) {
      if (entity is! File) {
        continue;
      }
      final extension = p.extension(entity.path).toLowerCase();
      if (!['.ttf', '.otf', '.woff', '.woff2'].contains(extension)) {
        continue;
      }
      final stat = await entity.stat();
      fonts.add(
        FontAsset(
          fileName: p.basename(entity.path),
          path: entity.path,
          family: p.basenameWithoutExtension(entity.path),
          size: stat.size,
        ),
      );
    }
    fonts.sort(
        (a, b) => a.family.toLowerCase().compareTo(b.family.toLowerCase()));
    return fonts;
  }

  Future<List<FontAsset>> importFont() async {
    final result = await FilePicker.platform.pickFiles(
      allowMultiple: false,
      type: FileType.custom,
      allowedExtensions: const ['ttf', 'otf', 'woff', 'woff2'],
    );
    final sourcePath = result?.files.single.path;
    if (sourcePath == null) {
      return listFonts();
    }
    await paths.ensureCreated();
    final source = File(sourcePath);
    final target = File(p.join(paths.fonts.path, p.basename(sourcePath)));
    if (!await target.exists()) {
      await source.copy(target.path);
    }
    return listFonts();
  }

  Future<List<FontAsset>> deleteFont(FontAsset font) async {
    final file = File(font.path);
    if (await file.exists()) {
      await file.delete();
    }
    return listFonts();
  }
}
