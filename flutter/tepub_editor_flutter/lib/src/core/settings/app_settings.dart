class AppSettings {
  const AppSettings({
    required this.readerFontSize,
    required this.readerLineHeight,
    required this.autoApplyTypos,
    required this.aiProviderName,
  });

  final double readerFontSize;
  final double readerLineHeight;
  final bool autoApplyTypos;
  final String aiProviderName;

  static const defaults = AppSettings(
    readerFontSize: 18,
    readerLineHeight: 1.7,
    autoApplyTypos: true,
    aiProviderName: '未配置',
  );

  factory AppSettings.fromJson(Map<String, Object?> json) {
    return AppSettings(
      readerFontSize: (json['readerFontSize'] as num?)?.toDouble() ??
          defaults.readerFontSize,
      readerLineHeight: (json['readerLineHeight'] as num?)?.toDouble() ??
          defaults.readerLineHeight,
      autoApplyTypos:
          json['autoApplyTypos'] as bool? ?? defaults.autoApplyTypos,
      aiProviderName:
          json['aiProviderName'] as String? ?? defaults.aiProviderName,
    );
  }

  Map<String, Object?> toJson() {
    return {
      'readerFontSize': readerFontSize,
      'readerLineHeight': readerLineHeight,
      'autoApplyTypos': autoApplyTypos,
      'aiProviderName': aiProviderName,
    };
  }

  AppSettings copyWith({
    double? readerFontSize,
    double? readerLineHeight,
    bool? autoApplyTypos,
    String? aiProviderName,
  }) {
    return AppSettings(
      readerFontSize: readerFontSize ?? this.readerFontSize,
      readerLineHeight: readerLineHeight ?? this.readerLineHeight,
      autoApplyTypos: autoApplyTypos ?? this.autoApplyTypos,
      aiProviderName: aiProviderName ?? this.aiProviderName,
    );
  }
}
