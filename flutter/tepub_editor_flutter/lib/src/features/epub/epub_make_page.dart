import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:path/path.dart' as p;

import '../../ui/theme/app_theme.dart';
import '../../ui/widgets/app_page.dart';
import '../../ui/widgets/app_surface.dart';
import '../settings/models/style_template.dart';
import '../settings/services/style_template_service.dart';
import 'models/epub_project.dart';
import 'providers/epub_make_controller.dart';

class EpubMakePage extends ConsumerWidget {
  const EpubMakePage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final project = ref.watch(epubMakeControllerProvider);
    final controller = ref.read(epubMakeControllerProvider.notifier);

    return AppPage(
      title: '制作 EPUB',
      subtitle: project.sourcePath.isEmpty
          ? '导入 TXT、MD 或 HTML 后扫描目录，填写元数据并生成 EPUB。'
          : project.sourcePath,
      actions: [
        OutlinedButton.icon(
          onPressed: project.busy ? null : controller.pickSource,
          icon: const Icon(Icons.upload_file),
          label: const Text('导入文本'),
        ),
        OutlinedButton.icon(
          onPressed:
              project.busy || !project.hasSource ? null : controller.pickCover,
          icon: const Icon(Icons.image_outlined),
          label: const Text('选择封面'),
        ),
        FilledButton.icon(
          onPressed:
              project.busy || !project.hasSource ? null : controller.buildEpub,
          icon: const Icon(Icons.auto_awesome_motion_outlined),
          label: const Text('生成 EPUB'),
        ),
      ],
      child: LayoutBuilder(
        builder: (context, constraints) {
          final compact = constraints.maxWidth < 1080;
          final left = ListView(
            padding: EdgeInsets.zero,
            children: [
              _MetadataCard(project: project, controller: controller),
              const SizedBox(height: 16),
              _RuleCard(project: project, controller: controller),
            ],
          );
          final right = ListView(
            padding: EdgeInsets.zero,
            children: [
              _TocPreviewCard(project: project, controller: controller),
              const SizedBox(height: 16),
              _ResultCard(project: project, controller: controller),
            ],
          );

          if (compact) {
            return ListView(
              padding: EdgeInsets.zero,
              children: [
                _MetadataCard(project: project, controller: controller),
                const SizedBox(height: 16),
                _RuleCard(project: project, controller: controller),
                const SizedBox(height: 16),
                _TocPreviewCard(project: project, controller: controller),
                const SizedBox(height: 16),
                _ResultCard(project: project, controller: controller),
              ],
            );
          }

          return Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              SizedBox(width: 420, child: left),
              const SizedBox(width: 16),
              Expanded(child: right),
            ],
          );
        },
      ),
    );
  }
}

class _MetadataCard extends ConsumerWidget {
  const _MetadataCard({
    required this.project,
    required this.controller,
  });

