import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:optimizador_pro/app.dart';
import 'package:optimizador_pro/features/battery/battery_screen.dart';
import 'package:optimizador_pro/domain/models.dart';
import 'package:optimizador_pro/platform/demo_agent.dart';
import 'package:optimizador_pro/state/providers.dart';

Widget _app() => ProviderScope(
      overrides: [agentProvider.overrideWithValue(DemoAgent(latency: Duration.zero))],
      child: const OptimizadorApp(),
    );

void main() {
  testWidgets('escanea, optimiza con 1 clic y permite deshacer', (tester) async {
    tester.platformDispatcher.localesTestValue = const [Locale('es')];
    await tester.pumpWidget(_app());
    await tester.pumpAndSettle();

    expect(find.text('58'), findsOneWidget);
    expect(find.text('Hay apps gastando procesador en segundo plano.'), findsOneWidget);

    await tester.tap(find.text('Optimizar'));
    await tester.pumpAndSettle();

    expect(find.text('92'), findsOneWidget);
    expect(find.text('Hemos aplicado 4 mejoras.'), findsOneWidget);
    expect(find.text('Antes 58 · Ahora 92'), findsOneWidget);
    expect(find.text('Deshacer'), findsOneWidget);

    await tester.tap(find.text('Deshacer'));
    await tester.pumpAndSettle();
    expect(find.text('58'), findsOneWidget);
  });

  testWidgets('el anillo de salud tiene una etiqueta accesible completa', (tester) async {
    tester.platformDispatcher.localesTestValue = const [Locale('es')];
    final handle = tester.ensureSemantics();
    await tester.pumpWidget(_app());
    await tester.pumpAndSettle();
    expect(find.bySemanticsLabel('Salud del sistema: 58 de 100, Mejorable'), findsOneWidget);
    handle.dispose();
  });

  testWidgets('con texto al 200 % no hay desbordamientos', (tester) async {
    tester.platformDispatcher.localesTestValue = const [Locale('es')];
    tester.platformDispatcher.textScaleFactorTestValue = 2.0;
    tester.view.physicalSize = const Size(1080, 2340);
    tester.view.devicePixelRatio = 3.0;
    addTearDown(tester.view.reset);
    addTearDown(tester.platformDispatcher.clearAllTestValues);

    await tester.pumpWidget(_app());
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNull);
  });

  testWidgets('batería: ajustes pendientes con estado y acción', (tester) async {
    tester.platformDispatcher.localesTestValue = const [Locale('es')];
    await tester.pumpWidget(_app());
    await tester.pumpAndSettle();

    await tester.tap(find.text('Batería'));
    await tester.pumpAndSettle();

    // Demo: app no exenta + ahorro de batería activo + fabricante agresivo (Xiaomi).
    expect(find.text('3 ajustes pendientes'), findsOneWidget);
    expect(find.text('Funcionar en segundo plano'), findsOneWidget);
    expect(find.text('Activado: el móvil va más lento'), findsOneWidget);
    expect(find.text('Ajuste de Xiaomi'), findsOneWidget);
    expect(find.text('Permitir'), findsOneWidget);
    // La tarjeta de temperatura queda por debajo de la pantalla de prueba (800×600): se desplaza.
    final margin = find.text('Margen antes de que el móvil se frene: 58 %');
    await tester.scrollUntilVisible(
      margin,
      300,
      scrollable: find.descendant(of: find.byType(BatteryScreen), matching: find.byType(Scrollable)),
    );
    expect(margin, findsOneWidget);
  });

  testWidgets('juegos: modo automático y partida en vivo', (tester) async {
    tester.platformDispatcher.localesTestValue = const [Locale('es')];
    await tester.pumpWidget(_app());
    await tester.pumpAndSettle();

    await tester.tap(find.text('Juegos'));
    await tester.pumpAndSettle();
    expect(find.text('Modo juego automático'), findsOneWidget);
    expect(find.text('Genshin Impact'), findsOneWidget);

    await tester.tap(find.byType(SwitchListTile));
    await tester.pumpAndSettle();
    expect(find.text('Verás una notificación discreta mientras está atento.'), findsOneWidget);

    // La partida emite un estado por segundo: se avanza el reloj a mano (no pumpAndSettle).
    await tester.tap(find.text('Empezar').first);
    await tester.pump();
    await tester.pump(const Duration(seconds: 3));
    expect(find.text('Partida en curso'), findsOneWidget);
    expect(find.text('Límite 45 FPS'), findsOneWidget);
    expect(find.text('Automático'), findsOneWidget);

    await tester.tap(find.text('Terminar'));
    await tester.pump();
    expect(find.text('Partida en curso'), findsNothing);
    expect(
      find.text('Última partida: Call of Duty: Mobile, 24 min. Todo restaurado (4 cambios).'),
      findsOneWidget,
    );
  });

  test('criterio de estabilidad igual que el núcleo', () {
    const steady = FrameStats(avgFps: 60, low1Fps: 56, stuttersPerMin: 0.5, pacingCv: 0.05);
    const shaky = FrameStats(avgFps: 70, low1Fps: 41, stuttersPerMin: 12, pacingCv: 0.2);
    expect(steady.stable, isTrue);
    expect(shaky.stable, isFalse);
  });
}
