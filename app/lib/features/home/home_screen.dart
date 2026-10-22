import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../design_system/tokens.dart';
import '../../design_system/widgets/health_ring.dart';
import '../../design_system/widgets/section_card.dart';
import '../../l10n/strings.dart';
import '../../platform/agent.dart';
import '../../state/mobile_providers.dart';
import '../../state/providers.dart';
import '../game_mode/mobile_games_screen.dart';
import '../results/result_panel.dart';

/// Pantalla principal: una pregunta, un número, una frase y un botón.
class HomeScreen extends ConsumerWidget {
  const HomeScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final s = ref.watch(homeProvider);
    final ctrl = ref.read(homeProvider.notifier);
    final busy = s.phase == OptimizePhase.scanning || s.phase == OptimizePhase.applying;
    final gutter = Space.gutter(context);
    // En el móvil hay pestañas: el modo juego y los enlaces viven en ellas.
    final mobile = ref.watch(mobileExtrasProvider) != null;
    final session = mobile ? ref.watch(sessionProvider).valueOrNull : null;

    return Scaffold(
      body: SafeArea(
        child: Center(
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 560),
            child: ListView(
              padding: EdgeInsets.fromLTRB(gutter, Space.l, gutter, Space.xl),
              children: [
                Semantics(
                  header: true,
                  child: Text(
                    context.tr(mobile ? 'home.question_mobile' : 'home.question'),
                    style: Theme.of(context).textTheme.titleLarge,
                    textAlign: TextAlign.center,
                  ),
                ),
                const SizedBox(height: Space.l),
                Center(
                  child: Stack(
                    alignment: Alignment.center,
                    children: [
                      HealthRing(
                        score: s.phase == OptimizePhase.scanning && s.scan == null ? null : s.scan?.score,
                        labelKey: s.scan?.labelKey ?? 'home.scan_first',
                        busy: busy,
                      ),
                      if (busy)
                        const SizedBox.square(
                          dimension: 200,
                          child: CircularProgressIndicator(strokeWidth: 14, strokeCap: StrokeCap.round),
                        ),
                    ],
                  ),
                ),
                const SizedBox(height: Space.m),
                if (s.error != null)
                  _ErrorBanner(error: s.error!, onRetry: ctrl.rescan)
                else
                  Text(
                    context.tr(busy && s.scan == null ? 'home.scan_first' : (s.scan?.headlineKey ?? 'home.scan_first')),
                    style: Theme.of(context).textTheme.bodyLarge,
                    textAlign: TextAlign.center,
                  ),
                const SizedBox(height: Space.l),
                FilledButton.icon(
                  onPressed: busy || s.scan == null ? null : ctrl.optimize,
                  icon: busy
                      ? const SizedBox.square(dimension: 20, child: CircularProgressIndicator(strokeWidth: 2.5))
                      : const Icon(Icons.bolt_rounded),
                  label: Text(context.tr(s.phase == OptimizePhase.applying ? 'home.optimizing' : 'home.optimize')),
                ),
                if (s.phase == OptimizePhase.done) ...[
                  const SizedBox(height: Space.m),
                  ResultPanel(state: s, onUndo: ctrl.undoLast),
                ],
                const SizedBox(height: Space.l),
                if (session != null && session.active)
                  LiveSessionCard(session: session)
                else
                  SectionCard(
                    title: context.tr('home.game_mode'),
                    trailing: const Icon(Icons.chevron_right_rounded),
                    onTap: () => mobile ? context.go('/games') : context.push('/game'),
                    child: Text(context.tr('home.game_mode_desc'), style: Theme.of(context).textTheme.bodyMedium),
                  ),
                const SizedBox(height: Space.m),
                // Wrap en vez de Row: con texto grande los enlaces pasan a otra línea.
                if (!mobile)
                  Wrap(
                    alignment: WrapAlignment.center,
                    spacing: Space.s,
                    children: [
                      TextButton.icon(
                        onPressed: () => context.push('/history'),
                        icon: const Icon(Icons.history_rounded),
                        label: Text(context.tr('home.history')),
                      ),
                      TextButton.icon(
                        onPressed: () => context.push('/advanced'),
                        icon: const Icon(Icons.tune_rounded),
                        label: Text(context.tr('home.advanced')),
                      ),
                    ],
                  ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _ErrorBanner extends StatelessWidget {
  const _ErrorBanner({required this.error, required this.onRetry});

  final Object error;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) => Column(
        children: [
          StatusLine(
            health: Health.problem,
            text: context.tr(error is AgentUnavailable ? 'error.agent' : 'error.generic'),
          ),
          TextButton(onPressed: onRetry, child: Text(context.tr('retry'))),
        ],
      );
}
