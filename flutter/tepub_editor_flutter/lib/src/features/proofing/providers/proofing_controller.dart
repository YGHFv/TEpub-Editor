import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/proofing_item.dart';
import '../services/proofing_task_service.dart';

final proofingControllerProvider =
    StateNotifierProvider<ProofingController, ProofingState>((ref) {
  return ProofingController(ref.read(proofingTaskServiceProvider.future))
    ..loadSession();
});

class ProofingState {
  const ProofingState({
    required this.running,
    required this.suggestions,
    required this.logs,
    required this.loaded,
  });

  final bool running;
  final List<ProofingSuggestion> suggestions;
  final List<ProofingLogEntry> logs;
  final bool loaded;

  List<ProofingSuggestion> get pendingSuggestions {
    return suggestions
        .where((item) => item.decision == ProofingDecision.pending)
        .toList();
  }

  List<ProofingSuggestion> get approvalResults {
    return suggestions
        .where((item) => item.decision != ProofingDecision.pending)
        .toList();
  }

  ProofingState copyWith({
    bool? running,
    List<ProofingSuggestion>? suggestions,
    List<ProofingLogEntry>? logs,
    bool? loaded,
  }) {
    return ProofingState(
      running: running ?? this.running,
      suggestions: suggestions ?? this.suggestions,
      logs: logs ?? this.logs,
      loaded: loaded ?? this.loaded,
    );
  }
}

class ProofingController extends StateNotifier<ProofingState> {
  ProofingController(this._taskServiceFuture)
      : super(
          ProofingState(
            running: false,
            suggestions: demoSuggestions,
            logs: _seedLogs,
            loaded: false,
          ),
        );

  final Future<ProofingTaskService> _taskServiceFuture;

  Future<void> loadSession() async {
    try {
      final service = await _taskServiceFuture;
      final session = await service.loadSession();
      if (!mounted) {
        return;
      }
      state = state.copyWith(
        suggestions:
            session.suggestions.isEmpty ? demoSuggestions : session.suggestions,
        logs: session.logs.isEmpty ? _seedLogs : session.logs,
        loaded: true,
      );
    } catch (error) {
      if (!mounted) {
        return;
      }
      state = state.copyWith(
        loaded: true,
        logs: [_log('读取校对日志失败：$error', 'error'), ...state.logs],
      );
    }
  }

  Future<void> start() async {
    state = state.copyWith(
      running: true,
      logs: [_log('开始校对任务', 'info'), ...state.logs],
    );
    await _persist();

    final taskService = await _taskServiceFuture;
    final suggestions = await taskService.runDemoTask();
    state = state.copyWith(
      running: false,
      suggestions: suggestions,
      logs: [_log('生成 ${suggestions.length} 条校对建议', 'success'), ...state.logs],
    );
    await _persist();
  }

  Future<void> pause() async {
    state = state.copyWith(
      running: false,
      logs: [_log('暂停校对任务', 'info'), ...state.logs],
    );
    await _persist();
  }

  Future<void> approve(String id) async {
    await _setDecision(id, ProofingDecision.approved, '人工通过建议');
  }

  Future<void> reject(String id) async {
    await _setDecision(id, ProofingDecision.rejected, '人工忽略建议');
  }

  Future<void> revert(String id) async {
    await _setDecision(id, ProofingDecision.reverted, '撤销已应用结果');
  }

  Future<void> applyAgain(String id) async {
    await _setDecision(id, ProofingDecision.approved, '重新应用结果');
  }

  Future<void> _setDecision(
    String id,
    ProofingDecision decision,
    String logMessage,
  ) async {
    state = state.copyWith(
      suggestions: [
        for (final item in state.suggestions)
          if (item.id == id) item.copyWith(decision: decision) else item,
      ],
      logs: [_log(logMessage, 'info'), ...state.logs],
    );
    await _persist();
  }

  ProofingLogEntry _log(String message, String level) {
    return ProofingLogEntry(
      time: DateTime.now(),
      message: message,
      level: level,
    );
  }

  Future<void> _persist() async {
    try {
      final service = await _taskServiceFuture;
      await service.saveSession(
        suggestions: state.suggestions,
        logs: state.logs,
      );
    } catch (error) {
      if (!mounted) {
        return;
      }
      state = state.copyWith(
        logs: [_log('保存校对日志失败：$error', 'error'), ...state.logs],
      );
    }
  }
}

final _seedLogs = [
  ProofingLogEntry(
    time: DateTime(2026, 5, 24, 15, 32),
    message: '错别字 proof-001 自动通过并应用',
    level: 'success',
  ),
  ProofingLogEntry(
    time: DateTime(2026, 5, 24, 15, 31),
    message: '生成 4 条校对建议',
    level: 'info',
  ),
];
