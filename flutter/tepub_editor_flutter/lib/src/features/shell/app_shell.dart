import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';

class AppShell extends StatelessWidget {
  const AppShell({required this.child, super.key});

  final Widget child;

  static const _items = [
    _NavItem('/library', Icons.local_library_outlined, '书库'),
    _NavItem('/editor', Icons.edit_note_outlined, '编辑'),
    _NavItem('/proofing', Icons.fact_check_outlined, '校对'),
    _NavItem('/settings', Icons.tune_outlined, '设置'),
  ];

  @override
  Widget build(BuildContext context) {
    final location = GoRouterState.of(context).uri.toString();

    return Scaffold(
      body: Row(
        children: [
          NavigationRail(
            selectedIndex:
                _items.indexWhere((item) => location.startsWith(item.path)),
            minWidth: 78,
            extended: MediaQuery.sizeOf(context).width >= 1180,
            onDestinationSelected: (index) => context.go(_items[index].path),
            labelType: NavigationRailLabelType.none,
            destinations: [
              for (final item in _items)
                NavigationRailDestination(
                  icon: Icon(item.icon),
                  selectedIcon: Icon(item.icon, fill: 1),
                  label: Text(item.label),
                ),
            ],
          ),
          const VerticalDivider(width: 1),
          Expanded(
            child: SafeArea(
              child: Padding(
                padding: const EdgeInsets.all(18),
                child: child,
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _NavItem {
  const _NavItem(this.path, this.icon, this.label);

  final String path;
  final IconData icon;
  final String label;
}
