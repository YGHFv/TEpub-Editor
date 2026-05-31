import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:webview_windows/webview_windows.dart';

import '../../ui/theme/app_theme.dart';
import '../../ui/widgets/app_page.dart';
import '../../ui/widgets/app_surface.dart';
import 'models/epub_editor_document.dart';
import 'providers/epub_editor_controller.dart';
import 'services/epub_editor_service.dart';

class EpubEditorPage extends ConsumerStatefulWidget {
  const EpubEditorPage({super.key});

  @override
  ConsumerState<EpubEditorPage> createState() => _EpubEditorPageState();
}

class _EpubEditorPageState extends ConsumerState<EpubEditorPage> {
  final _searchController = TextEditingController();
  final _replaceController = TextEditingController();

  @override
  void dispose() {
    _searchController.dispose();
    _replaceController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final document = ref.watch(epubEditorControllerProvider);
    final controller = ref.read(epubEditorControllerProvider.notifier);

    _syncText(_searchController, document.searchQuery);
    _syncText(_replaceController, document.replaceText);

    return AppPage(
      title: 'EPUB Editor',
      subtitle: document.epubPath.isEmpty ? document.status : document.epubPath,
      actions: [
        OutlinedButton.icon(
          onPressed: document.busy ? null : controller.pickAndOpen,
          icon: const Icon(Icons.folder_open_outlined),
          label: const Text('Open EPUB'),
        ),
        OutlinedButton.icon(
          onPressed:
              document.hasDocument && !document.busy ? controller.reload : null,
          icon: const Icon(Icons.refresh),
          label: const Text('Reload'),
        ),
        FilledButton.icon(
          onPressed: document.hasDocument && document.dirty && !document.busy
              ? controller.save
              : null,
          icon: const Icon(Icons.save_outlined),
          label: const Text('Save EPUB'),
        ),
      ],
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          SizedBox(
            width: 260,
            child: _FileTreePanel(
              document: document,
              onSelect: controller.selectFile,
            ),
          ),
          const SizedBox(width: 14),
          Expanded(
            child: _EditorPanel(
              document: document,
              searchController: _searchController,
              replaceController: _replaceController,
              onChanged: controller.updateSelectedContent,
              onSearchChanged: controller.updateSearchQuery,
              onReplaceChanged: controller.updateReplaceText,
              onRegexChanged: controller.setSearchRegex,
              onMatchCaseChanged: controller.setSearchMatchCase,
              onAllFilesChanged: controller.setSearchAllFiles,
              onFindNext: controller.findNextInSelected,
              onCountMatches: controller.countMatches,
              onReplaceAll: controller.replaceAllInSelected,
              onSearchHitSelected: controller.selectSearchHit,
            ),
          ),
          const SizedBox(width: 14),
          SizedBox(
            width: 330,
            child: _PreviewPanel(document: document),
          ),
        ],
      ),
    );
  }

  void _syncText(TextEditingController controller, String value) {
    if (controller.text == value) {
      return;
    }
    controller.text = value;
    controller.selection = TextSelection.collapsed(offset: value.length);
  }
}

class _FileTreePanel extends StatelessWidget {
  const _FileTreePanel({
    required this.document,
    required this.onSelect,
  });

  final EpubEditorDocument document;
  final ValueChanged<String> onSelect;

  @override
  Widget build(BuildContext context) {
    final files = document.visibleFiles;
    return AppSurface(
      padding: const EdgeInsets.all(12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            'Files',
            style: Theme.of(context).textTheme.titleMedium?.copyWith(
                  fontWeight: FontWeight.w800,
                ),
          ),
          const SizedBox(height: 10),
          Expanded(
            child: files.isEmpty
                ? const Center(child: Text('Open an EPUB first.'))
                : ListView.builder(
                    itemCount: files.length,
                    itemBuilder: (context, index) {
                      final file = files[index];
                      final selected = file.path == document.selectedPath;
                      return _FileRow(
                        file: file,
                        selected: selected,
                        onTap: () => onSelect(file.path),
                      );
                    },
                  ),
          ),
        ],
      ),
    );
  }
}

class _FileRow extends StatelessWidget {
  const _FileRow({
    required this.file,
    required this.selected,
    required this.onTap,
  });

