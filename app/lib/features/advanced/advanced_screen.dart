import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../design_system/tokens.dart';
import '../../design_system/widgets/section_card.dart';
import '../../domain/models.dart';
import '../../l10n/strings.dart';
import '../../state/providers.dart';

/// Panel Avanzado: valores actuales y acciones manuales, todas reversibles desde Historial.
class AdvancedScreen extends ConsumerWidget {
  const AdvancedScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final scan = ref.watch(homeProvider).scan;
    final caps = ref.watch(capabilitiesProvider);
    final gutter = Space.gutter(context);
    final isAndroid = defaultTargetPlatform == TargetPlatform.android;

    return Scaffold(
      appBar: AppBar(title: Text(context.tr('adv.title'))),
      body: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 720),
          child: ListView(
            padding: EdgeInsets.fromLTRB(gutter, Space.m, gutter, Space.xl),
            children: [
              StatusLine(health: Health.warning, text: context.tr('adv.warning')),
              const SizedBox(height: Space.m),
              if (isAndroid)
                caps.when(
                  loading: () => const LinearProgressIndicator(),
                  error: (_, __) => const SizedBox.shrink(),
                  data: (c) => _ElevationCard(caps: c),
                ),
              if (isAndroid) const SizedBox(height: Space.m),
              SectionCard(
                title: context.tr('adv.processes'),
                child: scan == null || scan.processes.isEmpty
                    ? Text(context.tr(isAndroid ? 'game.no_frames' : 'home.scan_first'))
                    : Column(
                        children: [for (final p in scan.processes.take(15)) _ProcessRow(p: p)],
                      ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _ElevationCard extends ConsumerWidget {
  const _ElevationCard({required this.caps});

  final Capabilities caps;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final ready = caps.elevated == 'ready';
    final missing = caps.elevated == 'not_running' || caps.elevated == 'unsupported';
    return SectionCard(
      title: context.tr('adv.elevated'),
      trailing: ready
          ? Chip(label: Text(context.tr('adv.elevated_ready')), avatar: const Icon(Icons.check_rounded, size: 18))
          : null,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(context.tr('adv.elevated_desc')),
          if (!ready) ...[
            const SizedBox(height: Space.s),
            missing
                ? Text(context.tr('adv.elevated_missing'), style: Theme.of(context).textTheme.labelMedium)
                : OutlinedButton(
                    onPressed: () async {
                      await ref.read(agentProvider).requestElevation();
                      ref.invalidate(capabilitiesProvider);
                    },
                    child: Text(context.tr('adv.elevated_enable')),
                  ),
          ],
        ],
      ),
    );
  }
}

class _ProcessRow extends ConsumerWidget {
  const _ProcessRow({required this.p});

  final ProcessRow p;

  @override
  Widget build(BuildContext context, WidgetRef ref) => ListTile(
        contentPadding: EdgeInsets.zero,
        title: Text(p.name, overflow: TextOverflow.ellipsis),
        subtitle: Text(context.tr('adv.cpu', {'pct': p.cpuPct.toStringAsFixed(1)})),
        trailing: TextButton(
          onPressed: () async {
            final r = await ref.read(agentProvider).apply(
              [
                {'kind': 'throttle_background', 'pid': p.pid, 'name': p.name},
              ],
              label: 'manual',
            );
            ref.read(historyProvider.notifier).add(r);
          },
          child: Text(context.tr('adv.throttle')),
        ),
      );
}
