enum ProofingItemType { typo, wording, punctuation, style }

enum ProofingDecision { pending, autoApplied, approved, rejected, reverted }

class ProofingSuggestion {
  const ProofingSuggestion({
    required this.id,
    required this.type,
    required this.original,
    required this.replacement,
    required this.reason,
    required this.confidence,
    required this.decision,
  });

  final String id;
  final ProofingItemType type;
  final String original;
  final String replacement;
  final String reason;
  final double confidence;
  final ProofingDecision decision;

  String get typeLabel => switch (type) {
        ProofingItemType.typo => '错别字',
        ProofingItemType.wording => '措辞',
        ProofingItemType.punctuation => '标点',
        ProofingItemType.style => '风格',
      };

  String get decisionLabel => switch (decision) {
        ProofingDecision.pending => '待审核',
        ProofingDecision.autoApplied => '自动通过',
        ProofingDecision.approved => '已通过',
        ProofingDecision.rejected => '已忽略',
        ProofingDecision.reverted => '已撤销',
      };

  ProofingSuggestion copyWith({ProofingDecision? decision}) {
    return ProofingSuggestion(
      id: id,
      type: type,
      original: original,
      replacement: replacement,
      reason: reason,
      confidence: confidence,
      decision: decision ?? this.decision,
    );
  }

  factory ProofingSuggestion.fromJson(Map<String, Object?> json) {
    return ProofingSuggestion(
      id: json['id'] as String? ?? '',
      type: ProofingItemType.values.byName(
        json['type'] as String? ?? ProofingItemType.wording.name,
      ),
      original: json['original'] as String? ?? '',
      replacement: json['replacement'] as String? ?? '',
      reason: json['reason'] as String? ?? '',
      confidence: (json['confidence'] as num?)?.toDouble() ?? 0,
      decision: ProofingDecision.values.byName(
        json['decision'] as String? ?? ProofingDecision.pending.name,
      ),
    );
  }

  Map<String, Object?> toJson() {
    return {
      'id': id,
      'type': type.name,
      'original': original,
      'replacement': replacement,
      'reason': reason,
      'confidence': confidence,
      'decision': decision.name,
    };
  }
}

class ProofingLogEntry {
  const ProofingLogEntry({
    required this.time,
    required this.message,
    required this.level,
  });

  final DateTime time;
  final String message;
  final String level;

  factory ProofingLogEntry.fromJson(Map<String, Object?> json) {
    return ProofingLogEntry(
      time: DateTime.tryParse(json['time'] as String? ?? '') ??
          DateTime.fromMillisecondsSinceEpoch(0),
      message: json['message'] as String? ?? '',
      level: json['level'] as String? ?? 'info',
    );
  }

  Map<String, Object?> toJson() {
    return {
      'time': time.toIso8601String(),
      'message': message,
      'level': level,
    };
  }
}

class ProofingSession {
  const ProofingSession({
    required this.updatedAt,
    required this.suggestions,
    required this.logs,
  });

  final DateTime updatedAt;
  final List<ProofingSuggestion> suggestions;
  final List<ProofingLogEntry> logs;

  factory ProofingSession.empty() {
    return ProofingSession(
      updatedAt: DateTime.now(),
      suggestions: const [],
      logs: const [],
    );
  }

  factory ProofingSession.fromJson(Map<String, Object?> json) {
    final suggestions = json['suggestions'] as List<Object?>? ?? const [];
    final logs = json['logs'] as List<Object?>? ?? const [];
    return ProofingSession(
      updatedAt: DateTime.tryParse(json['updatedAt'] as String? ?? '') ??
          DateTime.fromMillisecondsSinceEpoch(0),
      suggestions: [
        for (final item in suggestions)
          if (item is Map<String, Object?>) ProofingSuggestion.fromJson(item),
      ],
      logs: [
        for (final item in logs)
          if (item is Map<String, Object?>) ProofingLogEntry.fromJson(item),
      ],
    );
  }

  Map<String, Object?> toJson() {
    return {
      'updatedAt': updatedAt.toIso8601String(),
      'suggestions': suggestions.map((item) => item.toJson()).toList(),
      'logs': logs.map((item) => item.toJson()).toList(),
    };
  }
}
