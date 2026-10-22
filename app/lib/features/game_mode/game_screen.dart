import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../design_system/tokens.dart';
import '../../design_system/widgets/section_card.dart';
import '../../domain/models.dart';
import '../../l10n/strings.dart';
import '../../state/providers.dart';

/// Modo juego: elegir juego, ver estabilidad en vivo y terminar (deshace todo).
class GameScreen extends ConsumerWidget {
  const GameScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final game = ref.watch(gameProvider);
    final caps = ref.watch(capabilitiesProvider).valueOrNull;
    final gutter = Space.gutter(context);

    return Scaffold(
      appBar: AppBar(title: Text(context.tr('game.title'))),
      body: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 560),
          child: ListView(
            padding: EdgeInsets.fromLTRB(gutter, Space.m, gutter, Space.xl),
            children: [
              if (game.running)
                _LiveCard(state: game, measuresFps: caps?.measuresFps ?? true)
              else ...[
                if (game.summary != null)
                  Padding(
                    padding: const EdgeInsets.only(bottom: Space.m),
                    child: StatusLine(
                      health: Health.good,
                      text: context.tr('game.summary', {'n': '${game.summary}'}),
                    ),
                  ),
                const _GamePicker(),
              ],
            ],
          ),
        ),
      ),
    );
  }
}

class _GamePicker extends ConsumerWidget {
  const _GamePicker();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final games = ref.watch(gamesProvider);
    return games.when(
      loading: () => const Center(child: Padding(padding: EdgeInsets.all(Space.xl), child: CircularProgressIndicator())),
      error: (e, _) => StatusLine(health: Health.problem, text: context.tr('error.generic')),
      data: (list) => list.isEmpty
          ? Column(
              children: [
                Text(context.tr('game.none'), textAlign: TextAlign.center),
                const SizedBox(height: Space.m),
                OutlinedButton(onPressed: () => ref.invalidate(gamesProvider), child: Text(context.tr('retry'))),
              ],
            )
          : SectionCard(
              child: Column(
                children: [
                  for (final g in list)
                    ListTile(
                      contentPadding: EdgeInsets.zero,
                      leading: const Icon(Icons.sports_esports_rounded),
                      title: Text(g.label),
                      trailing: FilledButton.tonal(
                        onPressed: () => ref.read(gameProvider.notifier).start(g),
                        child: Text(context.tr('game.start')),
                      ),
                    ),
                ],
              ),
            ),
    );
  }
}

class _LiveCard extends ConsumerWidget {
  const _LiveCard({required this.state, required this.measuresFps});

  final GameState state;
  final bool measuresFps;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final stats = state.tick?.stats;
    final stable = stats?.stable ?? false;
    final health = stats == null ? Health.warning : (stable ? Health.good : Health.warning);
    final text = Theme.of(context).textTheme;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        SectionCard(
          title: state.game!.label,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Semantics(
                liveRegion: true,
                child: StatusLine(
                  health: health,
                  text: context.tr(stable ? 'game.stable' : 'game.unstable'),
                ),
              ),
              if (stats != null) ...[
                const SizedBox(height: Space.s),
                Text(context.tr('game.fps', {'fps': stats.avgFps.toStringAsFixed(0)}), style: text.displaySmall),
                Text(context.tr('game.low1', {'fps': stats.low1Fps.toStringAsFixed(0)}), style: text.bodyMedium),
              ] else if (!measuresFps)
                Padding(
                  padding: const EdgeInsets.only(top: Space.s),
                  child: Text(context.tr('game.no_frames'), style: text.bodyMedium),
                ),
              if (state.capTip != null)
                Padding(
                  padding: const EdgeInsets.only(top: Space.m),
                  child: StatusLine(
                    health: Health.warning,
                    text: context.tr('game.cap_tip', {'fps': '${state.capTip}'}),
                  ),
                ),
            ],
          ),
        ),
        const SizedBox(height: Space.l),
        OutlinedButton.icon(
          onPressed: () => ref.read(gameProvider.notifier).stop(),
          icon: const Icon(Icons.stop_rounded),
          label: Text(context.tr('game.stop')),
        ),
      ],
    );
  }
}
