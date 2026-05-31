import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:path/path.dart' as p;

import '../../../ui/theme/app_theme.dart';
import '../../../ui/widgets/app_surface.dart';
import '../../epub/models/epub_project.dart';
import '../../epub/services/epub_make_service.dart';
import '../../settings/models/style_template.dart';
import '../../settings/services/font_asset_service.dart';
import '../../settings/services/style_template_service.dart';
import '../models/editor_document.dart';

class EditorMakeEpubDialog extends ConsumerStatefulWidget {
  const EditorMakeEpubDialog({
    required this.document,
    super.key,
  });

  final EditorDocument document;

  @override
  ConsumerState<EditorMakeEpubDialog> createState() =>
      _EditorMakeEpubDialogState();
}

class _EditorMakeEpubDialogState extends ConsumerState<EditorMakeEpubDialog> {
  late final TextEditingController _titleController;
  late final TextEditingController _authorController;
  late final TextEditingController _publisherController;
  late final TextEditingController _dateController;
  late final TextEditingController _descriptionController;
  late final TextEditingController _tagsController;
  late final TextEditingController _mainCssController;
  late final TextEditingController _fontCssController;
  late final TextEditingController _uuidController;
  String _coverPath = '';
  String _status = '';
  List<CoverSearchResult> _coverResults = const [];
  bool _busy = false;
  bool _coverBusy = false;
  bool _advanced = false;

  @override
  void initState() {
    super.initState();
    _titleController = TextEditingController(text: widget.document.title);
    _authorController = TextEditingController();
    _publisherController = TextEditingController();
    _dateController = TextEditingController(
      text: DateTime.now().year.toString(),
    );
    _descriptionController = TextEditingController();
    _tagsController = TextEditingController();
    _mainCssController = TextEditingController();
    _fontCssController = TextEditingController();
    _uuidController = TextEditingController(
      text: 'tepub-${DateTime.now().microsecondsSinceEpoch}',
    );
  }

