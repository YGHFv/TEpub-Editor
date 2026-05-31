import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../ui/theme/app_theme.dart';
import '../../../ui/widgets/app_surface.dart';
import '../models/editor_document.dart';
import '../providers/editor_controller.dart';

class EditorSidebar extends ConsumerStatefulWidget {
  const EditorSidebar({super.key});

  @override
  ConsumerState<EditorSidebar> createState() => _EditorSidebarState();
}

class _EditorSidebarState extends ConsumerState<EditorSidebar> {
  late final ScrollController _scrollController;
  String? _lastSelectedChapterId;

  @override
  void initState() {
    super.initState();
    _scrollController = ScrollController();
  }

  @override
  void dispose() {
    _scrollController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final document = ref.watch(
      editorControllerProvider.select(
        (document) => (
          chapters: document.chapters,
          selectedChapterId: document.selectedChapterId,
          display: document.display,
          toc: document.toc,
          history: document.history,
          checkReport: document.checkReport,
        ),
      ),
    );
    final controller = ref.read(editorControllerProvider.notifier);
    final visibleChapters = _visibleChapters(document.chapters);
    final hasCollapsed = document.chapters.any((chapter) => chapter.collapsed);
    final selectedId = document.selectedChapterId;
    if (selectedId != null && selectedId != _lastSelectedChapterId) {
      _lastSelectedChapterId = selectedId;
      final selectedIndex = visibleChapters.indexWhere(
        (chapter) => chapter.id == selectedId,
      );
      if (selectedIndex >= 0) {
        WidgetsBinding.instance.addPostFrameCallback((_) {
          if (!_scrollController.hasClients) {
            return;
          }
          final target = (selectedIndex * 48.0).clamp(
            0.0,
            _scrollController.position.maxScrollExtent,
          );
          _scrollController.animateTo(
            target,
            duration: const Duration(milliseconds: 160),
            curve: Curves.easeOutCubic,
          );
        });
      }
    }

    return AppSurface(
      padding: const EdgeInsets.all(8),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Padding(
            padding: const EdgeInsets.fromLTRB(10, 10, 6, 8),
            child: Row(
              children: [
                Expanded(
                  child: Text(
                    '目录',
                    style: Theme.of(context).textTheme.titleMedium?.copyWith(
                          fontWeight: FontWeight.w800,
                          color: AppTheme.ink,
                        ),
                  ),
                ),
                Text(
                  '${document.chapters.length}',
                  style: Theme.of(context).textTheme.labelMedium?.copyWith(
                        color: AppTheme.muted,
                      ),
                ),
                const SizedBox(width: 4),
                IconButton(
                  tooltip: hasCollapsed ? '展开全部' : '折叠全部',
                  visualDensity: VisualDensity.compact,
                  onPressed: controller.toggleAllChapters,
                  icon: Icon(
                    hasCollapsed ? Icons.unfold_more : Icons.unfold_less,
                    size: 20,
                  ),
                ),
              ],
            ),
          ),
          Expanded(
            child: Scrollbar(
              controller: _scrollController,
              thumbVisibility: true,
              child: ListView.builder(
                controller: _scrollController,
                padding: const EdgeInsets.only(right: 10, bottom: 8),
                itemCount: visibleChapters.length,
                itemBuilder: (context, index) {
                  final chapter = visibleChapters[index];
                  return _ChapterTile(
                    chapter: chapter,
                    selected: chapter.id == document.selectedChapterId,
                    hasChildren: _hasChildren(document.chapters, chapter),
                    onTap: () => controller.selectChapter(chapter.id),
                    onToggle: () => controller.toggleChapterCollapsed(
                      chapter.id,
                    ),
                  );
                },
              ),
            ),
          ),
        ],
      ),
    );
  }

  List<EditorChapter> _visibleChapters(List<EditorChapter> chapters) {
    final hiddenLevels = <int>[];
    final visible = <EditorChapter>[];

    for (final chapter in chapters) {
      hiddenLevels.removeWhere((level) => level >= chapter.level);
      if (hiddenLevels.isNotEmpty) {
        continue;
      }
      visible.add(chapter);
      if (chapter.collapsed) {
        hiddenLevels.add(chapter.level);
      }
    }

    return visible;
  }

  bool _hasChildren(List<EditorChapter> chapters, EditorChapter chapter) {
    final index = chapters.indexWhere((item) => item.id == chapter.id);
    if (index < 0 || index == chapters.length - 1) {
      return false;
    }
    return chapters[index + 1].level > chapter.level;
  }
}

class EditorSettingsPanel extends ConsumerStatefulWidget {
  const EditorSettingsPanel({super.key});

