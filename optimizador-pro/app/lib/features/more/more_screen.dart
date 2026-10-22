import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';

import '../../design_system/tokens.dart';
import '../../design_system/widgets/section_card.dart';
import '../../l10n/strings.dart';

/// Pestaña "Más" (móvil): acceso a Avanzado e Historial.
class MoreScreen extends StatelessWidget {
  const MoreScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final gutter = Space.gutter(context);
    return Scaffold(
      appBar: AppBar(title: Text(context.tr('more.title'))),
      body: ListView(
        padding: EdgeInsets.fromLTRB(gutter, Space.s, gutter, Space.xl),
        children: [
          SectionCard(
            child: Column(
              children: [
                ListTile(
                  contentPadding: EdgeInsets.zero,
                  leading: const Icon(Icons.tune_rounded),
                  title: Text(context.tr('home.advanced')),
                  subtitle: Text(context.tr('more.advanced_desc')),
                  trailing: const Icon(Icons.chevron_right_rounded),
                  onTap: () => context.go('/more/advanced'),
                ),
                const Divider(height: 1),
                ListTile(
                  contentPadding: EdgeInsets.zero,
                  leading: const Icon(Icons.history_rounded),
                  title: Text(context.tr('home.history')),
                  subtitle: Text(context.tr('more.history_desc')),
                  trailing: const Icon(Icons.chevron_right_rounded),
                  onTap: () => context.go('/more/history'),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
