import 'package:flutter_test/flutter_test.dart';

import 'package:tepub_editor_flutter/src/app.dart';

void main() {
  testWidgets('TEpub Flutter shell renders library entry',
      (WidgetTester tester) async {
    await tester.pumpWidget(const TEpubFlutterApp());

    expect(find.text('书库'), findsWidgets);
  });
}
