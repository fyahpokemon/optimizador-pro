import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../design_system/tokens.dart';
import '../../design_system/widgets/section_card.dart';
import '../../domain/models.dart';
import '../../l10n/strings.dart';
import '../../platform/mobile_extras.dart';
import '../../state/mobile_providers.dart';
import '../../state/providers.dart';

/// Juegos (móvil): modo automático, partida en vivo y lista de juegos a optimizar.
class MobileGamesScreen extends ConsumerWidget {
  const MobileGamesScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final session = ref.watch(sessionProvider).valueOrNull ?? SessionState.idle;
    final caps = ref.watch(capabilitiesProvider).valueOrNull;
    final gutter = Space.gutter(context);

    return Scaffold(
      appBar: AppBar(title: Text(context.tr('games.title'))),
      body: ListView(
        padding: EdgeInsets.fromLTRB(gutter, Space.s, gutter, Space.xl),
        children: [
          if (caps != null && !caps.usageAccess) ...[
            const _UsageAccessBanner(),
            const SizedBox(height: Space.m),
          ],
          if (session.active) ...[
            LiveSessionCard(session: session),
            const SizedBox(height: Space.m),
          ] else if (session.summary != null) ...[
            StatusLine(
              health: Health.good,
              text: context.tr('session.summary', {
                'label': session.summary!.label,
                'min': '${session.summary!.minutes}',
                'n': '${session.summary!.restored}',
              }),
            ),
            const SizedBox(height: Space.m),
          ],
          const _AutoModeCard(),
          const SizedBox(height: Space.m),
          _GameList(sessionActive: session.active),
        ],
      ),
    );
  }
}

class _UsageAccessBanner extends ConsumerWidget {
  const _UsageAccessBanner();

  @override
  Widget build(BuildContext context, WidgetRef ref) => SectionCard(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            StatusLine(health: Health.warning, text: context.tr('games.usage_needed')),
            const SizedBox(height: Space.s),
            FilledButton.tonal(
              onPressed: () async {
                await ref.read(agentProvider).openScreen('usage_access');
                ref.invalidate(capabilitiesProvider);
              },
              child: Text(context.tr('games.grant')),
            ),
          ],
        ),
      );
}

class _AutoModeCard extends ConsumerWidget {
  const _AutoModeCard();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final auto = ref.watch(autoModeProvider);
    final on = auto.valueOrNull ?? false;
    return SectionCard(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SwitchListTile(
            contentPadding: EdgeInsets.zero,
            value: on,
            onChanged: auto.isLoading ? null : (v) => ref.read(autoModeProvider.notifier).set(v),
            title: Text(
              context.tr('games.auto_title'),
              style: Theme.of(context).textTheme.titleMedium,
            ),
            subtitle: Text(context.tr('games.auto_desc')),
          ),
          if (on)
            Text(context.tr('games.auto_on_hint'), style: Theme.of(context).textTheme.labelMedium),
        ],
      ),
    );
  }
}

/// Tarjeta de la partida en curso. La reutiliza Inicio cuando hay partida.
class LiveSessionCard extends ConsumerWidget {
  const LiveSessionCard({super.key, required this.session});

  final SessionState session;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final text = Theme.of(context).textTheme;
    final stats = session.stats;
    final stable = stats?.stable ?? false;
    final health = stats == null ? Health.warning : (stable ? Health.good : Health.warning);
    final status = stats == null
        ? context.tr(session.measuresFps ? 'session.measuring' : 'session.adjusting')
        : context.tr(stable ? 'session.stable' : 'session.adjusting');

