import 'package:flutter/material.dart';

import 'widgets/proofing_board.dart';
import 'widgets/proofing_stage.dart';
import 'widgets/proofing_stage_switcher.dart';
import '../../ui/widgets/app_page.dart';

class ProofingPage extends StatefulWidget {
  const ProofingPage({super.key});

  @override
  State<ProofingPage> createState() => _ProofingPageState();
}

class _ProofingPageState extends State<ProofingPage> {
  ProofingStage _stage = ProofingStage.suggestions;

  @override
  Widget build(BuildContext context) {
    return AppPage(
      title: '智能校对',
      subtitle: '校对流程会拆成建议、审批、日志三个组件区，错别字自动应用和人工审核分流后续迁移。',
      actions: [
        FilledButton.icon(
          onPressed: () {},
          icon: const Icon(Icons.play_arrow_rounded),
          label: const Text('开始校对'),
        ),
        OutlinedButton.icon(
          onPressed: null,
          icon: const Icon(Icons.pause_outlined),
          label: const Text('暂停'),
        ),
        ProofingStageSwitcher(
          stage: _stage,
          onChanged: (stage) => setState(() => _stage = stage),
        ),
      ],
      child: ProofingBoard(stage: _stage),
    );
  }
}