  final EpubEditorFile file;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final depth = file.path.split('/').length - 1;
    return Padding(
      padding: EdgeInsets.only(left: depth.clamp(0, 4) * 10.0),
      child: ListTile(
        dense: true,
        visualDensity: const VisualDensity(horizontal: -4, vertical: -4),
        selected: selected,
        selectedTileColor: AppTheme.panelSoft,
        leading: Icon(_iconFor(file), size: 18),
        title: Text(
          file.name,
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
        ),
        subtitle: Text(
          file.modified ? 'modified' : _formatSize(file.size),
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
        ),
        onTap: onTap,
      ),
    );
  }

  IconData _iconFor(EpubEditorFile file) {
    return switch (file.kind) {
      EpubEditorFileKind.text => Icons.code,
      EpubEditorFileKind.image => Icons.image_outlined,
      EpubEditorFileKind.font => Icons.font_download_outlined,
      EpubEditorFileKind.binary => Icons.insert_drive_file_outlined,
    };
  }
}

class _EditorPanel extends StatelessWidget {
  const _EditorPanel({
    required this.document,
    required this.searchController,
    required this.replaceController,
    required this.onChanged,
    required this.onSearchChanged,
    required this.onReplaceChanged,
    required this.onRegexChanged,
    required this.onMatchCaseChanged,
    required this.onAllFilesChanged,
    required this.onFindNext,
    required this.onCountMatches,
    required this.onReplaceAll,
    required this.onSearchHitSelected,
  });

  final EpubEditorDocument document;
  final TextEditingController searchController;
  final TextEditingController replaceController;
  final ValueChanged<String> onChanged;
  final ValueChanged<String> onSearchChanged;
  final ValueChanged<String> onReplaceChanged;
  final ValueChanged<bool> onRegexChanged;
  final ValueChanged<bool> onMatchCaseChanged;
  final ValueChanged<bool> onAllFilesChanged;
  final VoidCallback onFindNext;
  final VoidCallback onCountMatches;
  final VoidCallback onReplaceAll;
  final ValueChanged<EpubEditorSearchHit> onSearchHitSelected;

  @override
  Widget build(BuildContext context) {
    final file = document.selectedFile;
    return AppSurface(
      padding: const EdgeInsets.all(14),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            children: [
              const Icon(Icons.edit_note, color: AppTheme.muted),
              const SizedBox(width: 8),
              Expanded(
                child: Text(
                  file?.path ?? 'No file selected',
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: Theme.of(context).textTheme.titleMedium?.copyWith(
                        fontWeight: FontWeight.w800,
                      ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 12),
          Expanded(child: _editorBody(file)),
          if (file?.isText == true) ...[
            const SizedBox(height: 8),
            if (document.searchHits.isNotEmpty)
              _SearchHits(
                hits: document.searchHits,
                onSelected: onSearchHitSelected,
              ),
            const SizedBox(height: 8),
            _SearchStrip(
              document: document,
              searchController: searchController,
              replaceController: replaceController,
              onSearchChanged: onSearchChanged,
              onReplaceChanged: onReplaceChanged,
              onRegexChanged: onRegexChanged,
              onMatchCaseChanged: onMatchCaseChanged,
              onAllFilesChanged: onAllFilesChanged,
              onFindNext: onFindNext,
              onCountMatches: onCountMatches,
              onReplaceAll: onReplaceAll,
            ),
          ],
          const SizedBox(height: 8),
          Text(document.status, style: const TextStyle(color: AppTheme.muted)),
        ],
      ),
    );
  }

  Widget _editorBody(EpubEditorFile? file) {
    if (file == null) {
      return const Center(child: Text('Select a file from the left.'));
    }
    if (!file.isText) {
      return _BinaryPreview(file: file);
    }
    final content = file.content;
    if (content == null) {
      return const Center(child: CircularProgressIndicator());
    }
    return _VirtualTextEditor(file: file, onChanged: onChanged);
  }
}

class _VirtualTextEditor extends StatefulWidget {
  const _VirtualTextEditor({
    required this.file,
    required this.onChanged,
  });

  final EpubEditorFile file;
  final ValueChanged<String> onChanged;