  @override
  ConsumerState<EditorSettingsPanel> createState() =>
      _EditorSettingsPanelState();
}

class _EditorSettingsPanelState extends ConsumerState<EditorSettingsPanel> {
  late final TextEditingController _tocPatternController;

  @override
  void initState() {
    super.initState();
    _tocPatternController = TextEditingController();
  }

  @override
  void dispose() {
    _tocPatternController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final document = ref.watch(editorControllerProvider);
    final controller = ref.read(editorControllerProvider.notifier);

    if (_tocPatternController.text != document.toc.pattern) {
      _tocPatternController.text = document.toc.pattern;
      _tocPatternController.selection = TextSelection.collapsed(
        offset: _tocPatternController.text.length,
      );
    }

    return AppSurface(
      padding: EdgeInsets.zero,
      child: SizedBox(
        width: 460,
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
                      '编辑器设置',
                      style: Theme.of(context).textTheme.titleLarge?.copyWith(
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
            Padding(
              padding: const EdgeInsets.all(18),
              child: Column(
                children: [
                  _SettingsSection(
                    title: '显示设置',
                    subtitle: '控制编辑区字号、行高与换行行为。',
                    child: _DisplaySettingsBox(
                      settings: document.display,
                      onFontSizeChanged: controller.updateFontSize,
                      onLineHeightChanged: controller.updateLineHeight,
                      onWordWrapChanged: controller.setWordWrap,
                      onWhitespaceChanged: controller.setShowWhitespace,
                      onLineBreakChanged: controller.setShowLineBreaks,
                    ),
                  ),
                  const SizedBox(height: 14),
                  _SettingsSection(
                    title: '目录识别',
                    subtitle: '默认只识别顶行第 X 卷 / 第 X 章；其它格式可启用正则。',
                    child: _TocPatternBox(
                      controller: _tocPatternController,
                      toc: document.toc,
                      onToggle: controller.setCustomTocPattern,
                      onChanged: controller.updateTocPattern,
                    ),
                  ),
                  const SizedBox(height: 14),
                  _SettingsSection(
                    title: '章节检查',
                    subtitle: document.checkReport.totalIssues == 0
                        ? '当前未发现章节问题'
                        : '发现 ${document.checkReport.totalIssues} 个可检查项',
                    child: _ChapterCheckBox(
                      settings: document.display,
                      report: document.checkReport,
                      onMinChanged: controller.updateWordCountMin,
                      onMaxChanged: controller.updateWordCountMax,
                      onRun: controller.runChapterCheck,
                      onJump: controller.jumpToCheckIssue,
                    ),
                  ),
                  const SizedBox(height: 14),
                  _SettingsSection(
                    title: '历史版本',
                    subtitle: document.history.isEmpty
                        ? '暂无快照'
                        : '已保存 ${document.history.length} 个快照',
                    child: _HistorySnapshotList(
                      snapshots: document.history,
                      onCreate: controller.createHistorySnapshot,
                      onRestore: controller.restoreHistorySnapshot,
                    ),
                  ),
                ],
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
                      controller.applyTocPattern();
                      Navigator.of(context).pop();
                    },
                    icon: const Icon(Icons.check),
                    label: const Text('应用'),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _DisplaySettingsBox extends StatelessWidget {
  const _DisplaySettingsBox({
    required this.settings,
    required this.onFontSizeChanged,
    required this.onLineHeightChanged,
    required this.onWordWrapChanged,
    required this.onWhitespaceChanged,
    required this.onLineBreakChanged,
  });

  final EditorDisplaySettings settings;
  final ValueChanged<double> onFontSizeChanged;
  final ValueChanged<double> onLineHeightChanged;
  final ValueChanged<bool> onWordWrapChanged;
  final ValueChanged<bool> onWhitespaceChanged;
  final ValueChanged<bool> onLineBreakChanged;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        _SliderRow(
          label: '字号',
          value: settings.fontSize,
          min: 12,
          max: 32,
          divisions: 20,
          display: '${settings.fontSize.round()}',
          onChanged: onFontSizeChanged,
        ),
        _SliderRow(
          label: '行高',
          value: settings.lineHeight,
          min: 1.2,
          max: 2.4,
          divisions: 12,
          display: settings.lineHeight.toStringAsFixed(1),
          onChanged: onLineHeightChanged,
        ),
        SwitchListTile(
          dense: true,
          contentPadding: EdgeInsets.zero,
          title: const Text('自动换行'),
          value: settings.wordWrap,
          onChanged: onWordWrapChanged,
        ),
        SwitchListTile(
          dense: true,
          contentPadding: EdgeInsets.zero,
          title: const Text('显示空白符'),
          subtitle: const Text('用浅色符号显示空格和 Tab，不改写正文内容。'),
          value: settings.showWhitespace,
          onChanged: onWhitespaceChanged,
        ),
        SwitchListTile(
          dense: true,
          contentPadding: EdgeInsets.zero,
          title: const Text('显示换行符'),
          subtitle: const Text('在行尾显示换行标记，便于排查断行和章节格式。'),
          value: settings.showLineBreaks,
          onChanged: onLineBreakChanged,
        ),
      ],
    );
  }
}

class _ChapterCheckBox extends StatelessWidget {
  const _ChapterCheckBox({
    required this.settings,
    required this.report,
    required this.onMinChanged,
    required this.onMaxChanged,
    required this.onRun,
    required this.onJump,
  });

  final EditorDisplaySettings settings;
  final EditorCheckReport report;
  final ValueChanged<double> onMinChanged;
  final ValueChanged<double> onMaxChanged;
  final VoidCallback onRun;
  final ValueChanged<EditorCheckIssue> onJump;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        _SliderRow(
          label: '最少字数',
          value: settings.wordCountMin.toDouble(),
          min: 0,
          max: 6000,
          divisions: 60,
          display: '${settings.wordCountMin}',
          onChanged: onMinChanged,
        ),
        _SliderRow(
          label: '最多字数',
          value: settings.wordCountMax.toDouble(),
          min: 1000,
          max: 20000,
          divisions: 38,
          display: '${settings.wordCountMax}',
          onChanged: onMaxChanged,
        ),
        Align(
          alignment: Alignment.centerLeft,
          child: FilledButton.tonalIcon(
            onPressed: onRun,
            icon: const Icon(Icons.playlist_add_check),
            label: const Text('运行检查'),
          ),
        ),
        const SizedBox(height: 10),
        _IssueGroup(
          title: '断序检查',
          issues: report.sequenceIssues,
          onJump: onJump,
        ),
        _IssueGroup(
          title: '标题内容',
          issues: report.titleIssues,
          onJump: onJump,
        ),
        _IssueGroup(
          title: '字数检查',
          issues: report.wordCountIssues,
          onJump: onJump,
        ),
      ],
    );
  }
}