  final EpubProject project;
  final EpubMakeController controller;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final templates = ref.watch(styleTemplatesProvider);
    return AppSurface(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _SectionTitle(
            title: '图书信息',
            subtitle:
                project.sourceName.isEmpty ? '尚未导入文本' : project.sourceName,
          ),
          const SizedBox(height: 14),
          TextFormField(
            initialValue: project.title,
            onChanged: controller.updateTitle,
            decoration: const InputDecoration(labelText: '书名'),
          ),
          const SizedBox(height: 12),
          TextFormField(
            initialValue: project.author,
            onChanged: controller.updateAuthor,
            decoration: const InputDecoration(labelText: '作者'),
          ),
          const SizedBox(height: 12),
          TextFormField(
            initialValue: project.uuid,
            onChanged: controller.updateUuid,
            decoration: const InputDecoration(labelText: 'UUID'),
          ),
          const SizedBox(height: 14),
          _InfoLine(
            label: '封面',
            value: project.coverPath.isEmpty
                ? '未选择'
                : p.basename(project.coverPath),
          ),
          const SizedBox(height: 10),
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: [
              OutlinedButton.icon(
                onPressed:
                    project.coverSearching ? null : controller.searchCovers,
                icon: const Icon(Icons.travel_explore),
                label: Text(project.coverSearching ? '搜索中...' : '搜索封面'),
              ),
              OutlinedButton.icon(
                onPressed: project.coverSearching ? null : controller.pickCover,
                icon: const Icon(Icons.image_outlined),
                label: const Text('本地图片'),
              ),
            ],
          ),
          if (project.coverResults.isNotEmpty) ...[
            const SizedBox(height: 12),
            SizedBox(
              height: 180,
              child: ListView.separated(
                scrollDirection: Axis.horizontal,
                itemCount: project.coverResults.length,
                separatorBuilder: (context, index) => const SizedBox(width: 10),
                itemBuilder: (context, index) {
                  final result = project.coverResults[index];
                  return _CoverResultCard(
                    result: result,
                    onTap: () => controller.applyRemoteCover(result),
                  );
                },
              ),
            ),
          ],
          _InfoLine(label: '字数', value: '${project.wordCount}'),
          _InfoLine(label: '字体', value: '${project.fonts.length} 个已导入字体'),
          const SizedBox(height: 10),
          templates.when(
            data: (value) => _StyleTemplatePicker(
              templates: value,
              onSelected: controller.applyStyleTemplate,
            ),
            error: (error, stackTrace) => Text('样式模板读取失败：$error'),
            loading: () => const LinearProgressIndicator(),
          ),
        ],
      ),
    );
  }
}

class _StyleTemplatePicker extends StatelessWidget {
  const _StyleTemplatePicker({
    required this.templates,
    required this.onSelected,
  });

  final List<StyleTemplate> templates;
  final ValueChanged<String> onSelected;

