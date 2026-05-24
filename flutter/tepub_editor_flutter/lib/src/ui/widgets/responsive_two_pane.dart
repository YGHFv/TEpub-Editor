import 'package:flutter/material.dart';

class ResponsiveTwoPane extends StatelessWidget {
  const ResponsiveTwoPane({
    required this.side,
    required this.body,
    this.sideWidth = 300,
    this.gap = 16,
    this.breakpoint = 820,
    super.key,
  });

  final Widget side;
  final Widget body;
  final double sideWidth;
  final double gap;
  final double breakpoint;

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        if (constraints.maxWidth < breakpoint) {
          return Column(
            children: [
              SizedBox(height: 220, child: side),
              SizedBox(height: gap),
              Expanded(child: body),
            ],
          );
        }

        return Row(
          children: [
            SizedBox(width: sideWidth, child: side),
            SizedBox(width: gap),
            Expanded(child: body),
          ],
        );
      },
    );
  }
}
