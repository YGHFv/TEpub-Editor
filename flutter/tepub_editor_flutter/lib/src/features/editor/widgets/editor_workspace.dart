import 'dart:async';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../ui/theme/app_theme.dart';
import '../../../ui/widgets/app_surface.dart';
import '../models/editor_document.dart';
import '../providers/editor_controller.dart';

class EditorWorkspace extends ConsumerStatefulWidget {
  const EditorWorkspace({super.key});

  @override
  ConsumerState<EditorWorkspace> createState() => _EditorWorkspaceState();
}

class _EditorWorkspaceState extends ConsumerState<EditorWorkspace> {
  static const _fontFamilyFallback = [
    'Microsoft YaHei UI',
    'Microsoft YaHei',
    'Noto Sans Mono CJK SC',
    'monospace',
  ];

  late final _EditorTextController _textController;
  late final TextEditingController _searchController;
  late final TextEditingController _replaceController;
  late final UndoHistoryController _undoController;
  late final FocusNode _editorFocusNode;
  late final ScrollController _textScrollController;
  late final ScrollController _lineScrollController;
  double _editorViewportWidth = 0;
  int _lastScrollReportMs = 0;
  String? _lineOffsetSource;
  List<int> _lineOffsets = const [0];
  String? _lineCountSource;
  int _cachedLineCount = 1;
  Timer? _lineCountDebounce;

  @override
  void initState() {
    super.initState();
    _textController = _EditorTextController();
    _searchController = TextEditingController();
    _replaceController = TextEditingController();
    _undoController = UndoHistoryController();
    _editorFocusNode = FocusNode();
    _textScrollController = ScrollController();
    _lineScrollController = ScrollController();
    final initialDocument = ref.read(editorControllerProvider);
    _textController.text = initialDocument.content;
    _cachedLineCount = _lineCount(initialDocument.content);
    _textController.addListener(_handleLocalTextChanged);
    _textScrollController.addListener(_syncLineNumberScroll);
  }

  @override
  void dispose() {
    _textScrollController.removeListener(_syncLineNumberScroll);
    _textController.removeListener(_handleLocalTextChanged);
    _lineCountDebounce?.cancel();
    _textController.dispose();
    _searchController.dispose();
    _replaceController.dispose();
    _undoController.dispose();
    _editorFocusNode.dispose();
    _textScrollController.dispose();
    _lineScrollController.dispose();
    super.dispose();
  }

  void _syncLineNumberScroll() {
    if (!_lineScrollController.hasClients ||
        !_textScrollController.hasClients) {
      return;
    }
    final max = _lineScrollController.position.maxScrollExtent;
    final target = _textScrollController.offset.clamp(0.0, max);
    if ((_lineScrollController.offset - target).abs() > 0.5) {
      _lineScrollController.jumpTo(target);
    }

    final now = DateTime.now().millisecondsSinceEpoch;
    if (now - _lastScrollReportMs > 220) {
      _lastScrollReportMs = now;
      final display = ref.read(editorControllerProvider).display;
      final lineHeight = display.fontSize * display.lineHeight;
      final approxLine = (_textScrollController.offset / lineHeight).floor();
      final offsets = _ensureLineOffsets(_textController.text);
      final offset = offsets[approxLine.clamp(0, offsets.length - 1)];
      ref
          .read(editorControllerProvider.notifier)
          .updateActiveChapterByOffset(offset);
    }
  }

