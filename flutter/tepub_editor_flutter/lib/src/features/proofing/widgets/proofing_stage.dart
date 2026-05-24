enum ProofingStage {
  suggestions('suggestions', '建议'),
  approval('approval', '审批'),
  logs('logs', '日志');

  const ProofingStage(this.value, this.label);

  final String value;
  final String label;
}
