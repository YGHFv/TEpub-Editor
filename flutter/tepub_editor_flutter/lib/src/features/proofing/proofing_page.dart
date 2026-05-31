import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../ui/widgets/app_page.dart';
import '../library/models/library_book.dart';
import 'providers/proofing_controller.dart';
import 'widgets/proofing_board.dart';
import 'widgets/proofing_stage.dart';
import 'widgets/proofing_stage_switcher.dart';

class ProofingPage extends ConsumerStatefulWidget {
  const ProofingPage({this.sourceBook, super.key});

  final LibraryBook? sourceBook;

  @override
  ConsumerState<ProofingPage> createState() => _ProofingPageState();
}

class _ProofingPageState extends ConsumerState<ProofingPage> {
  ProofingStage _stage = ProofingStage.suggestions;

  @override
  Widget build(BuildContext context) {
    final proofing = ref.watch(proofingControllerProvider);
    final controller = ref.read(proofingControllerProvider.notifier);
    final bookTitle = widget.sourceBook?.title;

    return AppPage(
      title: '智能校对',
      subtitle: bookTitle == null
          ? '错别字自动应用，没有自动通过的建议进入人工审核；审批结果和日志共享同一份任务状态。'
          : '正在准备校对《$bookTitle》，后续会接入书库文件内容和真实 AI 请求。',
      actions: [
        FilledButton.icon(
          onPressed:
              proofing.running || !proofing.loaded ? null : controller.start,
          icon: const Icon(Icons.play_arrow_rounded),
          label: const Text('开始校对'),
        ),
        OutlinedButton.icon(
          onPressed: proofing.running ? controller.pause : null,
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