  @override
  State<_VirtualTextEditor> createState() => _VirtualTextEditorState();
}

class _VirtualTextEditorState extends State<_VirtualTextEditor> {
  static const _lineHeight = 23.0;

  final _scrollController = ScrollController();
  final _lineController = TextEditingController();
  Timer? _debounce;
  List<String> _lines = const [''];
  String _path = '';
  int? _editingLine;

  @override
  void initState() {
    super.initState();
    _syncFromFile();
  }

  @override
  void didUpdateWidget(covariant _VirtualTextEditor oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.file.path != widget.file.path ||
        oldWidget.file.content != widget.file.content) {
      _syncFromFile();
    }
  }

  @override
  void dispose() {
    _debounce?.cancel();
    _scrollController.dispose();
    _lineController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      decoration: BoxDecoration(
        color: Colors.white,
        border: Border.all(color: AppTheme.border),
        borderRadius: BorderRadius.circular(14),
      ),
      child: Scrollbar(
        controller: _scrollController,
        thumbVisibility: true,
        child: ListView.builder(
          controller: _scrollController,
          itemExtent: _lineHeight,
          // ignore: deprecated_member_use
          cacheExtent: _lineHeight * 80,
          itemCount: _lines.length,
          itemBuilder: (context, index) => _line(index),
        ),
      ),
    );
  }

  Widget _line(int index) {
    final editing = _editingLine == index;
    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onDoubleTap: () => _beginEdit(index),
      child: ColoredBox(
        color: editing ? const Color(0xFFFFF6DC) : Colors.transparent,
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            SizedBox(
              width: 58,
              child: Padding(
                padding: const EdgeInsets.only(top: 2, right: 10),
                child: Text(
                  '${index + 1}'.padLeft(4),
                  textAlign: TextAlign.right,
                  style: const TextStyle(
                    fontFamily: 'Consolas',
                    fontSize: 12,
                    color: AppTheme.muted,
                    height: 1.55,
                  ),
                ),
              ),
            ),
            Expanded(
              child: editing
                  ? TextField(
                      controller: _lineController,
                      autofocus: true,
                      maxLines: 1,
                      onChanged: (value) => _updateLine(index, value),
                      onSubmitted: (_) => setState(() => _editingLine = null),
                      style: _codeStyle,
                      decoration: const InputDecoration(
                        isDense: true,
                        border: InputBorder.none,
                        contentPadding: EdgeInsets.zero,
                      ),
                    )
                  : SingleChildScrollView(
                      scrollDirection: Axis.horizontal,
                      child: RichText(
                        maxLines: 1,
                        text: TextSpan(
                          style: _codeStyle,
                          children: _highlightLine(_lines[index], _path),
                        ),
                      ),
                    ),
            ),
          ],
        ),
      ),
    );
  }

  void _syncFromFile() {
    _path = widget.file.path;
    final text = widget.file.content ?? widget.file.previewContent ?? '';
    _lines = text.replaceAll('\r\n', '\n').replaceAll('\r', '\n').split('\n');
    if (_lines.isEmpty) {
      _lines = [''];
    }
    _editingLine = null;
    _lineController.clear();
  }

  void _beginEdit(int index) {
    setState(() {
      _editingLine = index;
      _lineController.text = _lines[index];
      _lineController.selection =
          TextSelection.collapsed(offset: _lineController.text.length);
    });
  }

  void _updateLine(int index, String value) {
    _lines[index] = value;
    _debounce?.cancel();
    _debounce = Timer(const Duration(milliseconds: 180), () {
      widget.onChanged(_lines.join('\n'));
    });
  }
}

class _SearchStrip extends StatelessWidget {
  const _SearchStrip({
    required this.document,
    required this.searchController,
    required this.replaceController,
    required this.onSearchChanged,
    required this.onReplaceChanged,
    required this.onRegexChanged,
    required this.onMatchCaseChanged,
    required this.onAllFilesChanged,
    required this.onFindNext,
    required this.onCountMatches,
    required this.onReplaceAll,
  });