class _IssueGroup extends StatelessWidget {
  const _IssueGroup({
    required this.title,
    required this.issues,
    required this.onJump,
  });

  final String title;
  final List<EditorCheckIssue> issues;
  final ValueChanged<EditorCheckIssue> onJump;

  @override
  Widget build(BuildContext context) {
    return ExpansionTile(
      tilePadding: EdgeInsets.zero,
      title: Text('$title (${issues.length})'),
      children: [
        if (issues.isEmpty)
          const Align(
            alignment: Alignment.centerLeft,
            child: Padding(
              padding: EdgeInsets.only(bottom: 8),
              child: Text('无'),
            ),
          )
        else
          for (final issue in issues)
            ListTile(
              dense: true,
              contentPadding: EdgeInsets.zero,
              title: Text(issue.title,
                  maxLines: 1, overflow: TextOverflow.ellipsis),
              subtitle: Text('第 ${issue.lineNumber} 行 · ${issue.message}'),
              trailing: TextButton(
                onPressed: () => onJump(issue),
                child: const Text('定位'),
              ),
            ),
      ],
    );
  }
}

class _SliderRow extends StatelessWidget {
  const _SliderRow({
    required this.label,
    required this.value,
    required this.min,
    required this.max,
    required this.divisions,
    required this.display,
    required this.onChanged,
  });

  final String label;
  final double value;
  final double min;
  final double max;
  final int divisions;
  final String display;
  final ValueChanged<double> onChanged;

  @override
  Widget build(BuildContext context) {
    return Row(
      children: [
        SizedBox(width: 70, child: Text(label)),
        Expanded(
          child: Slider(
            value: value.clamp(min, max),
            min: min,
            max: max,
            divisions: divisions,
            label: display,
            onChanged: onChanged,
          ),
        ),
        SizedBox(
          width: 48,
          child: Text(display, textAlign: TextAlign.right),
        ),
      ],
    );
  }
}

class _HistorySnapshotList extends StatelessWidget {
  const _HistorySnapshotList({
    required this.snapshots,
    required this.onCreate,
    required this.onRestore,
  });

