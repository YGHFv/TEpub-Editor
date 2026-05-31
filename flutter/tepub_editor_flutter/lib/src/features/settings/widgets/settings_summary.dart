import 'package:flutter/material.dart';

import '../../../core/settings/app_settings.dart';

class SettingsSummary extends StatelessWidget {
  const SettingsSummary({required this.settings, super.key});

  final AppSettings settings;

  @override
  Widget build(BuildContext context) {
    return Wrap(
      spacing: 14,
      runSpacing: 14,
      children: [
        _SettingChip(
          label: '阅读字号',
          value: settings.readerFontSize.toStringAsFixed(0),
        ),
        _SettingChip(
          label: '行高',
          value: settings.readerLineHeight.toStringAsFixed(1),
        ),
        _SettingChip(
          label: '错别字自动应用',
          value: settings.autoApplyTypos ? '开启' : '关闭',
        ),
        _SettingChip(label: 'AI 服务', value: settings.aiProviderName),
      ],
    );
  }
}

class _SettingChip extends StatelessWidget {
  const _SettingChip({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    return Chip(
      label: Text('$label：$value'),
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
    );
  }
}
