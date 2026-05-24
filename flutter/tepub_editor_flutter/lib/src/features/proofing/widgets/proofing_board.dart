import 'package:flutter/material.dart';

import '../../../ui/widgets/app_surface.dart';
import '../../../ui/widgets/empty_state.dart';
import 'proofing_stage.dart';

class ProofingBoard extends StatelessWidget {
  const ProofingBoard({required this.stage, super.key});

  final ProofingStage stage;

  @override
  Widget build(BuildContext context) {
    return AppSurface(
      child: AnimatedSwitcher(
        duration: const Duration(milliseconds: 180),
        child: _StageEmptyState(key: ValueKey(stage), stage: stage),
      ),
    );
  }
}

class _StageEmptyState extends StatelessWidget {
  const _StageEmptyState({required this.stage, super.key});

  final ProofingStage stage;

  @override
  Widget build(BuildContext context) {
    return switch (stage) {
      ProofingStage.suggestions => const EmptyState(
          icon: Icons.tips_and_updates_outlined,
          title: '建议列表准备迁移',
          message: '这里会承载原文、修改后、原因、置信度和人工处理按钮。',
        ),
      ProofingStage.approval => const EmptyState(
          icon: Icons.rule_folder_outlined,
          title: '审批结果准备迁移',
          message: '这里会承载自动通过、待审核、已撤销与可重新应用的结果列表。',
        ),
      ProofingStage.logs => const EmptyState(
          icon: Icons.receipt_long_outlined,
          title: '校对日志准备迁移',
          message: '这里会承载 AI 请求、自动审批、人工操作和异常记录。',
        ),
    };
  }
}
