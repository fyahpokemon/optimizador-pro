import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../l10n/strings.dart';
import '../../state/mobile_providers.dart';

/// Navegación inferior del móvil: Inicio · Juegos · Batería · Más.
/// Cada pestaña conserva su estado y su pila (StatefulShellRoute.indexedStack).
class MobileShell extends ConsumerWidget {
  const MobileShell({super.key, required this.shell});

  final StatefulNavigationShell shell;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    // Punto en la pestaña Juegos mientras hay partida en curso.
    final playing = ref.watch(sessionProvider).valueOrNull?.active ?? false;
    final battery = ref.watch(batteryProvider).valueOrNull;
    final pending = battery?.pendingSteps ?? 0;

    return Scaffold(
      body: shell,
      bottomNavigationBar: NavigationBar(
        selectedIndex: shell.currentIndex,
        // Volver a pulsar la pestaña activa la devuelve a su raíz.
        onDestinationSelected: (i) => shell.goBranch(i, initialLocation: i == shell.currentIndex),
        destinations: [
          NavigationDestination(
            icon: const Icon(Icons.speed_outlined),
            selectedIcon: const Icon(Icons.speed_rounded),
            label: context.tr('nav.home'),
          ),
          NavigationDestination(
            icon: Badge(isLabelVisible: playing, child: const Icon(Icons.sports_esports_outlined)),
            selectedIcon: Badge(isLabelVisible: playing, child: const Icon(Icons.sports_esports_rounded)),
            label: context.tr('nav.games'),
          ),
          NavigationDestination(
            icon: Badge.count(
              count: pending,
              isLabelVisible: pending > 0,
              child: const Icon(Icons.battery_std_outlined),
            ),
            selectedIcon: Badge.count(
              count: pending,
              isLabelVisible: pending > 0,
              child: const Icon(Icons.battery_std_rounded),
            ),
            label: context.tr('nav.battery'),
          ),
          NavigationDestination(
            icon: const Icon(Icons.more_horiz_rounded),
            label: context.tr('nav.more'),
          ),
        ],
      ),
    );
  }
}
