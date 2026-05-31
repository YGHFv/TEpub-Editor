import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../ui/widgets/app_surface.dart';
import '../../../ui/widgets/empty_state.dart';
import '../models/proofing_item.dart';
import '../providers/proofing_controller.dart';
import 'proofing_stage.dart';

class ProofingBoard extends ConsumerWidget {
  const ProofingBoard({required this.stage, super.key});

  final ProofingStage stage;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final state = ref.watch(proofingControllerProvider);
    final controller = ref.read(proofingControllerProvider.notifier);

    if (!state.loaded) {
      return const AppSurface(
        child: Center(child: CircularProgressIndicator()),
      );
    }

    return AppSurface(
      child: AnimatedSwitcher(
        duration: const Duration(milliseconds: 180),
        child: switch (stage) {
          ProofingStage.suggestions => _SuggestionList(
              key: const ValueKey('suggestions'),
              items: state.pendingSuggestions,
              onApprove: controller.approve,
              onReject: controller.reject,
            ),
          ProofingStage.approval => _ApprovalList(
              key: const ValueKey('approval'),
              items: state.approvalResults,
              onRevert: controller.revert,
              onApplyAgain: controller.applyAgain,
            ),
          ProofingStage.logs => _LogList(
              key: const ValueKey('logs'),
              logs: state.logs,
            ),
        },
      ),
    );
  }
}

class _SuggestionList extends StatelessWidget {
  const _SuggestionList({
    required this.items,
    required this.onApprove,
    required this.onReject,
    super.key,
  });

  final List<ProofingSuggestion> items;
  final ValueChanged<String> onApprove;
  final ValueChanged<String> onReject;

  @override
  Widget build(BuildContext context) {
    if (items.isEmpty) {
      return const EmptyState(
        icon: Icons.tips_and_updates_outlined,
        title: '暂无待审核建议',
        message: '自动通过的错别字会进入审批结果，未自动通过的建议会留在这里人工处理。',
      );
    }

    return ListView.separated(
      itemCount: items.length,
      separatorBuilder: (context, index) => const Divider(height: 24),
      itemBuilder: (context, index) {
        final item = items[index];
        return _ProofingTile(
          item: item,
          trailing: Wrap(
            spacing: 8,
            children: [
              OutlinedButton(
                onPressed: () => onReject(item.id),
                child: const Text('忽略'),
              ),
              FilledButton(
                onPressed: () => onApprove(item.id),
                child: const Text('通过'),
              ),
            ],
          ),
        );
      },
    );
  }
}

class _ApprovalList extends StatelessWidget {
  const _ApprovalList({
    required this.items,
    required this.onRevert,
    required this.onApplyAgain,
    super.key,
  });

  final List<ProofingSuggestion> items;
  final ValueChanged<String> onRevert;
  final ValueChanged<String> onApplyAgain;

  @override
  Widget build(BuildContext context) {
    if (items.isEmpty) {
      return const EmptyState(
        icon: Icons.rule_folder_outlined,
        title: '暂无审批结果',
        message: '自动应用和人工审核后的结果会显示在这里。',
      );
    }

    return ListView.separated(
      itemCount: items.length,
      separatorBuilder: (context, index) => const Divider(height: 24),
      itemBuilder: (context, index) {
        final item = items[index];
        final reverted = item.decision == ProofingDecision.reverted;
        return _ProofingTile(
          item: item,
          trailing: FilledButton(
            onPressed: () =>
                reverted ? onApplyAgain(item.id) : onRevert(item.id),
            child: Text(reverted ? '应用' : '撤销'),
          ),
        );
      },
    );
  }
}

class _LogList extends StatelessWidget {
  const _LogList({required this.logs, super.key});

  final List<ProofingLogEntry> logs;

  @override
  Widget build(BuildContext context) {
    if (logs.isEmpty) {
      return const EmptyState(
        icon: Icons.receipt_long_outlined,
        title: '暂无日志',
        message: '校对请求、自动审批和人工操作会记录在这里。',
      );
    }

    return ListView.separated(
      itemCount: logs.length,
      separatorBuilder: (context, index) => const Divider(height: 20),
      itemBuilder: (context, index) {
        final log = logs[index];
        return ListTile(
          leading: const Icon(Icons.circle, size: 10),
          title: Text(log.message),
          subtitle: Text(_formatTime(log.time)),
          trailing: Text(log.level),
        );
      },
    );
  }
}

class _ProofingTile extends StatelessWidget {
  const _ProofingTile({required this.item, required this.trailing});

  final ProofingSuggestion item;
  final Widget trailing;

  @override
  Widget build(BuildContext context) {
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Wrap(
                spacing: 8,
                runSpacing: 8,
                children: [
                  Chip(label: Text(item.typeLabel)),
                  Chip(label: Text(item.decisionLabel)),
                  Chip(label: Text('置信度 ${(item.confidence * 100).round()}%')),
                ],
              ),
              const SizedBox(height: 10),
              Text('原文：${item.original}'),
              const SizedBox(height: 6),
              Text('修改后：${item.replacement}'),
              const SizedBox(height: 6),
              Text('原因：${item.reason}'),
            ],
          ),
        ),
        const SizedBox(width: 14),
        trailing,
      ],
    );
  }
}

String _formatTime(DateTime value) {
  return '${value.month.toString().padLeft(2, '0')}-'
      '${value.day.toString().padLeft(2, '0')} '
      '${value.hour.toString().padLeft(2, '0')}:'
      '${value.minute.toString().padLeft(2, '0')}';
}
