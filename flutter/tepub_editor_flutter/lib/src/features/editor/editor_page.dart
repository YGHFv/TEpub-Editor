import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../ui/widgets/app_page.dart';
import '../../ui/widgets/responsive_two_pane.dart';
import '../library/models/library_book.dart';
import 'providers/editor_controller.dart';
import 'widgets/editor_make_epub_dialog.dart';
import 'widgets/editor_sidebar.dart';
import 'widgets/editor_workspace.dart';

class EditorPage extends ConsumerStatefulWidget {
  const EditorPage({this.sourceBook, super.key});

  final LibraryBook? sourceBook;

  @override
  ConsumerState<EditorPage> createState() => _EditorPageState();
}

class _EditorPageState extends ConsumerState<EditorPage> {
  String? _loadedBookId;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => _loadSourceBook());
  }

  @override
  void didUpdateWidget(covariant EditorPage oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.sourceBook?.id != widget.sourceBook?.id) {
      WidgetsBinding.instance.addPostFrameCallback((_) => _loadSourceBook());
    }
  }

  Future<void> _loadSourceBook() async {
    final book = widget.sourceBook;
    if (!mounted || book == null || _loadedBookId == book.id) {
      return;
    }
    _loadedBookId = book.id;
    await ref.read(editorControllerProvider.notifier).openLibraryBook(book);
  }

  @override
  Widget build(BuildContext context) {
    final header = ref.watch(
      editorControllerProvider.select(
        (document) => (
          title: document.title,
          dirty: document.dirty,
          filePath: document.filePath,
        ),
      ),
    );
    final controller = ref.read(editorControllerProvider.notifier);

    return AppPage(
      title: 'TXT 编辑器',
      subtitle: header.dirty
          ? '${header.title} 有未保存修改'
          : header.filePath ?? '打开 TXT 文件后可编辑、查找替换、保存，并通过当前目录制作 EPUB。',
      actions: [
        OutlinedButton.icon(
          onPressed: controller.openTextFile,
          icon: const Icon(Icons.folder_open_outlined),
          label: const Text('打开 TXT'),
        ),
        OutlinedButton.icon(
          onPressed: controller.showSearch,
          icon: const Icon(Icons.manage_search),
          label: const Text('查找替换'),
        ),
        OutlinedButton.icon(
          onPressed: () => _showMakeEpub(context),
          icon: const Icon(Icons.auto_awesome_motion_outlined),
          label: const Text('制作 EPUB'),
        ),
        OutlinedButton.icon(
          onPressed: () => _showEditorSettings(context),
          icon: const Icon(Icons.tune_outlined),
          label: const Text('设置'),
        ),
        FilledButton.icon(
          onPressed: header.dirty ? controller.save : null,
          icon: const Icon(Icons.save_outlined),
          label: const Text('保存'),
        ),
      ],
      child: const ResponsiveTwoPane(
        sideWidth: 260,
        side: EditorSidebar(),
        body: EditorWorkspace(),
      ),
    );
  }

  Future<void> _showEditorSettings(BuildContext context) async {
    await showDialog<void>(
      context: context,
      builder: (context) => const Dialog(
        backgroundColor: Colors.transparent,
        insetPadding: EdgeInsets.all(24),
        child: EditorSettingsPanel(),
      ),
    );
  }

  Future<void> _showMakeEpub(BuildContext context) async {
    final document = ref.read(editorControllerProvider.notifier).rebuildToc();
    await showDialog<void>(
      context: context,
      builder: (context) => Dialog(
        backgroundColor: Colors.transparent,
        insetPadding: const EdgeInsets.all(24),
        child: EditorMakeEpubDialog(document: document),
      ),
    );
  }
}
