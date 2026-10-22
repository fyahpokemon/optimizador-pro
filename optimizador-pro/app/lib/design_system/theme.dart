import 'package:flutter/material.dart';

import 'tokens.dart';

/// Temas claro y oscuro. Contraste verificado WCAG 2.2 AA (4,5:1 en texto).
/// Tipografía: Figtree (la del diseño) si se añade a `assets/fonts` (ver README); si no, la del sistema.
ThemeData buildTheme(Brightness brightness) {
  final dark = brightness == Brightness.dark;
  final scheme = ColorScheme.fromSeed(
    seedColor: Palette.accent,
    brightness: brightness,
    primary: dark ? Palette.accentDark : Palette.accent,
    surface: dark ? Palette.surfaceDark : Palette.surfaceLight,
  );
  final base = ThemeData(useMaterial3: true, colorScheme: scheme, brightness: brightness);
  final text = base.textTheme.copyWith(
    displaySmall: const TextStyle(fontSize: TypeScale.score, fontWeight: FontWeight.w700, height: 1.1),
    titleLarge: const TextStyle(fontSize: TypeScale.title, fontWeight: FontWeight.w600, height: 1.25),
    bodyLarge: const TextStyle(fontSize: TypeScale.body, height: 1.4),
    bodyMedium: const TextStyle(fontSize: TypeScale.secondary, height: 1.4),
    labelMedium: const TextStyle(fontSize: TypeScale.label, fontWeight: FontWeight.w500),
  );

  return base.copyWith(
    scaffoldBackgroundColor: dark ? Palette.bgDark : Palette.bgLight,
    textTheme: text.apply(
      fontFamily: 'Figtree',
      bodyColor: scheme.onSurface,
      displayColor: scheme.onSurface,
    ),
    cardTheme: CardThemeData(
      elevation: 0,
      color: scheme.surface,
      margin: EdgeInsets.zero,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(Radii.card)),
    ),
    filledButtonTheme: FilledButtonThemeData(
      style: FilledButton.styleFrom(
        minimumSize: const Size.fromHeight(64), // área táctil generosa
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(Radii.button)),
        textStyle: const TextStyle(fontSize: TypeScale.body, fontWeight: FontWeight.w600),
      ),
    ),
    outlinedButtonTheme: OutlinedButtonThemeData(
      style: OutlinedButton.styleFrom(
        minimumSize: const Size(48, 48),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(Radii.button)),
      ),
    ),
    // Foco visible para teclado (Windows).
    focusColor: scheme.primary.withValues(alpha: 0.24),
    visualDensity: VisualDensity.standard,
    materialTapTargetSize: MaterialTapTargetSize.padded,
  );
}
