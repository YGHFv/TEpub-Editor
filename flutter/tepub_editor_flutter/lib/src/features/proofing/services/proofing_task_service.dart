import 'dart:convert';
import 'dart:io';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:path/path.dart' as p;

import '../../../core/storage/app_storage_paths.dart';
import '../models/proofing_item.dart';

final proofingTaskServiceProvider = FutureProvider<ProofingTaskService>((
  ref,
) async {
  final paths = await ref.watch(appStoragePathsProvider.future);
  return ProofingTaskService(paths);
});

class ProofingTaskService {
  const ProofingTaskService(this.paths);

  final AppStoragePaths paths;

  Future<List<ProofingSuggestion>> runDemoTask() async {
    await Future<void>.delayed(const Duration(milliseconds: 280));
    return demoSuggestions;
  }

  Future<ProofingSession> loadSession() async {
    final file = await _sessionFile();
    if (!await file.exists()) {
      return ProofingSession.empty();
    }
    try {
      final json = jsonDecode(await file.readAsString());
      if (json is Map<String, Object?>) {
        return ProofingSession.fromJson(json);
      }
    } catch (_) {
      // Corrupt proof logs should not block the UI.
    }
    return ProofingSession.empty();
  }

  Future<void> saveSession({
    required List<ProofingSuggestion> suggestions,
    required List<ProofingLogEntry> logs,
  }) async {
    final file = await _sessionFile();
    final session = ProofingSession(
      updatedAt: DateTime.now(),
      suggestions: suggestions,
      logs: logs,
    );
    await file.writeAsString(
      const JsonEncoder.withIndent('  ').convert(session.toJson()),
    );
  }

  Future<File> _sessionFile() async {
    await paths.ensureCreated();
    final dir = Directory(p.join(paths.logs.path, 'proofing'));
    if (!await dir.exists()) {
      await dir.create(recursive: true);
    }
    return File(p.join(dir.path, 'latest-session.json'));
  }
}

const demoSuggestions = [
  ProofingSuggestion(
    id: 'proof-001',
    type: ProofingItemType.typo,
    original: '在重构里保留原来的打包板本',
    replacement: '在重构里保留原来的打包版本',
    reason: '“板本”应为“版本”。高置信错别字可自动应用。',
    confidence: 0.98,
    decision: ProofingDecision.autoApplied,
  ),
  ProofingSuggestion(
    id: 'proof-002',
    type: ProofingItemType.wording,
    original: '目录和正文都会往上跳一段',
    replacement: '目录和正文不应在校对完成后改变滚动位置',
    reason: '表达更明确，但涉及语义改写，建议人工审核。',
    confidence: 0.74,
    decision: ProofingDecision.pending,
  ),
  ProofingSuggestion(
    id: 'proof-003',
    type: ProofingItemType.punctuation,
    original: '建议、审批、日志三个组件区',
    replacement: '建议、审批、日志三个组件区。',
    reason: '句末缺少标点。',
    confidence: 0.91,
    decision: ProofingDecision.pending,
  ),
  ProofingSuggestion(
    id: 'proof-004',
    type: ProofingItemType.style,
    original: '这个组件有点乱',
    replacement: '这个组件需要继续拆分职责',
    reason: '更贴近工程描述，保留人工判断。',
    confidence: 0.66,
    decision: ProofingDecision.rejected,
  ),
];