  final EpubEditorDocument document;
  final TextEditingController searchController;
  final TextEditingController replaceController;
  final ValueChanged<String> onSearchChanged;
  final ValueChanged<String> onReplaceChanged;
  final ValueChanged<bool> onRegexChanged;
  final ValueChanged<bool> onMatchCaseChanged;
  final ValueChanged<bool> onAllFilesChanged;
  final VoidCallback onFindNext;
  final VoidCallback onCountMatches;
  final VoidCallback onReplaceAll;

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      decoration: BoxDecoration(
        color: AppTheme.panelSoft.withValues(alpha: 0.72),
        border: Border.all(color: AppTheme.border),
        borderRadius: BorderRadius.circular(12),
      ),
      child: Padding(
        padding: const EdgeInsets.all(8),
        child: Wrap(
          spacing: 8,
          runSpacing: 8,
          crossAxisAlignment: WrapCrossAlignment.center,
          children: [
            SizedBox(
              width: 170,
              child: TextField(
                controller: searchController,
                onChanged: onSearchChanged,
                decoration: const InputDecoration(
                  isDense: true,
                  labelText: 'Find',
                ),
              ),
            ),
            SizedBox(
              width: 170,
              child: TextField(
                controller: replaceController,
                onChanged: onReplaceChanged,
                decoration: const InputDecoration(
                  isDense: true,
                  labelText: 'Replace',
                ),
              ),
            ),
            FilterChip(
              label: const Text('Regex'),
              selected: document.searchRegex,
              onSelected: onRegexChanged,
            ),
            FilterChip(
              label: const Text('Case'),
              selected: document.searchMatchCase,
              onSelected: onMatchCaseChanged,
            ),
            FilterChip(
              label: const Text('All files'),
              selected: document.searchAllFiles,
              onSelected: onAllFilesChanged,
            ),
            OutlinedButton(
              onPressed: document.searchQuery.isEmpty ? null : onFindNext,
              child: const Text('Find'),
            ),
            OutlinedButton(
              onPressed: document.searchQuery.isEmpty ? null : onCountMatches,
              child: const Text('Count'),
            ),
            FilledButton(
              onPressed: document.searchQuery.isEmpty ? null : onReplaceAll,
              child: const Text('Replace all'),
            ),
          ],
        ),
      ),
    );
  }
}

class _SearchHits extends StatelessWidget {
  const _SearchHits({
    required this.hits,
    required this.onSelected,
  });

  final List<EpubEditorSearchHit> hits;
  final ValueChanged<EpubEditorSearchHit> onSelected;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      height: 92,
      child: ListView.builder(
        itemCount: hits.length,
        itemBuilder: (context, index) {
          final hit = hits[index];
          return ListTile(
            dense: true,
            title: Text(hit.filePath, maxLines: 1),
            subtitle: Text(hit.preview, maxLines: 1),
            trailing: Text('@${hit.offset}'),
            onTap: () => onSelected(hit),
          );
        },
      ),
    );
  }
}

class _BinaryPreview extends StatelessWidget {
  const _BinaryPreview({required this.file});

  final EpubEditorFile file;

  @override
  Widget build(BuildContext context) {
    if (file.kind == EpubEditorFileKind.image &&
        file.bytes != null &&
        file.bytes!.isNotEmpty) {
      return InteractiveViewer(
        child: Image.memory(Uint8List.fromList(file.bytes!)),
      );
    }
    return Center(
      child: Text('${file.name}\n${_formatSize(file.size)}'),
    );
  }
}

class _PreviewPanel extends StatefulWidget {
  const _PreviewPanel({required this.document});

  final EpubEditorDocument document;

  @override
  State<_PreviewPanel> createState() => _PreviewPanelState();
}

class _PreviewPanelState extends State<_PreviewPanel> {
  var _showPreview = true;

  @override
  Widget build(BuildContext context) {
    final file = widget.document.selectedFile;
    return AppSurface(
      padding: const EdgeInsets.all(12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          SegmentedButton<bool>(
            segments: const [
              ButtonSegment(value: true, label: Text('Preview')),
              ButtonSegment(value: false, label: Text('TOC')),
            ],
            selected: {_showPreview},
            onSelectionChanged: (value) {
              setState(() => _showPreview = value.first);
            },
          ),
          const SizedBox(height: 12),
          Expanded(
            child: _showPreview
                ? _PreviewBody(document: widget.document, file: file)
                : _TocPreview(document: widget.document),
          ),
        ],
      ),
    );
  }
}

