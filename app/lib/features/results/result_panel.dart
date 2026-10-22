import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../design_system/tokens.dart';
import '../../design_system/widgets/section_card.dart';
import '../../domain/models.dart';
import '../../l10n/strings.dart';
import '../../state/providers.dart';

/// Resultado de "Optimizar": qué se hizo (con su beneficio), antes/después medido,
/// "Deshacer" y los pasos que requieren al usuario.
class ResultPanel extends ConsumerWidget {
  const ResultPanel({super.key, required this.state, required this.onUndo});

  final HomeState state;
  final VoidCallback onUndo;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final applied = state.lastApply?.applied ?? const [];
    final failed = state.lastApply?.failed ?? 0;
    final guided = state.scan?.guided ?? const [];

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        SectionCard(
          title: context.tr('result.title'),
          trailing: applied.isEmpty
              ? null
              : TextButton.icon(
                  onPressed: onUndo,
                  icon: const Icon(Icons.undo_rounded),
                  label: Text(context.tr('result.undo')),
                ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                applied.isEmpty ? context.tr('result.nothing') : context.tr('result.applied', {'n': '${applied.length}'}),
                style: Theme.of(context).textTheme.bodyLarge,
              ),
              if (state.before != null && state.scan != null)
                Padding(
                  padding: const EdgeInsets.only(top: Space.xs, bottom: Space.s),
                  child: Text(
                    context.tr('result.before_after', {'before': '${state.before}', 'after': '${state.scan!.score}'}),
                    style: Theme.of(context).textTheme.labelMedium,
                  ),
                ),
              for (final t in applied) StatusLine(health: Health.good, text: describeTweak(context, t)),
              if (failed > 0) StatusLine(health: Health.warning, text: context.tr('result.failed', {'n': '$failed'})),
            ],
          ),
        ),
        if (guided.isNotEmpty) ...[
          const SizedBox(height: Space.m),
          SectionCard(
            title: context.tr('result.guided_title'),
            child: Column(
              children: [
                for (final g in guided)
                  ListTile(
                    contentPadding: EdgeInsets.zero,
                    leading: const Icon(Icons.touch_app_rounded),
                    title: Text(context.tr('screen.${g['screen']}')),
                    trailing: OutlinedButton(
                      onPressed: () => ref.read(agentProvider).openScreen(g['screen'] as String),
                      child: Text(context.tr('result.take_me')),
                    ),
                  ),
              ],
            ),
          ),
        ],
      ],
    );
  }
}

/// Texto en lenguaje cotidiano para un ajuste aplicado.
String describeTweak(BuildContext context, Tweak t) => context.tr('tweak.${t.kind}', {
      'name': (t['name'] as String?) ?? '',
      'fps': '${t['fps'] ?? ''}',
    });
