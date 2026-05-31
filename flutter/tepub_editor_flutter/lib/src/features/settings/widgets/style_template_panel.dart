import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../ui/theme/app_theme.dart';
import '../models/style_template.dart';
import '../services/style_template_service.dart';

class StyleTemplatePanel extends ConsumerStatefulWidget {
  const StyleTemplatePanel({super.key});

  @override
  ConsumerState<StyleTemplatePanel> createState() => _StyleTemplatePanelState();
}

class _StyleTemplatePanelState extends ConsumerState<StyleTemplatePanel> {
  final _nameController = TextEditingController();
  final _cssController = TextEditingController();
  List<StyleTemplate> _templates = const [];
  String _message = '';
  bool _busy = false;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => _loadTemplates());
  }

  @override
  void dispose() {
    _nameController.dispose();
    _cssController.dispose();
    super.dispose();
  }

  Future<void> _loadTemplates() async {
    final service = await ref.read(styleTemplateServiceProvider.future);
    final templates = await service.listTemplates();
    if (mounted) {
      setState(() => _templates = templates);
    }
  }

  Future<void> _importTemplate() async {
    setState(() {
      _busy = true;
      _message = '正在导入 CSS 模板...';
    });
    try {
      final service = await ref.read(styleTemplateServiceProvider.future);
      final templates = await service.importTemplate();
      ref.invalidate(styleTemplatesProvider);
      if (mounted) {
        setState(() {
          _templates = templates;
          _message = '样式模板已更新。';
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

  Future<void> _saveTemplate() async {
    if (_cssController.text.trim().isEmpty) {
      setState(() => _message = '请先填写 CSS 内容。');
      return;
    }
    setState(() {
      _busy = true;
      _message = '正在保存模板...';
    });
    try {
      final service = await ref.read(styleTemplateServiceProvider.future);
      final templates = await service.saveTemplate(
        name: _nameController.text,
        css: _cssController.text,
      );
      ref.invalidate(styleTemplatesProvider);
      if (mounted) {
        setState(() {
          _templates = templates;
          _message = '模板已保存。';
        });
      }
    } catch (error) {
      if (mounted) {
        setState(() => _message = '保存失败：$error');
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  Future<void> _deleteTemplate(StyleTemplate template) async {
    setState(() {
      _busy = true;
      _message = '正在删除模板...';
    });
    try {
      final service = await ref.read(styleTemplateServiceProvider.future);
      final templates = await service.deleteTemplate(template);
      ref.invalidate(styleTemplatesProvider);
      if (mounted) {
        setState(() {
          _templates = templates;
          _message = template.builtin ? '内置模板不能删除。' : '模板已删除。';
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

  void _loadIntoEditor(StyleTemplate template) {
    setState(() {
      _nameController.text = template.builtin ? '' : template.name;
      _cssController.text = template.css;
      _message = '已载入 ${template.name}，可修改后另存。';
    });
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
                '样式模板',
                style: Theme.of(context).textTheme.titleMedium?.copyWith(
                      fontWeight: FontWeight.w800,
                    ),
              ),
            ),
            OutlinedButton.icon(
              onPressed: _busy ? null : _importTemplate,
              icon: const Icon(Icons.file_upload_outlined),
              label: const Text('导入 CSS'),
            ),
          ],
        ),
        const SizedBox(height: 8),
        Text(
          _message.isEmpty ? '管理 main.css 模板，制作 EPUB 时可直接套用。' : _message,
          style: const TextStyle(color: AppTheme.muted),
        ),
        if (_busy) ...[
          const SizedBox(height: 8),
          const LinearProgressIndicator(),
        ],
        const SizedBox(height: 10),
        SizedBox(
          height: 132,
          child: _templates.isEmpty
              ? const Center(child: Text('暂无样式模板。'))
              : ListView.separated(
                  itemCount: _templates.length,
                  separatorBuilder: (context, index) => const Divider(),
                  itemBuilder: (context, index) {
                    final template = _templates[index];
                    return ListTile(
                      dense: true,
                      contentPadding: EdgeInsets.zero,
                      title: Text(template.name),
                      subtitle: Text(
                        template.builtin
                            ? '内置默认'
                            : template.updatedAt.toLocal().toString(),
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      ),
                      onTap: () => _loadIntoEditor(template),
                      trailing: TextButton(
                        onPressed: _busy || template.builtin
                            ? null
                            : () => _deleteTemplate(template),
                        child: const Text('删除'),
                      ),
                    );
                  },
                ),
        ),
        const SizedBox(height: 10),
        TextField(
          controller: _nameController,
          decoration: const InputDecoration(
            labelText: '模板名',
            hintText: '例如：番茄正文样式',
          ),
        ),
        const SizedBox(height: 8),
        Expanded(
          child: TextField(
            controller: _cssController,
            expands: true,
            minLines: null,
            maxLines: null,
            textAlignVertical: TextAlignVertical.top,
            decoration: const InputDecoration(
              labelText: 'main.css',
              alignLabelWithHint: true,
            ),
          ),
        ),
        const SizedBox(height: 10),
        Align(
          alignment: Alignment.centerRight,
          child: FilledButton.icon(
            onPressed: _busy ? null : _saveTemplate,
            icon: const Icon(Icons.save_outlined),
            label: const Text('保存模板'),
          ),
        ),
      ],
    );
  }
}