  @override
  Widget build(BuildContext context) {
    ref.listen<String>(
      editorControllerProvider.select((document) => document.content),
      (previous, next) {
        if (_textController.text == next) {
          return;
        }
        final oldSelection = _textController.selection;
        _textController.text = next;
        if (oldSelection.isValid && oldSelection.end <= next.length) {
          _textController.selection = oldSelection;
        }
        _refreshLineCount(next);
      },
    );

    final document = ref.watch(
      editorControllerProvider.select(
        (document) => (
          search: document.search,
          display: document.display,
          pendingScrollOffset: document.pendingScrollOffset,
        ),
      ),
    );
    final controller = ref.read(editorControllerProvider.notifier);
    final editorStyle = TextStyle(
      fontSize: document.display.fontSize,
      height: document.display.lineHeight,
      color: AppTheme.ink,
      fontFamily: 'Consolas',
      fontFamilyFallback: _fontFamilyFallback,
    );
    _textController
      ..display = document.display
      ..baseStyle = editorStyle;
    final lineHeight = document.display.fontSize * document.display.lineHeight;

    if (_searchController.text != document.search.query) {
      _searchController.text = document.search.query;
      _searchController.selection = TextSelection.collapsed(
        offset: _searchController.text.length,
      );
    }

    if (_replaceController.text != document.search.replacement) {
      _replaceController.text = document.search.replacement;
      _replaceController.selection = TextSelection.collapsed(
        offset: _replaceController.text.length,
      );
    }

    final pendingOffset = document.pendingScrollOffset;
    if (pendingOffset != null) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (!mounted) {
          return;
        }
        final offset = pendingOffset.clamp(0, _textController.text.length);
        _scrollToTextOffset(offset);
        controller.clearPendingScrollOffset();
      });
    }

    return AppSurface(
      padding: const EdgeInsets.all(18),
      child: Column(
        children: [
          _EditorCommandBar(
            undoController: _undoController,
            onUndo: _undoController.undo,
            onRedo: _undoController.redo,
            onSelectAll: () => _selectAll(context),
            onSave: controller.save,
            onSearch: controller.showSearch,
            onCheck: controller.runChapterCheck,
          ),
          const SizedBox(height: 12),
          if (document.search.visible) ...[
            _SearchReplaceBar(
              controller: _searchController,
              replaceController: _replaceController,
              search: document.search,
              onChanged: controller.updateSearchQuery,
              onReplaceChanged: controller.updateReplacement,
              onModeChanged: controller.setSearchMode,
              onMatchCaseChanged: controller.setSearchMatchCase,
              onWholeWordChanged: controller.setSearchWholeWord,
              onScopeChanged: controller.setSearchScope,
              onPrevious: controller.previousSearchMatch,
              onNext: controller.nextSearchMatch,
              onReplaceCurrent: controller.replaceCurrent,
              onReplaceAll: controller.replaceAll,
              onClose: controller.toggleSearch,
            ),
            const SizedBox(height: 12),
          ],
          Expanded(
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                _LineNumberGutter(
                  lineCount: _cachedLineCount,
                  controller: _lineScrollController,
                  lineHeight: lineHeight,
                ),
                const VerticalDivider(width: 18),
                Expanded(
                  child: LayoutBuilder(
                    builder: (context, constraints) {
                      _editorViewportWidth = constraints.maxWidth;
                      return Shortcuts(
                        shortcuts: const {
                          SingleActivator(
                            LogicalKeyboardKey.keyA,
                            control: true,
                          ): _SafeSelectAllIntent(),
                        },
                        child: Actions(
                          actions: {
                            _SafeSelectAllIntent:
                                CallbackAction<_SafeSelectAllIntent>(
                              onInvoke: (_) {
                                _selectAll(context);
                                return null;
                              },
                            ),
                          },
                          child: RepaintBoundary(
                            child: ScrollConfiguration(
                              behavior: ScrollConfiguration.of(context)
                                  .copyWith(scrollbars: true),
                              child: EditableText(
                                controller: _textController,
                                focusNode: _editorFocusNode,
                                scrollController: _textScrollController,
                                undoController: _undoController,
                                onChanged: controller.updateContent,
                                autocorrect: false,
                                enableSuggestions: false,
                                smartDashesType: SmartDashesType.disabled,
                                smartQuotesType: SmartQuotesType.disabled,
                                maxLines: document.display.wordWrap ? null : 1,
                                minLines: null,
                                textAlign: TextAlign.start,
                                keyboardType: TextInputType.multiline,
                                cursorColor: AppTheme.brand,
                                backgroundCursorColor: AppTheme.muted,
                                selectionColor:
                                    AppTheme.brand.withValues(alpha: 0.22),
                                selectionWidthStyle: ui.BoxWidthStyle.tight,
                                spellCheckConfiguration:
                                    const SpellCheckConfiguration.disabled(),
                                magnifierConfiguration:
                                    TextMagnifierConfiguration.disabled,
                                stylusHandwritingEnabled: false,
                                clipBehavior: Clip.hardEdge,
                                style: editorStyle,
                                strutStyle: StrutStyle(
                                  fontSize: document.display.fontSize,
                                  height: document.display.lineHeight,
                                  forceStrutHeight: true,
                                ),
                                contextMenuBuilder:
                                    (context, editableTextState) {
                                  return AdaptiveTextSelectionToolbar
                                      .editableText(
                                    editableTextState: editableTextState,
                                  );
                                },
                              ),
                            ),
                          ),
                        ),
                      );
                    },
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  int _lineCount(String text) {
    if (_lineCountSource == text) {
      return _cachedLineCount;
    }
    if (text.isEmpty) {
      _lineCountSource = text;
      _cachedLineCount = 1;
      return 1;
    }
    _lineCountSource = text;
    _cachedLineCount = _ensureLineOffsets(text).length;
    return _cachedLineCount;
  }

  void _refreshLineCount(String text) {
    final previousCount = _cachedLineCount;
    final nextCount = _lineCount(text);
    if (mounted && previousCount != nextCount) {
      setState(() {});
    }
  }

  void _handleLocalTextChanged() {
    _lineCountDebounce?.cancel();
    _lineCountDebounce = Timer(const Duration(milliseconds: 120), () {
      if (mounted) {
        _refreshLineCount(_textController.text);
      }
    });
  }

  List<int> _ensureLineOffsets(String content) {
    if (_lineOffsetSource == content) {
      return _lineOffsets;
    }
    final offsets = <int>[0];
    var index = content.indexOf('\n');
    while (index >= 0) {
      offsets.add(index + 1);
      index = content.indexOf('\n', index + 1);
    }
    _lineOffsetSource = content;
    _lineOffsets = offsets;
    return offsets;
  }

  void _scrollToTextOffset(int offset) {
    if (!_textScrollController.hasClients) {
      return;
    }

    final safeOffset = offset.clamp(0, _textController.text.length);
    final display = ref.read(editorControllerProvider).display;
    final lineHeight = display.fontSize * display.lineHeight;
    final visualTop = _visualTopForOffset(safeOffset, display);
    final fallbackTop = _lineIndexForOffset(safeOffset) * lineHeight;
    final target = (visualTop ?? fallbackTop).clamp(
      0.0,
      _textScrollController.position.maxScrollExtent,
    );

    _textScrollController.jumpTo(target);
    _editorFocusNode.requestFocus();
  }

  double? _visualTopForOffset(int offset, EditorDisplaySettings display) {
    final width = _editorViewportWidth;
    if (width <= 0 || !_textController.text.contains('\n')) {
      return null;
    }

    final painter = TextPainter(
      text: TextSpan(
        text: _textController.text,
        style: TextStyle(
          fontSize: display.fontSize,
          height: display.lineHeight,
          color: AppTheme.ink,
          fontFamily: 'Consolas',
          fontFamilyFallback: _fontFamilyFallback,
        ),
      ),
      strutStyle: StrutStyle(
        fontSize: display.fontSize,
        height: display.lineHeight,
        forceStrutHeight: true,
      ),
      textDirection: TextDirection.ltr,
      maxLines: null,
    )..layout(maxWidth: width);

    return painter
        .getOffsetForCaret(TextPosition(offset: offset), Rect.zero)
        .dy;
  }

  int _lineIndexForOffset(int offset) {
    final offsets = _ensureLineOffsets(_textController.text);
    var low = 0;
    var high = offsets.length - 1;
    while (low <= high) {
      final mid = low + ((high - low) >> 1);
      if (offsets[mid] <= offset) {
        low = mid + 1;
      } else {
        high = mid - 1;
      }
    }
    return high.clamp(0, offsets.length - 1);
  }

  void _selectAll(BuildContext context) {
    const safeSelectionLimit = 200000;
    if (_textController.text.length > safeSelectionLimit) {
      Clipboard.setData(ClipboardData(text: _textController.text));
      _textController.selection = TextSelection.collapsed(
        offset: _textController.selection.extentOffset.clamp(
          0,
          _textController.text.length,
        ),
      );
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(
          content: Text('全文较长，已复制全部内容，避免全选渲染导致卡死。'),
          duration: Duration(seconds: 2),
        ),
      );
      return;
    }
    _textController.selection = TextSelection(
      baseOffset: 0,
      extentOffset: _textController.text.length,
    );
    _editorFocusNode.requestFocus();
  }
}

class _EditorTextController extends TextEditingController {
  EditorDisplaySettings display = EditorDisplaySettings.defaults;
  TextStyle baseStyle = const TextStyle();

  @override
  TextSpan buildTextSpan({
    required BuildContext context,
    TextStyle? style,
    required bool withComposing,
  }) {
    final effectiveStyle = (style ?? const TextStyle()).merge(baseStyle);
    if (!display.showWhitespace && !display.showLineBreaks) {
      return TextSpan(style: effectiveStyle, text: text);
    }

    final markerStyle = effectiveStyle.copyWith(
      color: AppTheme.muted.withValues(alpha: 0.72),
    );
    final spans = <TextSpan>[];
    final buffer = StringBuffer();
    void flush() {
      if (buffer.isEmpty) {
        return;
      }
      spans.add(TextSpan(text: buffer.toString()));
      buffer.clear();
    }

    for (final codeUnit in text.codeUnits) {
      final char = String.fromCharCode(codeUnit);
      if (display.showWhitespace && char == ' ') {
        flush();
        spans.add(TextSpan(text: '·', style: markerStyle));
      } else if (display.showWhitespace && char == '\t') {
        flush();
        spans.add(TextSpan(text: '→\t', style: markerStyle));
      } else if (display.showLineBreaks && char == '\n') {
        flush();
        spans.add(TextSpan(text: '↵', style: markerStyle));
        spans.add(const TextSpan(text: '\n'));
      } else {
        buffer.write(char);
      }
    }
    flush();
    return TextSpan(style: effectiveStyle, children: spans);
  }
}

class _EditorCommandBar extends StatelessWidget {
  const _EditorCommandBar({
    required this.undoController,
    required this.onUndo,
    required this.onRedo,
    required this.onSelectAll,
    required this.onSave,
    required this.onSearch,
    required this.onCheck,
  });

  final UndoHistoryController undoController;
  final VoidCallback onUndo;
  final VoidCallback onRedo;
  final VoidCallback onSelectAll;
  final VoidCallback onSave;
  final VoidCallback onSearch;
  final VoidCallback onCheck;

  @override
  Widget build(BuildContext context) {
    return Align(
      alignment: Alignment.centerLeft,
      child: ValueListenableBuilder<UndoHistoryValue>(
        valueListenable: undoController,
        builder: (context, value, child) {
          return Wrap(
            spacing: 8,
            runSpacing: 8,
            children: [
              _ToolButton(
                icon: Icons.undo,
                label: '撤销',
                onPressed: value.canUndo ? onUndo : null,
              ),
              _ToolButton(
                icon: Icons.redo,
                label: '重做',
                onPressed: value.canRedo ? onRedo : null,
              ),
              _ToolButton(
                icon: Icons.select_all,
                label: '全选',
                onPressed: onSelectAll,
              ),
              _ToolButton(
                icon: Icons.manage_search,
                label: '查找替换',
                onPressed: onSearch,
              ),
              _ToolButton(
                icon: Icons.rule_folder_outlined,
                label: '章节检查',
                onPressed: onCheck,
              ),
              _ToolButton(
                icon: Icons.save_outlined,
                label: '保存',
                onPressed: onSave,
              ),
            ],
          );
        },
      ),
    );
  }
}

class _ToolButton extends StatelessWidget {
  const _ToolButton({
    required this.icon,
    required this.label,
    required this.onPressed,
  });

  final IconData icon;
  final String label;
  final VoidCallback? onPressed;

  @override
  Widget build(BuildContext context) {
    return OutlinedButton.icon(
      onPressed: onPressed,
      icon: Icon(icon, size: 18),
      label: Text(label),
      style: OutlinedButton.styleFrom(
        visualDensity: VisualDensity.compact,
      ),
    );
  }
}

class _LineNumberGutter extends StatelessWidget {
  const _LineNumberGutter({
    required this.lineCount,
    required this.controller,
    required this.lineHeight,
  });

  final int lineCount;
  final ScrollController controller;
  final double lineHeight;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: 54,
      child: ScrollConfiguration(
        behavior: ScrollConfiguration.of(context).copyWith(scrollbars: false),
        child: IgnorePointer(
          child: ListView.builder(
            controller: controller,
            padding: EdgeInsets.zero,
            itemExtent: lineHeight,
            itemCount: lineCount,
            itemBuilder: (context, index) {
              return Align(
                alignment: Alignment.centerRight,
                child: Text(
                  '${index + 1}',
                  style: const TextStyle(
                    color: AppTheme.muted,
                    fontFeatures: [FontFeature.tabularFigures()],
                  ),
                ),
              );
            },
          ),
        ),
      ),
    );
  }
}

class _SearchReplaceBar extends StatelessWidget {
  const _SearchReplaceBar({
    required this.controller,
    required this.replaceController,
    required this.search,
    required this.onChanged,
    required this.onReplaceChanged,
    required this.onModeChanged,
    required this.onMatchCaseChanged,
    required this.onWholeWordChanged,
    required this.onScopeChanged,
    required this.onPrevious,
    required this.onNext,
    required this.onReplaceCurrent,
    required this.onReplaceAll,
    required this.onClose,
  });

  final TextEditingController controller;
  final TextEditingController replaceController;
  final EditorSearchState search;
  final ValueChanged<String> onChanged;
  final ValueChanged<String> onReplaceChanged;
  final ValueChanged<EditorSearchMode> onModeChanged;
  final ValueChanged<bool> onMatchCaseChanged;
  final ValueChanged<bool> onWholeWordChanged;
  final ValueChanged<EditorSearchScope> onScopeChanged;
  final VoidCallback onPrevious;
  final VoidCallback onNext;
  final VoidCallback onReplaceCurrent;
  final VoidCallback onReplaceAll;
  final VoidCallback onClose;

  @override
  Widget build(BuildContext context) {
    final matchText = search.error != null
        ? '正则错误'
        : search.matches.isEmpty
            ? '0 / 0'
            : '${search.currentIndex + 1} / ${search.matches.length}';
    final canReplace = search.scope == EditorSearchScope.content;

    return Shortcuts(
      shortcuts: const {
        SingleActivator(LogicalKeyboardKey.escape): _CloseSearchIntent(),
      },
      child: Actions(
        actions: {
          _CloseSearchIntent: CallbackAction<_CloseSearchIntent>(
            onInvoke: (_) {
              onClose();
              return null;
            },
          ),
        },
        child: DecoratedBox(
          decoration: BoxDecoration(
            color: AppTheme.panelSoft.withValues(alpha: 0.62),
            border: Border.all(color: AppTheme.border),
            borderRadius: BorderRadius.circular(18),
          ),
          child: Padding(
            padding: const EdgeInsets.all(10),
            child: Column(
              children: [
                Row(
                  children: [
                    Expanded(
                      child: TextField(
                        controller: controller,
                        autofocus: true,
                        onChanged: onChanged,
                        decoration: InputDecoration(
                          prefixIcon: const Icon(Icons.search),
                          hintText: search.scope == EditorSearchScope.toc
                              ? '在目录中查找'
                              : '搜索正文',
                          errorText: search.error,
                        ),
                      ),
                    ),
                    const SizedBox(width: 10),
                    SegmentedButton<EditorSearchScope>(
                      segments: const [
                        ButtonSegment(
                          value: EditorSearchScope.content,
                          label: Text('正文'),
                        ),
                        ButtonSegment(
                          value: EditorSearchScope.toc,
                          label: Text('目录'),
                        ),
                      ],
                      selected: {search.scope},
                      onSelectionChanged: (selection) {
                        onScopeChanged(selection.first);
                      },
                    ),
                    const SizedBox(width: 10),
                    SegmentedButton<EditorSearchMode>(
                      segments: const [
                        ButtonSegment(
                          value: EditorSearchMode.normal,
                          label: Text('普通'),
                        ),
                        ButtonSegment(
                          value: EditorSearchMode.regex,
                          label: Text('正则'),
                        ),
                        ButtonSegment(
                          value: EditorSearchMode.extended,
                          label: Text('扩展'),
                        ),
                      ],
                      selected: {search.mode},
                      onSelectionChanged: (selection) {
                        onModeChanged(selection.first);
                      },
                    ),
                    const SizedBox(width: 10),
                    Text(matchText),
                    IconButton(
                      tooltip: '上一个',
                      onPressed: onPrevious,
                      icon: const Icon(Icons.keyboard_arrow_up),
                    ),
                    IconButton(
                      tooltip: '下一个',
                      onPressed: onNext,
                      icon: const Icon(Icons.keyboard_arrow_down),
                    ),
                    IconButton(
                      tooltip: '关闭查找替换',
                      onPressed: onClose,
                      icon: const Icon(Icons.close),
                    ),
                  ],
                ),
                const SizedBox(height: 10),
                Row(
                  children: [
                    Expanded(
                      child: TextField(
                        controller: replaceController,
                        enabled: canReplace,
                        onChanged: onReplaceChanged,
                        decoration: InputDecoration(
                          prefixIcon: const Icon(Icons.find_replace),
                          hintText: canReplace ? '替换为' : '目录查找不支持替换',
                        ),
                      ),
                    ),
                    const SizedBox(width: 10),
                    OutlinedButton(
                      onPressed: canReplace ? onReplaceCurrent : null,
                      child: const Text('替换当前'),
                    ),
                    const SizedBox(width: 8),
                    FilledButton(
                      onPressed: canReplace ? onReplaceAll : null,
                      child: const Text('全部替换'),
                    ),
                    const SizedBox(width: 10),
                    FilterChip(
                      label: const Text('区分大小写'),
                      selected: search.matchCase,
                      onSelected: onMatchCaseChanged,
                    ),
                    const SizedBox(width: 8),
                    FilterChip(
                      label: const Text('全词匹配'),
                      selected: search.wholeWord,
                      onSelected: onWholeWordChanged,
                    ),
                  ],
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _CloseSearchIntent extends Intent {
  const _CloseSearchIntent();
}

class _SafeSelectAllIntent extends Intent {
  const _SafeSelectAllIntent();
}
