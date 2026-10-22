import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../design_system/tokens.dart';
import '../../design_system/widgets/section_card.dart';
import '../../l10n/strings.dart';
import '../../platform/mobile_extras.dart';
import '../../state/mobile_providers.dart';
import '../../state/providers.dart';

/// Batería: lo que decide si el modo juego puede trabajar en segundo plano y si el ahorro de
/// energía está frenando el móvil. Cada fila dice el estado y lleva a la pantalla exacta.
class BatteryScreen extends ConsumerStatefulWidget {
  const BatteryScreen({super.key});

  @override
  ConsumerState<BatteryScreen> createState() => _BatteryScreenState();
}

class _BatteryScreenState extends ConsumerState<BatteryScreen> {
  late final AppLifecycleListener _lifecycle;

  @override
  void initState() {
    super.initState();
    // Al volver de Ajustes, el estado puede haber cambiado: se vuelve a leer.
    _lifecycle = AppLifecycleListener(onResume: () => ref.invalidate(batteryProvider));
  }

  @override
  void dispose() {
    _lifecycle.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final battery = ref.watch(batteryProvider);
    final gutter = Space.gutter(context);

    return Scaffold(
      appBar: AppBar(title: Text(context.tr('battery.title'))),
      body: RefreshIndicator(
        onRefresh: () => ref.refresh(batteryProvider.future),
        child: battery.when(
          loading: () => const Center(child: CircularProgressIndicator()),
          error: (e, _) => ListView(
            padding: EdgeInsets.all(gutter),
            children: [StatusLine(health: Health.problem, text: context.tr('error.generic'))],
          ),
          data: (b) => ListView(
            padding: EdgeInsets.fromLTRB(gutter, Space.s, gutter, Space.xl),
            children: [
              _Header(b: b),
              const SizedBox(height: Space.m),
              _SetupCard(b: b),
              const SizedBox(height: Space.m),
              _GamesCard(b: b),
              const SizedBox(height: Space.m),
              _HeatCard(b: b),
            ],
          ),
        ),
      ),
    );
  }
}

class _Header extends StatelessWidget {
  const _Header({required this.b});

  final BatteryStatus b;

  @override
  Widget build(BuildContext context) {
    final pending = b.pendingSteps;
    final health = pending == 0 ? Health.good : Health.warning;
    final text = Theme.of(context).textTheme;
    final summary = pending == 0
        ? context.tr('battery.ready')
        : pending == 1
            ? context.tr('battery.pending_one')
            : context.tr('battery.pending', {'n': '$pending'});
    return Semantics(
      container: true,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Wrap(
            spacing: Space.s,
            runSpacing: Space.s,
            children: [
              if (b.levelPct != null)
                _Chip(
                  icon: b.charging ? Icons.battery_charging_full_rounded : Icons.battery_5_bar_rounded,
                  text: '${b.levelPct} %',
                ),
              _Chip(icon: Icons.thermostat_rounded, text: '${b.tempC.toStringAsFixed(1)} °C'),
              _Chip(
                icon: b.charging ? Icons.power_rounded : Icons.power_off_rounded,
                text: context.tr(b.charging ? 'battery.charging' : 'battery.on_battery'),
              ),
            ],
          ),
          const SizedBox(height: Space.m),
          Row(
            children: [
              Icon(health.icon, color: health.color(Theme.of(context).brightness)),
              const SizedBox(width: Space.s),
              Expanded(child: Text(summary, style: text.titleMedium)),
            ],
          ),
        ],
      ),
    );
  }
}

class _Chip extends StatelessWidget {
  const _Chip({required this.icon, required this.text});

  final IconData icon;
  final String text;

  @override
  Widget build(BuildContext context) => Chip(
        avatar: Icon(icon, size: 18),
        label: Text(text),
        visualDensity: VisualDensity.compact,
      );
}

/// Fila de ajuste: estado (icono + texto, nunca solo color) y acción directa.
class _SettingRow extends StatelessWidget {
  const _SettingRow({
    required this.ok,
    required this.title,
    required this.status,
    this.action,
    this.onAction,
  });

  final bool ok;
  final String title;
  final String status;
  final String? action;
  final VoidCallback? onAction;

