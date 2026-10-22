import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../design_system/tokens.dart';
import '../../design_system/widgets/section_card.dart';
import '../../l10n/strings.dart';
import '../../state/providers.dart';
import '../results/result_panel.dart';

/// Cambios aplicados en esta sesión de la app, cada uno con "Deshacer".
class HistoryScreen extends ConsumerWidget {
  const HistoryScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final entries = ref.watch(historyProvider);
    final gutter = Space.gutter(context);
    final time = MaterialLocalizations.of(context);

    return Scaffold(
      appBar: AppBar(title: Text(context.tr('history.title'))),
      body: entries.isEmpty
          ? Center(child: Text(context.tr('history.empty')))
          : ListView.separated(
              padding: EdgeInsets.fromLTRB(gutter, Space.m, gutter, Space.xl),
              itemCount: entries.length,
              separatorBuilder: (_, __) => const SizedBox(height: Space.m),
              itemBuilder: (context, i) {
                final e = entries[i];
                return SectionCard(
                  title: time.formatTimeOfDay(TimeOfDay.fromDateTime(e.when)),
                  trailing: e.undone
                      ? Text(context.tr('result.undone'), style: Theme.of(context).textTheme.labelMedium)
                      : TextButton(
                          onPressed: () => ref.read(historyProvider.notifier).undo(e.session),
                          child: Text(context.tr('result.undo')),
                        ),
                  child: Column(
                    children: [
                      for (final t in e.applied)
                        StatusLine(health: e.undone ? Health.warning : Health.good, text: describeTweak(context, t)),
                    ],
                  ),
                );
              },
            ),
    );
  }
}