class _PreviewBody extends StatelessWidget {
  const _PreviewBody({
    required this.document,
    required this.file,
  });

  final EpubEditorDocument document;
  final EpubEditorFile? file;

  @override
  Widget build(BuildContext context) {
    final current = file;
    if (current == null) {
      return const Center(child: Text('Select a file to preview.'));
    }
    if (current.kind == EpubEditorFileKind.image) {
      return _BinaryPreview(file: current);
    }
    if (!current.isText) {
      return Center(child: Text(current.name));
    }
    final content = current.content ?? current.previewContent ?? '';
    if (_isHtml(current.path)) {
      return _WebHtmlPreview(
        document: document,
        file: current,
        content: content,
      );
    }
    return SingleChildScrollView(
      child: SelectableText(
        content.isEmpty ? 'Empty file' : content,
        style: _codeStyle,
      ),
    );
  }

  bool _isHtml(String path) {
    final lower = path.toLowerCase();
    return lower.endsWith('.html') ||
        lower.endsWith('.htm') ||
        lower.endsWith('.xhtml');
  }
}

class _WebHtmlPreview extends ConsumerStatefulWidget {
  const _WebHtmlPreview({
    required this.document,
    required this.file,
    required this.content,
  });

  final EpubEditorDocument document;
  final EpubEditorFile file;
  final String content;

  @override
  ConsumerState<_WebHtmlPreview> createState() => _WebHtmlPreviewState();
}

class _WebHtmlPreviewState extends ConsumerState<_WebHtmlPreview> {
  late final WebviewController _controller;
  late final Future<void> _init;
  String _lastKey = '';
  bool _failed = false;

  @override
  void initState() {
    super.initState();
    _controller = WebviewController();
    _init = _controller.initialize().catchError((_) {
      if (mounted) {
        setState(() => _failed = true);
      }
    });
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final base = ref.watch(epubPreviewResourceServerProvider);
    _ensureServer(base);
    final html = _buildHtml(base);
    final key = '${widget.file.path}|${widget.content.length}|$base';
    if (key != _lastKey) {
      _lastKey = key;
      _load(html);
    }
    if (_failed) {
      return SingleChildScrollView(child: SelectableText(_plainText(html)));
    }
    return FutureBuilder<void>(
      future: _init,
      builder: (context, snapshot) {
        if (snapshot.connectionState != ConnectionState.done) {
          return const Center(child: CircularProgressIndicator());
        }
        _load(html);
        return Webview(_controller);
      },
    );
  }

  void _ensureServer(Uri? base) {
    if (base != null || widget.document.epubPath.isEmpty) {
      return;
    }
    Future.microtask(() {
      if (!mounted) {
        return;
      }
      ref.read(epubPreviewResourceServerProvider.notifier).ensureStarted(
            epubPath: widget.document.epubPath,
            files: widget.document.visibleFiles,
          );
    });
  }

  void _load(String html) {
    if (_failed) {
      return;
    }
    _init.then((_) {
      if (mounted && !_failed) {
        _controller.loadStringContent(html);
      }
    });
  }

  String _buildHtml(Uri? base) {
    var html = widget.content;
    html = html.replaceAllMapped(
      RegExp(r'''<link\b[^>]*rel=["']stylesheet["'][^>]*>''',
          caseSensitive: false),
      (match) {
        final tag = match.group(0) ?? '';
        final href = RegExp(r'''href=["']([^"']+)["']''', caseSensitive: false)
                .firstMatch(tag)
                ?.group(1) ??
            '';
        final path = _resolvePath(widget.file.path, href);
        final cssFile = _findFile(path);
        final css = cssFile?.content ?? cssFile?.previewContent;
        if (css == null) {
          return '';
        }
        return '<style>${_rewriteCssUrls(css, cssFile!.path, base)}</style>';
      },
    );
    html = html.replaceAllMapped(
      RegExp(r'''<img\b([^>]*?)\bsrc=["']([^"']+)["']([^>]*)>''',
          caseSensitive: false),
      (match) {
        final before = match.group(1) ?? '';
        final src = match.group(2) ?? '';
        final after = match.group(3) ?? '';
        final url = _resourceUrl(_resolvePath(widget.file.path, src), base);
        return url == null
            ? (match.group(0) ?? '')
            : '<img$before src="$url"$after>';
      },
    );
    return '''
<!doctype html>
<html>
<head>
<meta charset="utf-8">
<style>
body { margin: 16px; background: #fffcf4; color: #172026; line-height: 1.75; }
img { max-width: 100%; height: auto; }
</style>
</head>
<body>$html</body>
</html>
''';
  }

