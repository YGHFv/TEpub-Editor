import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../ui/theme/app_theme.dart';
import '../models/font_asset.dart';
import '../services/font_asset_service.dart';

class FontAssetPanel extends ConsumerStatefulWidget {
  const FontAssetPanel({super.key});

  @override
  ConsumerState<FontAssetPanel> createState() => _FontAssetPanelState();
}

class _FontAssetPanelState extends ConsumerState<FontAssetPanel> {
  List<FontAsset> _fonts = const [];
  String _message = '';
  bool _busy = false;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => _loadFonts());
  }

  Future<void> _loadFonts() async {
    final service = await ref.read(fontAssetServiceProvider.future);
    final fonts = await service.listFonts();
    if (mounted) {
      setState(() => _fonts = fonts);
    }
  }

  Future<void> _importFont() async {
    setState(() {
      _busy = true;
      _message = '正在导入字体...';
    });
    try {
      final service = await ref.read(fontAssetServiceProvider.future);
      final fonts = await service.importFont();
      if (mounted) {
        setState(() {
          _fonts = fonts;
          _message = '字体列表已更新。';
        });
      }
    } catch (error) {
      if (mounted) {
        setState(() => _message = '导入失败：$error');
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  Future<void> _deleteFont(FontAsset font) async {
    setState(() {
      _busy = true;
      _message = '正在删除字体...';
    });
    try {
      final service = await ref.read(fontAssetServiceProvider.future);
      final fonts = await service.deleteFont(font);
      if (mounted) {
        setState(() {
          _fonts = fonts;
          _message = '已删除 ${font.fileName}。';
        });
      }
    } catch (error) {
      if (mounted) {
        setState(() => _message = '删除失败：$error');
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          children: [
            Expanded(
              child: Text(
                '字体管理',
                style: Theme.of(context).textTheme.titleMedium?.copyWith(
                      fontWeight: FontWeight.w800,
                    ),
              ),
            ),
            OutlinedButton.icon(
              onPressed: _busy ? null : _importFont,
              icon: const Icon(Icons.upload_file),
              label: const Text('导入字体'),
            ),
          ],
        ),
        const SizedBox(height: 8),
        Text(
          _message.isEmpty
              ? '支持 ttf / otf / woff / woff2，写入重构版独立 fonts 目录。'
              : _message,
          style: const TextStyle(color: AppTheme.muted),
        ),
        const SizedBox(height: 10),
        if (_busy) const LinearProgressIndicator(),
        Expanded(
          child: _fonts.isEmpty
              ? const Center(child: Text('暂无导入字体。'))
              : ListView.separated(
                  itemCount: _fonts.length,
                  separatorBuilder: (context, index) => const Divider(),
                  itemBuilder: (context, index) {
                    final font = _fonts[index];
                    return ListTile(
                      dense: true,
                      contentPadding: EdgeInsets.zero,
                      title: Text(font.family),
                      subtitle:
                          Text('${font.fileName} · ${_formatSize(font.size)}'),
                      trailing: TextButton(
                        onPressed: _busy ? null : () => _deleteFont(font),
                        child: const Text('删除'),
                      ),
                    );
                  },
                ),
        ),
      ],
    );
  }

  String _formatSize(int size) {
    if (size >= 1024 * 1024) {
      return '${(size / 1024 / 1024).toStringAsFixed(1)} MB';
    }
    return '${(size / 1024).toStringAsFixed(1)} KB';
  }
}
