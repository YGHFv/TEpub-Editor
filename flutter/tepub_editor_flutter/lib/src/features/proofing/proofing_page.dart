import 'package:flutter/material.dart';

class ProofingPage extends StatelessWidget {
  const ProofingPage({super.key});

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text('智能校对', style: Theme.of(context).textTheme.headlineMedium),
        const SizedBox(height: 12),
        Wrap(
          spacing: 10,
          children: [
            FilledButton(onPressed: () {}, child: const Text('开始')),
            OutlinedButton(onPressed: null, child: const Text('停止')),
            SegmentedButton<String>(
              segments: const [
                ButtonSegment(value: 'suggestions', label: Text('建议')),
                ButtonSegment(value: 'approval', label: Text('审批')),
                ButtonSegment(value: 'logs', label: Text('日志')),
              ],
              selected: const {'suggestions'},
              onSelectionChanged: (_) {},
            ),
          ],
        ),
        const SizedBox(height: 16),
        const Expanded(
          child: Card(
            elevation: 0,
            child: Center(child: Text('AI 校对、审批、日志模块待迁移')),
          ),
        ),
      ],
    );
  }
}