  @override
  Widget build(BuildContext context) {
    final health = ok ? Health.good : Health.warning;
    final text = Theme.of(context).textTheme;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: Space.s),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Padding(
            padding: const EdgeInsets.only(top: 2),
            child: Icon(health.icon, size: 22, color: health.color(Theme.of(context).brightness)),
          ),
          const SizedBox(width: Space.m),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(title, style: text.bodyLarge?.copyWith(fontWeight: FontWeight.w600)),
                const SizedBox(height: 2),
                Text(status, style: text.bodyMedium),
                if (!ok && action != null)
                  Padding(
                    padding: const EdgeInsets.only(top: Space.s),
                    child: OutlinedButton(onPressed: onAction, child: Text(action!)),
                  ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _SetupCard extends ConsumerWidget {
  const _SetupCard({required this.b});

  final BatteryStatus b;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final agent = ref.read(agentProvider);
    final brand = b.manufacturer.isEmpty ? '' : '${b.manufacturer[0].toUpperCase()}${b.manufacturer.substring(1)}';
    return SectionCard(
      title: context.tr('battery.section_setup'),
      child: Column(
        children: [
          _SettingRow(
            ok: b.canRunInBackground,
            title: context.tr('battery.background_title'),
            status: b.canRunInBackground
                ? context.tr('battery.background_ok')
                : context.tr(b.backgroundRestricted ? 'battery.background_restricted' : 'battery.background_bad'),
            action: context.tr('battery.allow'),
            onAction: () =>
                agent.openScreen(b.backgroundRestricted ? 'app_details' : 'battery_optimization'),
          ),
          _SettingRow(
            ok: !b.powerSave,
            title: context.tr('battery.saver_title'),
            status: context.tr(b.powerSave ? 'battery.saver_on' : 'battery.saver_off'),
            action: context.tr('battery.turn_off'),
            onAction: () => agent.openScreen('battery_saver'),
          ),
          if (b.aggressiveOem)
            // No hay API para leer este ajuste: se muestra siempre como "revisar".
            _SettingRow(
              ok: false,
              title: context.tr('battery.oem_title', {'brand': brand}),
              status: context.tr('battery.oem_desc'),
              action: context.tr('battery.review'),
              onAction: () => agent.openScreen('oem_autostart'),
            ),
          if (b.bucketLimited)
            _SettingRow(
              ok: false,
              title: context.tr('battery.bucket_title'),
              status: context.tr('battery.bucket_bad'),
            ),
        ],
      ),
    );
  }
}

class _GamesCard extends ConsumerWidget {
  const _GamesCard({required this.b});

  final BatteryStatus b;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    if (b.games.isEmpty) return const SizedBox.shrink();
    final elevated = b.elevated == 'ready';
    final text = Theme.of(context).textTheme;
    return SectionCard(
      title: context.tr('battery.section_games'),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          for (final g in b.games)
            ListTile(
              contentPadding: EdgeInsets.zero,
              leading: const Icon(Icons.sports_esports_rounded),
              title: Text(g.label, overflow: TextOverflow.ellipsis),
              subtitle: Text(context.tr(g.exempt ? 'battery.game_exempt' : 'battery.game_not')),
              trailing: Icon(
                g.exempt ? Health.good.icon : Health.warning.icon,
                color: (g.exempt ? Health.good : Health.warning).color(Theme.of(context).brightness),
              ),
            ),
          const SizedBox(height: Space.xs),
          Text(context.tr(elevated ? 'battery.games_hint_elevated' : 'battery.games_hint'), style: text.bodyMedium),
          if (!elevated)
            Padding(
              padding: const EdgeInsets.only(top: Space.s),
              child: OutlinedButton(
                onPressed: () => ref.read(agentProvider).openScreen('battery_optimization'),
                child: Text(context.tr('battery.open_list')),
              ),
            ),
        ],
      ),
    );
  }
}

class _HeatCard extends StatelessWidget {
  const _HeatCard({required this.b});

  final BatteryStatus b;

  @override
  Widget build(BuildContext context) {
    final health = switch (b.thermal) {
      'normal' || 'light' => Health.good,
      'moderate' => Health.warning,
      _ => Health.problem,
    };
    // headroom: 0 = fresco, 1 = a punto de limitar. Se muestra el margen que queda.
    final margin = b.thermalHeadroom == null ? null : ((1 - b.thermalHeadroom!).clamp(0.0, 1.0) * 100).round();
    return SectionCard(
      title: context.tr('battery.section_heat'),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          StatusLine(health: health, text: context.tr('thermal.${b.thermal}')),
          if (margin != null) ...[
            const SizedBox(height: Space.s),
            Semantics(
              label: context.tr('battery.headroom', {'pct': '$margin'}),
              child: ExcludeSemantics(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    ClipRRect(
                      borderRadius: BorderRadius.circular(Radii.chip),
                      child: LinearProgressIndicator(
                        value: margin / 100,
                        minHeight: 8,
                        color: health.color(Theme.of(context).brightness),
                      ),
                    ),
                    const SizedBox(height: Space.xs),
                    Text(
                      context.tr('battery.headroom', {'pct': '$margin'}),
                      style: Theme.of(context).textTheme.labelMedium,
                    ),
                  ],
                ),
              ),
            ),
          ],
        ],
      ),
    );
  }
}