    return SectionCard(
      title: context.tr('session.title'),
      trailing: session.auto
          ? Chip(label: Text(context.tr('session.auto')), visualDensity: VisualDensity.compact)
          : null,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(session.label ?? '', style: text.titleMedium, overflow: TextOverflow.ellipsis),
          const SizedBox(height: Space.s),
          if (stats != null)
            Semantics(
              liveRegion: true,
              label: '${context.tr('game.fps', {'fps': stats.avgFps.toStringAsFixed(0)})}, $status',
              child: ExcludeSemantics(
                child: Row(
                  crossAxisAlignment: CrossAxisAlignment.end,
                  children: [
                    Text(stats.avgFps.toStringAsFixed(0), style: text.displaySmall?.copyWith(fontSize: 48)),
                    const SizedBox(width: Space.s),
                    Padding(
                      padding: const EdgeInsets.only(bottom: 8),
                      child: Text('FPS', style: text.titleMedium),
                    ),
                    const Spacer(),
                    Padding(
                      padding: const EdgeInsets.only(bottom: 8),
                      child: Text(
                        context.tr('session.lows', {'fps': stats.low1Fps.toStringAsFixed(0)}),
                        style: text.bodyMedium,
                      ),
                    ),
                  ],
                ),
              ),
            )
          else if (!session.measuresFps)
            Text(context.tr('session.no_fps'), style: text.bodyMedium),
          StatusLine(health: health, text: status),
          const SizedBox(height: Space.s),
          Wrap(
            spacing: Space.s,
            runSpacing: Space.s,
            children: [
              _InfoChip(
                icon: Icons.speed_rounded,
                text: session.fpsCap == null
                    ? context.tr('session.uncapped')
                    : context.tr('session.cap', {'fps': '${session.fpsCap}'}),
              ),
              if (session.batteryTempC != null)
                _InfoChip(
                  icon: Icons.thermostat_rounded,
                  text: context.tr('session.temp', {'t': session.batteryTempC!.toStringAsFixed(1)}),
                ),
              if (session.applied > 0)
                _InfoChip(icon: Icons.tune_rounded, text: context.tr('session.applied', {'n': '${session.applied}'})),
            ],
          ),
          if (session.capReason != null && session.fpsCap != null && session.capTip == null) ...[
            const SizedBox(height: Space.s),
            Text(
              context.tr('session.reason_${session.capReason}', {'fps': '${session.fpsCap}'}),
              style: text.bodyMedium,
            ),
          ],
          if (session.capTip != null) ...[
            const SizedBox(height: Space.s),
            StatusLine(health: Health.warning, text: context.tr('game.cap_tip', {'fps': '${session.capTip}'})),
          ],
          const SizedBox(height: Space.m),
          OutlinedButton.icon(
            onPressed: () => ref.read(agentProvider).gameStop(),
            icon: const Icon(Icons.stop_rounded),
            label: Text(context.tr('session.stop')),
          ),
        ],
      ),
    );
  }
}

class _InfoChip extends StatelessWidget {
  const _InfoChip({required this.icon, required this.text});

  final IconData icon;
  final String text;

  @override
  Widget build(BuildContext context) =>
      Chip(avatar: Icon(icon, size: 18), label: Text(text), visualDensity: VisualDensity.compact);
}

class _GameList extends ConsumerWidget {
  const _GameList({required this.sessionActive});

  final bool sessionActive;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final games = ref.watch(mobileGamesProvider);
    return SectionCard(
      title: context.tr('games.list'),
      child: games.when(
        loading: () => const Padding(
          padding: EdgeInsets.all(Space.l),
          child: Center(child: CircularProgressIndicator()),
        ),
        error: (_, __) => StatusLine(health: Health.problem, text: context.tr('error.generic')),
        data: (list) => list.isEmpty
            ? Text(context.tr('games.none'))
            : Column(
                children: [
                  for (final g in list)
                    _GameRow(game: g, canStart: !sessionActive && g.enabled),
                ],
              ),
      ),
    );
  }
}

class _GameRow extends ConsumerWidget {
  const _GameRow({required this.game, required this.canStart});

  final MobileGame game;
  final bool canStart;

  @override
  Widget build(BuildContext context, WidgetRef ref) => Padding(
        padding: const EdgeInsets.symmetric(vertical: Space.xs),
        child: Row(
          children: [
            const Icon(Icons.sports_esports_rounded),
            const SizedBox(width: Space.m),
            Expanded(child: Text(game.label, overflow: TextOverflow.ellipsis)),
            if (canStart)
              TextButton(
                onPressed: () =>
                    ref.read(agentProvider).gameStart(GameInfo(id: game.package, label: game.label)),
                child: Text(context.tr('games.start')),
              ),
            Semantics(
              label: game.label, // el interruptor se anuncia con el nombre del juego
              child: Switch(
                value: game.enabled,
                onChanged: (v) => ref.read(mobileGamesProvider.notifier).setEnabled(game.package, v),
              ),
            ),
          ],
        ),
      );
}
