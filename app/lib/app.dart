import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'design_system/theme.dart';
import 'features/advanced/advanced_screen.dart';
import 'features/battery/battery_screen.dart';
import 'features/game_mode/game_screen.dart';
import 'features/game_mode/mobile_games_screen.dart';
import 'features/history/history_screen.dart';
import 'features/home/home_screen.dart';
import 'features/more/more_screen.dart';
import 'features/shell/mobile_shell.dart';
import 'l10n/strings.dart';
import 'state/mobile_providers.dart';

/// Móvil: navegación inferior con cuatro pestañas.
GoRouter _mobileRouter() => GoRouter(
      routes: [
        StatefulShellRoute.indexedStack(
          builder: (context, state, shell) => MobileShell(shell: shell),
          branches: [
            StatefulShellBranch(routes: [GoRoute(path: '/', builder: (_, __) => const HomeScreen())]),
            StatefulShellBranch(routes: [GoRoute(path: '/games', builder: (_, __) => const MobileGamesScreen())]),
            StatefulShellBranch(routes: [GoRoute(path: '/battery', builder: (_, __) => const BatteryScreen())]),
            StatefulShellBranch(
              routes: [
                GoRoute(
                  path: '/more',
                  builder: (_, __) => const MoreScreen(),
                  routes: [
                    GoRoute(path: 'advanced', builder: (_, __) => const AdvancedScreen()),
                    GoRoute(path: 'history', builder: (_, __) => const HistoryScreen()),
                  ],
                ),
              ],
            ),
          ],
        ),
      ],
    );

/// Escritorio: una pantalla principal con enlaces.
GoRouter _desktopRouter() => GoRouter(
      routes: [
        GoRoute(path: '/', builder: (_, __) => const HomeScreen()),
        GoRoute(path: '/game', builder: (_, __) => const GameScreen()),
        GoRoute(path: '/advanced', builder: (_, __) => const AdvancedScreen()),
        GoRoute(path: '/history', builder: (_, __) => const HistoryScreen()),
      ],
    );

class OptimizadorApp extends ConsumerStatefulWidget {
  const OptimizadorApp({super.key});

  @override
  ConsumerState<OptimizadorApp> createState() => _OptimizadorAppState();
}

class _OptimizadorAppState extends ConsumerState<OptimizadorApp> {
  // El router se crea una sola vez: recrearlo en cada build perdería la navegación.
  late final GoRouter _router =
      ref.read(mobileExtrasProvider) != null ? _mobileRouter() : _desktopRouter();

  @override
  void dispose() {
    _router.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => MaterialApp.router(
        onGenerateTitle: (context) => context.tr('app.title'),
        debugShowCheckedModeBanner: false,
        theme: buildTheme(Brightness.light),
        darkTheme: buildTheme(Brightness.dark),
        highContrastTheme: buildTheme(Brightness.light),
        highContrastDarkTheme: buildTheme(Brightness.dark),
        themeMode: ThemeMode.system,
        supportedLocales: supportedLocales,
        localizationsDelegates: GlobalMaterialLocalizations.delegates,
        routerConfig: _router,
      );
}
