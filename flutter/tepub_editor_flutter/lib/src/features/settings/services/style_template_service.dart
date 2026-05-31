import 'dart:io';

import 'package:file_picker/file_picker.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:path/path.dart' as p;

import '../../../core/storage/app_storage_paths.dart';
import '../../epub/services/epub_make_service.dart';
import '../models/style_template.dart';

final styleTemplateServiceProvider =
    FutureProvider<StyleTemplateService>((ref) async {
  final paths = await ref.watch(appStoragePathsProvider.future);
  return StyleTemplateService(paths);
});

final styleTemplatesProvider = FutureProvider<List<StyleTemplate>>((ref) async {
  final service = await ref.watch(styleTemplateServiceProvider.future);
  return service.listTemplates();
});

class StyleTemplateService {
  const StyleTemplateService(this.paths);

  final AppStoragePaths paths;

  Future<List<StyleTemplate>> listTemplates() async {
    await paths.ensureCreated();
    await _ensureBuiltinTemplate();
    final templates = <StyleTemplate>[];
    await for (final entity in paths.styleTemplates.list()) {
      if (entity is! File || p.extension(entity.path).toLowerCase() != '.css') {
        continue;
      }
      final stat = await entity.stat();
      final id = p.basenameWithoutExtension(entity.path);
      templates.add(
        StyleTemplate(
          id: id,
          name: _templateName(id),
          path: entity.path,
          css: await entity.readAsString(),
          updatedAt: stat.modified,
          builtin: id == 'builtin',
        ),
      );
    }
    templates.sort((a, b) {
      if (a.builtin != b.builtin) {
        return a.builtin ? -1 : 1;
      }
      return a.name.toLowerCase().compareTo(b.name.toLowerCase());
    });
    return templates;
  }

  Future<List<StyleTemplate>> importTemplate() async {
    final result = await FilePicker.platform.pickFiles(
      allowMultiple: false,
      type: FileType.custom,
      allowedExtensions: const ['css'],
    );
    final sourcePath = result?.files.single.path;
    if (sourcePath == null) {
      return listTemplates();
    }
    await paths.ensureCreated();
    final target = File(
      p.join(
        paths.styleTemplates.path,
        '${_safeId(p.basenameWithoutExtension(sourcePath))}.css',
      ),
    );
    await File(sourcePath).copy(target.path);
    return listTemplates();
  }

  Future<List<StyleTemplate>> saveTemplate({
    required String name,
    required String css,
  }) async {
    await paths.ensureCreated();
    final id = _safeId(name.trim().isEmpty ? 'style-template' : name);
    await File(p.join(paths.styleTemplates.path, '$id.css')).writeAsString(css);
    return listTemplates();
  }

  Future<List<StyleTemplate>> deleteTemplate(StyleTemplate template) async {
    if (template.builtin) {
      return listTemplates();
    }
    final file = File(template.path);
    if (await file.exists()) {
      await file.delete();
    }
    return listTemplates();
  }

  Future<void> _ensureBuiltinTemplate() async {
    final file = File(p.join(paths.styleTemplates.path, 'builtin.css'));
    if (!await file.exists()) {
      await file.writeAsString(EpubMakeService.defaultMainCss);
    }
  }

  String _templateName(String id) {
    if (id == 'builtin') {
      return '内置默认样式';
    }
    return id.replaceAll(RegExp(r'[_-]+'), ' ');
  }

  String _safeId(String input) {
    final safe = input
        .trim()
        .replaceAll(RegExp(r'[\\/:*?"<>|]+'), '_')
        .replaceAll(RegExp(r'\s+'), '_');
    return safe.isEmpty ? 'style-template' : safe;
  }
}
