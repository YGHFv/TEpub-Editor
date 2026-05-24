import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';

import 'core/app_constants.dart';
import 'features/editor/editor_page.dart';
import 'features/library/library_page.dart';
import 'features/proofing/proofing_page.dart';
import 'features/settings/settings_page.dart';
import 'features/shell/app_shell.dart';
import 'ui/theme/app_theme.dart';

final _router = GoRouter(
  initialLocation: '/library',
  routes: [
    ShellRoute(
      builder: (context, state, child) => AppShell(child: child),
      routes: [
        GoRoute(
          path: '/library',
          builder: (context, state) => const LibraryPage(),
        ),
        GoRoute(
          path: '/editor',
          builder: (context, state) => const EditorPage(),
        ),
        GoRoute(
          path: '/proofing',
          builder: (context, state) => const ProofingPage(),
        ),
        GoRoute(
          path: '/settings',
          builder: (context, state) => const SettingsPage(),
        ),
      ],
    ),
  ],
);

class TEpubFlutterApp extends StatelessWidget {
  const TEpubFlutterApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp.router(
      title: AppConstants.appName,
      debugShowCheckedModeBanner: false,
      theme: AppTheme.light(),
      routerConfig: _router,
    );
  }
}