  final List<EditorHistorySnapshot> snapshots;
  final Future<void> Function() onCreate;
  final Future<void> Function(EditorHistorySnapshot snapshot) onRestore;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Align(
          alignment: Alignment.centerLeft,
          child: FilledButton.tonalIcon(
            onPressed: onCreate,
            icon: const Icon(Icons.add),
            label: const Text('创建快照'),
          ),
        ),
        if (snapshots.isEmpty) ...[
          const SizedBox(height: 10),
          Text(
            '创建快照后，可以从这里恢复到之前的正文内容。',
            style: Theme.of(context).textTheme.bodySmall?.copyWith(
                  color: AppTheme.muted,
                ),
          ),
        ] else ...[
          const SizedBox(height: 10),
          ConstrainedBox(
            constraints: const BoxConstraints(maxHeight: 220),
            child: ListView.separated(
              shrinkWrap: true,
              itemCount: snapshots.length,
              separatorBuilder: (context, index) => const Divider(height: 1),
              itemBuilder: (context, index) {
                final snapshot = snapshots[index];
                return ListTile(
                  contentPadding: EdgeInsets.zero,
                  dense: true,
                  title: Text(
                    snapshot.title,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                  subtitle: Text('${snapshot.contentLength} 字符'),
                  trailing: TextButton(
                    onPressed: () => onRestore(snapshot),
                    child: const Text('恢复'),
                  ),
                );
              },
            ),
          ),
        ],
      ],
    );
  }
}

class _SettingsSection extends StatelessWidget {
  const _SettingsSection({
    required this.title,
    required this.subtitle,
    required this.child,
  });

  final String title;
  final String subtitle;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      decoration: BoxDecoration(
        color: AppTheme.panelSoft.withValues(alpha: 0.5),
        border: Border.all(color: AppTheme.border),
        borderRadius: BorderRadius.circular(16),
      ),
      child: Padding(
        padding: const EdgeInsets.all(14),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(
              title,
              style: Theme.of(context).textTheme.titleMedium?.copyWith(
                    fontWeight: FontWeight.w800,
                  ),
            ),
            const SizedBox(height: 4),
            Text(
              subtitle,
              style: Theme.of(context).textTheme.bodySmall?.copyWith(
                    color: AppTheme.muted,
                    height: 1.35,
                  ),
            ),
            const SizedBox(height: 12),
            child,
          ],
        ),
      ),
    );
  }
}

class _TocPatternBox extends StatelessWidget {
  const _TocPatternBox({
    required this.controller,
    required this.toc,
    required this.onToggle,
    required this.onChanged,
  });

  final TextEditingController controller;
  final EditorTocState toc;
  final ValueChanged<bool> onToggle;
  final ValueChanged<String> onChanged;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        SwitchListTile(
          dense: true,
          contentPadding: EdgeInsets.zero,
          title: const Text('启用自定义正则'),
          value: toc.useCustomPattern,
          onChanged: onToggle,
        ),
        TextField(
          controller: controller,
          enabled: toc.useCustomPattern,
          minLines: 1,
          maxLines: 3,
          onChanged: onChanged,
          decoration: InputDecoration(
            isDense: true,
            hintText: EditorTocState.defaultPattern,
            errorText: toc.error,
          ),
        ),
      ],
    );
  }
}

class _ChapterTile extends StatelessWidget {
  const _ChapterTile({
    required this.chapter,
    required this.selected,
    required this.hasChildren,
    required this.onTap,
    required this.onToggle,
  });

  final EditorChapter chapter;
  final bool selected;
  final bool hasChildren;
  final VoidCallback onTap;
  final VoidCallback onToggle;

  @override
  Widget build(BuildContext context) {
    final indent = ((chapter.level - 1).clamp(0, 3) * 14).toDouble();

    return Padding(
      padding: EdgeInsets.only(left: indent),
      child: ListTile(
        selected: selected,
        selectedTileColor: AppTheme.panelSoft,
        minLeadingWidth: 20,
        horizontalTitleGap: 4,
        contentPadding: const EdgeInsets.only(left: 4, right: 6),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(14)),
        leading: SizedBox(
          width: 24,
          child: hasChildren
              ? IconButton(
                  tooltip: chapter.collapsed ? '展开' : '折叠',
                  padding: EdgeInsets.zero,
                  constraints: const BoxConstraints.tightFor(
                    width: 24,
                    height: 24,
                  ),
                  onPressed: onToggle,
                  icon: Icon(
                    chapter.collapsed ? Icons.chevron_right : Icons.expand_more,
                    size: 20,
                  ),
                )
              : const SizedBox.shrink(),
        ),
        title: Text(
          chapter.title,
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
          style: TextStyle(
            fontWeight: chapter.level == 1 ? FontWeight.w800 : FontWeight.w600,
          ),
        ),
        dense: true,
        onTap: onTap,
      ),
    );
  }
}
