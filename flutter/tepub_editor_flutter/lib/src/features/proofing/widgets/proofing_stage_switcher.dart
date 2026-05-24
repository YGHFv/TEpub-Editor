import 'package:flutter/material.dart';

import 'proofing_stage.dart';

class ProofingStageSwitcher extends StatelessWidget {
  const ProofingStageSwitcher({
    required this.stage,
    required this.onChanged,
    super.key,
  });

  final ProofingStage stage;
  final ValueChanged<ProofingStage> onChanged;

  @override
  Widget build(BuildContext context) {
    return SegmentedButton<ProofingStage>(
      segments: [
        for (final item in ProofingStage.values)
          ButtonSegment(value: item, label: Text(item.label)),
      ],
      selected: {stage},
      onSelectionChanged: (selection) => onChanged(selection.first),
    );
  }
}