  String _rewriteCssUrls(String css, String cssPath, Uri? base) {
    return css.replaceAllMapped(
      RegExp(r'''url\((["']?)([^)"']+)\1\)'''),
      (match) {
        final raw = (match.group(2) ?? '').trim();
        if (raw.startsWith('data:') || raw.startsWith('http')) {
          return match.group(0) ?? '';
        }
        final url = _resourceUrl(_resolvePath(cssPath, raw), base);
        return url == null ? (match.group(0) ?? '') : 'url("$url")';
      },
    );
  }

  String? _resourceUrl(String path, Uri? base) {
    if (base == null) {
      return null;
    }
    final file = _findFile(path);
    if (file == null) {
      return null;
    }
    return base
        .replace(pathSegments: _normalizePath(file.path).split('/'))
        .toString();
  }

  EpubEditorFile? _findFile(String path) {
    final normalized = _normalizePath(path).toLowerCase();
    for (final file in widget.document.visibleFiles) {
      final item = _normalizePath(file.path).toLowerCase();
      if (item == normalized || item.endsWith('/$normalized')) {
        return file;
      }
    }
    return null;
  }
}

class _TocPreview extends StatelessWidget {
  const _TocPreview({required this.document});

  final EpubEditorDocument document;

  @override
  Widget build(BuildContext context) {
    final toc = document.visibleFiles.where(_isToc).firstOrNull;
    final content = toc?.content ?? toc?.previewContent ?? '';
    final items = _parseNcx(content);
    if (items.isEmpty) {
      return const Center(child: Text('No toc.ncx loaded.'));
    }
    return ListView.builder(
      itemCount: items.length,
      itemBuilder: (context, index) {
        final item = items[index];
        return Padding(
          padding: EdgeInsets.only(left: item.depth * 12.0),
          child: ListTile(
            dense: true,
            title:
                Text(item.title, maxLines: 1, overflow: TextOverflow.ellipsis),
            subtitle:
                Text(item.src, maxLines: 1, overflow: TextOverflow.ellipsis),
          ),
        );
      },
    );
  }

  bool _isToc(EpubEditorFile file) {
    return file.path.toLowerCase().endsWith('.ncx');
  }

  List<_TocItem> _parseNcx(String source) {
    if (source.trim().isEmpty) {
      return const [];
    }
    final items = <_TocItem>[];
    final navPoint = RegExp(
      r'<navPoint\b[\s\S]*?</navPoint>',
      caseSensitive: false,
    );
    for (final match in navPoint.allMatches(source).take(500)) {
      final block = match.group(0) ?? '';
      final title = RegExp(
            r'<text[^>]*>([\s\S]*?)</text>',
            caseSensitive: false,
          ).firstMatch(block)?.group(1) ??
          '';
      final src = RegExp(
            r'''<content[^>]*src=["']([^"']+)["']''',
            caseSensitive: false,
          ).firstMatch(block)?.group(1) ??
          '';
      final depth =
          '<navPoint'.allMatches(source.substring(0, match.start)).length -
              '</navPoint>'.allMatches(source.substring(0, match.start)).length;
      items.add(_TocItem(_stripTags(title), src, depth.clamp(0, 6)));
    }
    return items;
  }
}

class _TocItem {
  const _TocItem(this.title, this.src, this.depth);

  final String title;
  final String src;
  final int depth;
}

const _codeStyle = TextStyle(
  fontFamily: 'Consolas',
  fontSize: 13,
  height: 1.55,
  color: AppTheme.ink,
);

