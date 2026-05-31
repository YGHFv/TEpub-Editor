import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:tepub_editor_flutter/src/app.dart';

void main() {
  testWidgets('TEpub Flutter shell renders library entry',
      (WidgetTester tester) async {
    await tester.pumpWidget(const ProviderScope(child: TEpubFlutterApp()));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 500));
    await tester.pump();

    expect(find.text('书库'), findsWidgets);
    expect(find.text('添加图书'), findsOneWidget);
    expect(find.text('搜索书名、作者或格式'), findsOneWidget);
  });
}
