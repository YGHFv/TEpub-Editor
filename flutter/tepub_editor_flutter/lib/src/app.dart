import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';

import 'core/app_constants.dart';
import 'features/editor/editor_page.dart';
import 'features/epub_editor/epub_editor_page.dart';
import 'features/library/library_page.dart';
import 'features/library/models/library_book.dart';
import 'features/proofing/proofing_page.dart';
import 'features/settings/settings_page.dart';
import 'features/shell/app_shell.dart';
import 'ui/theme/app_theme.dart';

final _router = GoRouter(
  initialLocation: '/library',
  routes: [
    StatefulShellRoute.indexedStack(
      builder: (context, state, navigationShell) =>
          AppShell(navigationShell: navigationShell),
      branches: [
        StatefulShellBranch(
          routes: [
            GoRoute(
              path: '/library',
              pageBuilder: (context, state) => const NoTransitionPage(
                child: LibraryPage(),
              ),
            ),
          ],
        ),
        StatefulShellBranch(
          routes: [
            GoRoute(
              path: '/editor',
              pageBuilder: (context, state) => NoTransitionPage(
                child: EditorPage(
                  sourceBook: state.extra is LibraryBook
                      ? state.extra as LibraryBook
                      : null,
                ),
              ),
            ),
          ],
        ),
        StatefulShellBranch(
          routes: [
            GoRoute(
              path: '/epub-editor',
              pageBuilder: (context, state) => const NoTransitionPage(
                child: EpubEditorPage(),
              ),
            ),
          ],
        ),
        StatefulShellBranch(
          routes: [
            GoRoute(
              path: '/proofing',
              pageBuilder: (context, state) => NoTransitionPage(
                child: ProofingPage(
                  sourceBook: state.extra is LibraryBook
                      ? state.extra as LibraryBook
                      : null,
                ),
              ),
            ),
          ],
        ),
        StatefulShellBranch(
          routes: [
            GoRoute(
              path: '/settings',
              pageBuilder: (context, state) => const NoTransitionPage(
                child: SettingsPage(),
              ),
            ),
          ],
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