List<TextSpan> _highlightLine(String line, String path) {
  final lower = path.toLowerCase();
  if (lower.endsWith('.css')) {
    return _highlightCss(line);
  }
  if (lower.endsWith('.html') ||
      lower.endsWith('.htm') ||
      lower.endsWith('.xhtml') ||
      lower.endsWith('.xml') ||
      lower.endsWith('.opf') ||
      lower.endsWith('.ncx')) {
    return _highlightMarkup(line);
  }
  return [TextSpan(text: line)];
}

List<TextSpan> _highlightMarkup(String line) {
  final spans = <TextSpan>[];
  final token = RegExp(r'<!--.*?-->|<[^>]*>');
  var cursor = 0;
  for (final match in token.allMatches(line)) {
    if (match.start > cursor) {
      spans.add(TextSpan(text: line.substring(cursor, match.start)));
    }
    final value = match.group(0)!;
    spans.add(TextSpan(text: value, style: _tagStyle));
    cursor = match.end;
  }
  if (cursor < line.length) {
    spans.add(TextSpan(text: line.substring(cursor)));
  }
  return spans;
}

List<TextSpan> _highlightCss(String line) {
  final spans = <TextSpan>[];
  final token =
      RegExp(r'/\*.*?\*/|#[0-9a-fA-F]{3,8}\b|[\w-]+(?=\s*:)|[{}:;(),]');
  var cursor = 0;
  for (final match in token.allMatches(line)) {
    if (match.start > cursor) {
      spans.add(TextSpan(text: line.substring(cursor, match.start)));
    }
    final value = match.group(0)!;
    final style = value.startsWith('#')
        ? _valueStyle
        : RegExp(r'^[\w-]+$').hasMatch(value)
            ? _nameStyle
            : _punctuationStyle;
    spans.add(TextSpan(text: value, style: style));
    cursor = match.end;
  }
  if (cursor < line.length) {
    spans.add(TextSpan(text: line.substring(cursor)));
  }
  return spans;
}

const _tagStyle = TextStyle(color: Color(0xFF0A7F46));
const _nameStyle = TextStyle(color: Color(0xFF953800));
const _valueStyle = TextStyle(color: Color(0xFFB42318));
const _punctuationStyle = TextStyle(color: Color(0xFF6B7280));

String _resolvePath(String fromPath, String target) {
  final clean = target.split('#').first.split('?').first;
  if (clean.isEmpty || clean.startsWith('data:') || clean.startsWith('http')) {
    return clean;
  }
  if (clean.startsWith('/')) {
    return clean.substring(1);
  }
  final parts = fromPath.replaceAll('\\', '/').split('/')..removeLast();
  for (final part in clean.replaceAll('\\', '/').split('/')) {
    if (part.isEmpty || part == '.') {
      continue;
    }
    if (part == '..') {
      if (parts.isNotEmpty) {
        parts.removeLast();
      }
    } else {
      parts.add(part);
    }
  }
  return parts.join('/');
}

String _normalizePath(String path) {
  final parts = <String>[];
  for (final part in path.replaceAll('\\', '/').split('/')) {
    if (part.isEmpty || part == '.') {
      continue;
    }
    if (part == '..') {
      if (parts.isNotEmpty) {
        parts.removeLast();
      }
    } else {
      parts.add(part);
    }
  }
  return parts.join('/');
}

String _plainText(String html) {
  return _stripTags(html)
      .replaceAll('&nbsp;', ' ')
      .replaceAll('&amp;', '&')
      .replaceAll('&lt;', '<')
      .replaceAll('&gt;', '>')
      .trim();
}

String _stripTags(String html) {
  return html
      .replaceAll(RegExp(r'<[^>]+>'), '')
      .replaceAll(RegExp(r'\s+'), ' ');
}

String _formatSize(int size) {
  if (size >= 1024 * 1024) {
    return '${(size / 1024 / 1024).toStringAsFixed(1)} MB';
  }
  if (size >= 1024) {
    return '${(size / 1024).toStringAsFixed(1)} KB';
  }
  return '$size B';
}

extension _FirstOrNull<T> on Iterable<T> {
  T? get firstOrNull {
    final iterator = this.iterator;
    return iterator.moveNext() ? iterator.current : null;
  }
}