  @override
  Widget build(BuildContext context) {
    if (templates.isEmpty) {
      return const Text('暂无样式模板。');
    }
    return DropdownButtonFormField<String>(
      decoration: const InputDecoration(
        labelText: '应用样式模板',
        isDense: true,
      ),
      items: [
        for (final template in templates)
          DropdownMenuItem(
            value: template.id,
            child: Text(template.name),
          ),
      ],
      onChanged: (value) {
        if (value != null) {
          onSelected(value);
        }
      },
    );
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

class _RuleCard extends StatelessWidget {
  const _RuleCard({
    required this.project,
    required this.controller,
  });

  final EpubProject project;
  final EpubMakeController controller;

  @override
  Widget build(BuildContext context) {
    return AppSurface(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _SectionTitle(
            title: '目录规则',
            subtitle: '${project.rules.length} 条正则，按顺序匹配',
          ),
          const SizedBox(height: 12),
          for (var index = 0; index < project.rules.length; index += 1)
            Padding(
              padding: const EdgeInsets.only(bottom: 10),
              child: _RuleRow(
                index: index,
                rule: project.rules[index],
                onPatternChanged: controller.updateRulePattern,
                onLevelChanged: controller.updateRuleLevel,
                onRemove: controller.removeRule,
              ),
            ),
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: [
              OutlinedButton.icon(
                onPressed: () => controller.addRule(1),
                icon: const Icon(Icons.add),
                label: const Text('添加卷规则'),
              ),
              OutlinedButton.icon(
                onPressed: () => controller.addRule(3),
                icon: const Icon(Icons.add),
                label: const Text('添加章节规则'),
              ),
              FilledButton.icon(
                onPressed: project.hasSource ? controller.scanToc : null,
                icon: const Icon(Icons.refresh),
                label: const Text('重新扫描'),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

class _RuleRow extends StatelessWidget {
  const _RuleRow({
    required this.index,
    required this.rule,
    required this.onPatternChanged,
    required this.onLevelChanged,
    required this.onRemove,
  });

  final int index;
  final EpubTocRule rule;
  final void Function(int index, String value) onPatternChanged;
  final void Function(int index, int value) onLevelChanged;
  final void Function(int index) onRemove;

  @override
  Widget build(BuildContext context) {
    return Row(
      children: [
        SizedBox(
          width: 92,
          child: DropdownButtonFormField<int>(
            initialValue: rule.level,
            decoration: const InputDecoration(isDense: true),
            items: const [
              DropdownMenuItem(value: 1, child: Text('卷')),
              DropdownMenuItem(value: 3, child: Text('章节')),
            ],
            onChanged: (value) {
              if (value != null) {
                onLevelChanged(index, value);
              }
            },
          ),
        ),
        const SizedBox(width: 8),
        Expanded(
          child: TextFormField(
            initialValue: rule.pattern,
            onChanged: (value) => onPatternChanged(index, value),
            decoration: const InputDecoration(isDense: true),
          ),
        ),
        IconButton(
          tooltip: '删除规则',
          onPressed: () => onRemove(index),
          icon: const Icon(Icons.close),
        ),
      ],
    );
  }
}

class _TocPreviewCard extends StatelessWidget {
  const _TocPreviewCard({
    required this.project,
    required this.controller,
  });

  final EpubProject project;
  final EpubMakeController controller;

  @override
  Widget build(BuildContext context) {
    return AppSurface(
      child: SizedBox(
        height: 520,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            _SectionTitle(
              title: '目录预览',
              subtitle: project.status,
              trailing: Text('${project.chapters.length} 项'),
            ),
            const SizedBox(height: 12),
            if (project.error != null)
              Padding(
                padding: const EdgeInsets.only(bottom: 10),
                child: Text(
                  project.error!,
                  style: const TextStyle(color: Colors.redAccent),
                ),
              ),
            Expanded(
              child: project.chapters.isEmpty
                  ? const Center(child: Text('导入文本后会在这里显示目录。'))
                  : ListView.builder(
                      itemCount: project.chapters.length,
                      itemBuilder: (context, index) {
                        final chapter = project.chapters[index];
                        return ListTile(
                          dense: true,
                          contentPadding: EdgeInsets.only(
                            left: chapter.level == 1 ? 0 : 18,
                            right: 4,
                          ),
                          leading: Icon(
                            chapter.level == 1
                                ? Icons.folder_outlined
                                : Icons.article_outlined,
                            size: 20,
                          ),
                          title: Text(
                            chapter.title,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                          ),
                          subtitle: Text(
                            '第 ${chapter.lineNumber} 行 · ${chapter.wordCount} 字',
                          ),
                          trailing: IconButton(
                            tooltip: '编辑章节',
                            icon: const Icon(Icons.edit_outlined),
                            onPressed: () => _showChapterEditor(
                              context,
                              index,
                              project,
                              controller,
                            ),
                          ),
                        );
                      },
                    ),
            ),
          ],
        ),
      ),
    );
  }

  Future<void> _showChapterEditor(
    BuildContext context,
    int index,
    EpubProject project,
    EpubMakeController controller,
  ) async {
    final chapter = project.chapters[index];
    final titleController = TextEditingController(text: chapter.title);
    final bodyController = TextEditingController(
      text: _chapterBody(project, index),
    );
    await showDialog<void>(
      context: context,
      builder: (context) => Dialog(
        backgroundColor: Colors.transparent,
        insetPadding: const EdgeInsets.all(24),
        child: AppSurface(
          padding: EdgeInsets.zero,
          child: SizedBox(
            width: 720,
            height: 620,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Padding(
                  padding: const EdgeInsets.fromLTRB(20, 18, 14, 12),
                  child: Row(
                    children: [
                      Expanded(
                        child: Text(
                          '编辑章节',
                          style:
                              Theme.of(context).textTheme.titleLarge?.copyWith(
                                    fontWeight: FontWeight.w800,
                                  ),
                        ),
                      ),
                      IconButton(
                        tooltip: '关闭',
                        onPressed: () => Navigator.of(context).pop(),
                        icon: const Icon(Icons.close),
                      ),
                    ],
                  ),
                ),
                const Divider(height: 1),
                Expanded(
                  child: Padding(
                    padding: const EdgeInsets.all(18),
                    child: Column(
                      children: [
                        TextField(
                          controller: titleController,
                          decoration: const InputDecoration(labelText: '目录标题'),
                        ),
                        const SizedBox(height: 12),
                        Expanded(
                          child: TextField(
                            controller: bodyController,
                            expands: true,
                            maxLines: null,
                            minLines: null,
                            textAlignVertical: TextAlignVertical.top,
                            decoration: const InputDecoration(
                              labelText: '本章正文',
                              alignLabelWithHint: true,
                            ),
                          ),
                        ),
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
                        onPressed: () => Navigator.of(context).pop(),
                        child: const Text('取消'),
                      ),
                      const SizedBox(width: 10),
                      FilledButton.icon(
                        onPressed: () {
                          controller.saveChapterEdit(
                            index,
                            titleController.text,
                            bodyController.text,
                          );
                          Navigator.of(context).pop();
                        },
                        icon: const Icon(Icons.check),
                        label: const Text('保存'),
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
    titleController.dispose();
    bodyController.dispose();
  }

  String _chapterBody(EpubProject project, int index) {
    final lines = project.content.split('\n');
    final chapter = project.chapters[index];
    final start = chapter.lineNumber;
    final endExclusive = index + 1 < project.chapters.length
        ? project.chapters[index + 1].lineNumber - 1
        : lines.length;
    if (start < 0 || start > lines.length || endExclusive < start) {
      return '';
    }
    return lines.sublist(start, endExclusive).join('\n');
  }
}

class _ResultCard extends StatelessWidget {
  const _ResultCard({
    required this.project,
    required this.controller,
  });

  final EpubProject project;
  final EpubMakeController controller;

  @override
  Widget build(BuildContext context) {
    final result = project.result;
    return AppSurface(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          const _SectionTitle(title: '生成结果', subtitle: '生成后会显示输出路径'),
          const SizedBox(height: 12),
          if (project.busy) const LinearProgressIndicator(),
          if (result == null && !project.busy)
            const Text('尚未生成 EPUB。')
          else if (result != null) ...[
            _InfoLine(label: '书名', value: result.title),
            _InfoLine(label: '目录', value: '${result.chapterCount} 项'),
            _InfoLine(label: '字数', value: '${result.wordCount}'),
            const SizedBox(height: 8),
            SelectableText(result.outputPath),
            const SizedBox(height: 10),
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: [
                OutlinedButton.icon(
                  onPressed: () => controller.openGeneratedFile(),
                  icon: const Icon(Icons.auto_stories_outlined),
                  label: const Text('打开 EPUB'),
                ),
                OutlinedButton.icon(
                  onPressed: () => controller.revealGeneratedFile(),
                  icon: const Icon(Icons.folder_open_outlined),
                  label: const Text('打开文件位置'),
                ),
              ],
            ),
          ],
        ],
      ),
    );
  }
}

class _SectionTitle extends StatelessWidget {
  const _SectionTitle({
    required this.title,
    required this.subtitle,
    this.trailing,
  });

  final String title;
  final String subtitle;
  final Widget? trailing;

  @override
  Widget build(BuildContext context) {
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                title,
                style: Theme.of(context).textTheme.titleMedium?.copyWith(
                      fontWeight: FontWeight.w800,
                      color: AppTheme.ink,
                    ),
              ),
              const SizedBox(height: 3),
              Text(
                subtitle,
                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                      color: AppTheme.muted,
                    ),
              ),
            ],
          ),
        ),
        if (trailing != null) trailing!,
      ],
    );
  }
}

class _InfoLine extends StatelessWidget {
  const _InfoLine({
    required this.label,
    required this.value,
  });

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 3),
      child: Row(
        children: [
          SizedBox(
            width: 56,
            child: Text(label, style: const TextStyle(color: AppTheme.muted)),
          ),
          Expanded(
            child: Text(
              value,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
            ),
          ),
        ],
      ),
    );
  }
}
