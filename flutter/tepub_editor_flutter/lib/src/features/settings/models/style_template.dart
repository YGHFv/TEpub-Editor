class StyleTemplate {
  const StyleTemplate({
    required this.id,
    required this.name,
    required this.path,
    required this.css,
    required this.updatedAt,
    this.builtin = false,
  });

  final String id;
  final String name;
  final String path;
  final String css;
  final DateTime updatedAt;
  final bool builtin;
}
