import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';

import '../../ui/theme/app_theme.dart';

class AppShell extends StatelessWidget {
  const AppShell({required this.navigationShell, super.key});

  final StatefulNavigationShell navigationShell;

  static const _items = [
    _NavItem('/library', Icons.local_library_outlined, '书库'),
    _NavItem('/editor', Icons.edit_note_outlined, '编辑'),
    _NavItem('/epub-editor', Icons.folder_zip_outlined, 'EPUB'),
    _NavItem('/proofing', Icons.fact_check_outlined, '校对'),
    _NavItem('/settings', Icons.tune_outlined, '设置'),
  ];

  @override
  Widget build(BuildContext context) {
    final location = GoRouterState.of(context).uri.toString();
    final selectedIndex = navigationShell.currentIndex;

    return Scaffold(
      backgroundColor: AppTheme.page,
      body: Row(
        children: [
          Container(
            width: 54,
            color: Colors.white,
            padding: const EdgeInsets.symmetric(vertical: 14),
            child: Column(
              children: [
                for (var index = 0; index < _items.length; index += 1)
                  Padding(
                    padding: const EdgeInsets.symmetric(vertical: 4),
                    child: _RailButton(
                      item: _items[index],
                      selected: index == selectedIndex,
                      onTap: index == selectedIndex ||
                              location.startsWith(_items[index].path)
                          ? null
                          : () => navigationShell.goBranch(
                                index,
                                initialLocation: index == selectedIndex,
                              ),
                    ),
                  ),
              ],
            ),
          ),
          const VerticalDivider(width: 1),
          Expanded(
            child: SafeArea(
              child: Padding(
                padding: const EdgeInsets.all(18),
                child: RepaintBoundary(child: navigationShell),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _RailButton extends StatelessWidget {
  const _RailButton({
    required this.item,
    required this.selected,
    required this.onTap,
  });

  final _NavItem item;
  final bool selected;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    return Tooltip(
      message: item.label,
      waitDuration: const Duration(milliseconds: 350),
      child: InkWell(
        borderRadius: BorderRadius.circular(14),
        onTap: onTap,
        child: AnimatedContainer(
          duration: const Duration(milliseconds: 160),
          curve: Curves.easeOutCubic,
          width: 40,
          height: 40,
          decoration: BoxDecoration(
            color: selected ? AppTheme.panelSoft : Colors.transparent,
            borderRadius: BorderRadius.circular(14),
            border: Border.all(
              color: selected ? AppTheme.border : Colors.transparent,
            ),
          ),
          child: Icon(
            item.icon,
            color: selected ? AppTheme.brand : AppTheme.ink,
            size: 22,
          ),
        ),
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