  @override
  void dispose() {
    _titleController.dispose();
    _authorController.dispose();
    _publisherController.dispose();
    _dateController.dispose();
    _descriptionController.dispose();
    _tagsController.dispose();
    _mainCssController.dispose();
    _fontCssController.dispose();
    _uuidController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final service = ref.read(epubMakeServiceProvider);
    final chapters = _editorChaptersToEpub(widget.document);
    final styleTemplates = ref.watch(styleTemplatesProvider);

    return AppSurface(
      padding: EdgeInsets.zero,
      child: SizedBox(
        width: 560,
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxHeight: 760),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Padding(
                padding: const EdgeInsets.fromLTRB(20, 18, 14, 12),
                child: Row(
                  children: [
                    Expanded(
                      child: Text(
                        '制作 EPUB',
                        style: Theme.of(context).textTheme.titleLarge?.copyWith(
                              fontWeight: FontWeight.w800,
                            ),
                      ),
                    ),
                    IconButton(
                      tooltip: '关闭',
                      onPressed:
                          _busy ? null : () => Navigator.of(context).pop(),
                      icon: const Icon(Icons.close),
                    ),
                  ],
                ),
              ),
              const Divider(height: 1),
              Flexible(
                child: SingleChildScrollView(
                  padding: const EdgeInsets.all(18),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      _InfoBanner(
                        text: chapters.isEmpty
                            ? '当前没有目录项，将按全文生成单章 EPUB。'
                            : '使用当前 TXT 编辑器正文和当前目录生成 EPUB，共 ${chapters.length} 个目录项。',
                      ),
                      const SizedBox(height: 14),
                      TextField(
                        controller: _titleController,
                        decoration: const InputDecoration(labelText: '书名'),
                      ),
                      const SizedBox(height: 12),
                      TextField(
                        controller: _authorController,
                        decoration: const InputDecoration(labelText: '作者'),
                      ),
                      const SizedBox(height: 12),
                      TextField(
                        controller: _descriptionController,
                        minLines: 3,
                        maxLines: 5,
                        decoration: const InputDecoration(
                          labelText: '简介',
                          alignLabelWithHint: true,
                        ),
                      ),
                      const SizedBox(height: 12),
                      TextField(
                        controller: _tagsController,
                        decoration: const InputDecoration(
                          labelText: '标签',
                          hintText: '用逗号或空格分隔',
                        ),
                      ),
                      const SizedBox(height: 12),
                      TextField(
                        controller: _uuidController,
                        decoration: const InputDecoration(labelText: 'UUID'),
                      ),
                      const SizedBox(height: 12),
                      Row(
                        children: [
                          Expanded(
                            child: Text(
                              _coverPath.isEmpty
                                  ? '未选择封面'
                                  : '封面：${p.basename(_coverPath)}',
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                              style: const TextStyle(color: AppTheme.muted),
                            ),
                          ),
                          OutlinedButton.icon(
                            onPressed: _busy
                                ? null
                                : () async {
                                    final path = await service.pickCover();
                                    if (path != null && mounted) {
                                      setState(() => _coverPath = path);
                                    }
                                  },
                            icon: const Icon(Icons.image_outlined),
                            label: const Text('选择封面'),
                          ),
                        ],
                      ),
                      const SizedBox(height: 10),
                      Align(
                        alignment: Alignment.centerLeft,
                        child: OutlinedButton.icon(
                          onPressed: _busy || _coverBusy
                              ? null
                              : () => _searchCovers(service),
                          icon: const Icon(Icons.travel_explore),
                          label: Text(_coverBusy ? '搜索中...' : '搜索封面'),
                        ),
                      ),
                      if (_coverResults.isNotEmpty) ...[
                        const SizedBox(height: 12),
                        SizedBox(
                          height: 180,
                          child: ListView.separated(
                            scrollDirection: Axis.horizontal,
                            itemCount: _coverResults.length,
                            separatorBuilder: (context, index) =>
                                const SizedBox(width: 10),
                            itemBuilder: (context, index) {
                              final result = _coverResults[index];
                              return _CoverResultCard(
                                result: result,
                                onTap: () => _applyRemoteCover(service, result),
                              );
                            },
                          ),
                        ),
                      ],
                      if (_status.isNotEmpty) ...[
                        const SizedBox(height: 12),
                        Text(_status),
                      ],
                      const SizedBox(height: 12),
                      ExpansionPanelList(
                        expansionCallback: (index, expanded) {
                          setState(() => _advanced = !_advanced);
                        },
                        children: [
                          ExpansionPanel(
                            isExpanded: _advanced,
                            headerBuilder: (context, expanded) {
                              return const ListTile(
                                title: Text('高级选项'),
                                subtitle: Text('出版社、日期、自定义 CSS'),
                              );
                            },
                            body: Padding(
                              padding: const EdgeInsets.fromLTRB(16, 0, 16, 16),
                              child: Column(
                                children: [
                                  TextField(
                                    controller: _publisherController,
                                    decoration:
                                        const InputDecoration(labelText: '出版社'),
                                  ),
                                  const SizedBox(height: 12),
                                  TextField(
                                    controller: _dateController,
                                    decoration:
                                        const InputDecoration(labelText: '日期'),
                                  ),
                                  const SizedBox(height: 12),
                                  TextField(
                                    controller: _mainCssController,
                                    minLines: 3,
                                    maxLines: 8,
                                    decoration: const InputDecoration(
                                      labelText: 'main.css',
                                      alignLabelWithHint: true,
                                    ),
                                  ),
                                  const SizedBox(height: 10),
                                  styleTemplates.when(
                                    data: (templates) => _StyleTemplateDropdown(
                                      templates: templates,
                                      onSelected: (template) {
                                        setState(() {
                                          _mainCssController.text =
                                              template.css;
                                          _status = '已应用样式模板：${template.name}';
                                        });
                                      },
                                    ),
                                    error: (error, stackTrace) =>
                                        Text('样式模板读取失败：$error'),
                                    loading: () =>
                                        const LinearProgressIndicator(),
                                  ),
                                  const SizedBox(height: 12),
                                  TextField(
                                    controller: _fontCssController,
                                    minLines: 2,
                                    maxLines: 6,
                                    decoration: const InputDecoration(
                                      labelText: 'font.css',
                                      alignLabelWithHint: true,
                                    ),
                                  ),
                                ],
                              ),
                            ),
                          ),
                        ],
                      ),
                      if (_busy) ...[
                        const SizedBox(height: 12),
                        const LinearProgressIndicator(),
                      ],
                    ],
                  ),
                ),
              ),
              const Divider(height: 1),
              Padding(
                padding: const EdgeInsets.fromLTRB(18, 12, 18, 16),
                child: Row(
                  mainAxisAlignment: MainAxisAlignment.end,
                  children: [
                    TextButton(
                      onPressed:
                          _busy ? null : () => Navigator.of(context).pop(),
                      child: const Text('取消'),
                    ),
                    const SizedBox(width: 10),
                    FilledButton.icon(
                      onPressed: _busy
                          ? null
                          : () async {
                              await _buildEpub(service, chapters);
                            },
                      icon: const Icon(Icons.auto_awesome_motion_outlined),
                      label: const Text('生成 EPUB'),
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  Future<void> _buildEpub(
    EpubMakeService service,
    List<EpubChapter> chapters,
  ) async {
    setState(() {
      _busy = true;
      _status = '正在生成 EPUB...';
    });

    try {
      final result = await service.buildFromEditor(
        title: _titleController.text.trim().isEmpty
            ? widget.document.title
            : _titleController.text.trim(),
        author: _authorController.text.trim(),
        publisher: _publisherController.text.trim(),
        date: _dateController.text.trim(),
        description: _descriptionController.text.trim(),
        tags: _tagsController.text
            .split(RegExp(r'[,，;；\s]+'))
            .map((tag) => tag.trim())
            .where((tag) => tag.isNotEmpty)
            .toList(),
        mainCss: _mainCssController.text,
        fontCss: _fontCssController.text,
        uuid: _uuidController.text.trim(),
        sourceName: widget.document.filePath == null
            ? '${widget.document.title}.txt'
            : p.basename(widget.document.filePath!),
        content: widget.document.content,
        coverPath: _coverPath,
        chapters: chapters,
        fonts:
            await (await ref.read(fontAssetServiceProvider.future)).listFonts(),
      );
      if (!mounted) {
        return;
      }
      setState(() {
        _status = result == null ? '已取消生成。' : '已生成：${result.outputPath}';
      });
    } catch (error) {
      if (!mounted) {
        return;
      }
      setState(() => _status = '生成失败：$error');
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  Future<void> _searchCovers(EpubMakeService service) async {
    setState(() {
      _coverBusy = true;
      _status = '正在搜索封面...';
    });
    try {
      final results = await service.searchCovers(
        title: _titleController.text.trim().isEmpty
            ? widget.document.title
            : _titleController.text.trim(),
        author: _authorController.text.trim(),
      );
      if (!mounted) {
        return;
      }
      setState(() {
        _coverResults = results;
        _status =
            results.isEmpty ? '没有找到合适的封面。' : '找到 ${results.length} 个封面结果。';
      });
    } catch (error) {
      if (!mounted) {
        return;
      }
      setState(() => _status = '封面搜索失败：$error');
    } finally {
      if (mounted) {
        setState(() => _coverBusy = false);
      }
    }
  }

  Future<void> _applyRemoteCover(
    EpubMakeService service,
    CoverSearchResult result,
  ) async {
    setState(() {
      _coverBusy = true;
      _status = '正在下载封面...';
    });
    try {
      final path = await service.downloadCoverToCache(result);
      if (!mounted) {
        return;
      }
      setState(() {
        _coverPath = path;
        _status = '已应用封面：${result.title}';
      });
    } catch (error) {
      if (!mounted) {
        return;
      }
      setState(() => _status = '封面下载失败：$error');
    } finally {
      if (mounted) {
        setState(() => _coverBusy = false);
      }
    }
  }

  List<EpubChapter> _editorChaptersToEpub(EditorDocument document) {
    if (document.chapters.isEmpty) {
      return const [];
    }
    return [
      for (var index = 0; index < document.chapters.length; index += 1)
        EpubChapter(
          id: document.chapters[index].id,
          title: document.chapters[index].title,
          lineNumber: document.chapters[index].lineNumber,
          level: document.chapters[index].level == 1 ? 1 : 3,
          isMeta: _isMetaTitle(document.chapters[index].title),
          wordCount: _chapterWordCount(document, index),
        ),
    ];
  }

  int _chapterWordCount(EditorDocument document, int index) {
    final start = document.chapters[index].offset;
    final end = index + 1 < document.chapters.length
        ? document.chapters[index + 1].offset
        : document.content.length;
    return document.content.substring(start, end).trim().runes.length;
  }

  bool _isMetaTitle(String title) {
    if (title.contains('卷') || title.contains('部')) {
      return false;
    }
    return RegExp(r'^(书名|作者|简介|序章|楔子|引子|前言|后记|尾声)$').hasMatch(title.trim());
  }
}

class _CoverResultCard extends StatelessWidget {
  const _CoverResultCard({
    required this.result,
    required this.onTap,
  });

  final CoverSearchResult result;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: 112,
      child: InkWell(
        borderRadius: BorderRadius.circular(14),
        onTap: onTap,
        child: DecoratedBox(
          decoration: BoxDecoration(
            border: Border.all(
              color: result.preferred ? AppTheme.brand : AppTheme.border,
            ),
            borderRadius: BorderRadius.circular(14),
          ),
          child: Padding(
            padding: const EdgeInsets.all(6),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Expanded(
                  child: ClipRRect(
                    borderRadius: BorderRadius.circular(10),
                    child: Image.network(
                      result.imageUrl,
                      fit: BoxFit.cover,
                      errorBuilder: (context, error, stackTrace) {
                        return const ColoredBox(
                          color: AppTheme.panelSoft,
                          child: Icon(Icons.broken_image_outlined),
                        );
                      },
                    ),
                  ),
                ),
                const SizedBox(height: 6),
                Text(
                  result.title,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: Theme.of(context).textTheme.labelSmall,
                ),
                Text(
                  result.source,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: Theme.of(context).textTheme.labelSmall?.copyWith(
                        color: AppTheme.muted,
                      ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _StyleTemplateDropdown extends StatelessWidget {
  const _StyleTemplateDropdown({
    required this.templates,
    required this.onSelected,
  });

  final List<StyleTemplate> templates;
  final ValueChanged<StyleTemplate> onSelected;

  @override
  Widget build(BuildContext context) {
    if (templates.isEmpty) {
      return const Align(
        alignment: Alignment.centerLeft,
        child: Text('暂无样式模板。'),
      );
    }
    return DropdownButtonFormField<String>(
      decoration: const InputDecoration(
        labelText: '套用样式模板',
        isDense: true,
      ),
      items: [
        for (final template in templates)
          DropdownMenuItem(
            value: template.id,
            child: Text(template.name),
          ),
      ],
      onChanged: (id) {
        if (id == null) {
          return;
        }
        onSelected(templates.firstWhere((template) => template.id == id));
      },
    );
  }
}

class _InfoBanner extends StatelessWidget {
  const _InfoBanner({required this.text});

  final String text;

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      decoration: BoxDecoration(
        color: AppTheme.panelSoft.withValues(alpha: 0.68),
        border: Border.all(color: AppTheme.border),
        borderRadius: BorderRadius.circular(14),
      ),
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Text(text),
      ),
    );
  }
}
