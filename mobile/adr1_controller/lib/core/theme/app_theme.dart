import 'package:flutter/material.dart';

class AppColors {
  static const primary = Color(0xFF2E7D32);       // Agricultural green
  static const primaryDark = Color(0xFF1B5E20);
  static const accent = Color(0xFF66BB6A);
  static const surface = Color(0xFF1E1E2E);
  static const surfaceLight = Color(0xFF2A2A3E);
  static const background = Color(0xFF121220);
  static const error = Color(0xFFEF5350);
  static const warning = Color(0xFFFFA726);
  static const success = Color(0xFF66BB6A);
  static const info = Color(0xFF42A5F5);
  static const textPrimary = Color(0xFFE0E0E0);
  static const textSecondary = Color(0xFF9E9E9E);

  // Sensor health colors
  static const ndviHealthy = Color(0xFF4CAF50);
  static const ndviStressed = Color(0xFFFF9800);
  static const ndviDead = Color(0xFFF44336);

  // Battery colors
  static Color batteryColor(double soc) {
    if (soc > 0.6) return success;
    if (soc > 0.25) return warning;
    return error;
  }
}

class AppTheme {
  static ThemeData get dark => ThemeData(
    useMaterial3: true,
    brightness: Brightness.dark,
    colorScheme: const ColorScheme.dark(
      primary: AppColors.primary,
      secondary: AppColors.accent,
      surface: AppColors.surface,
      error: AppColors.error,
    ),
    scaffoldBackgroundColor: AppColors.background,
    cardTheme: CardTheme(
      color: AppColors.surface,
      elevation: 2,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
    ),
    appBarTheme: const AppBarTheme(
      backgroundColor: AppColors.surface,
      elevation: 0,
      centerTitle: true,
    ),
    bottomNavigationBarTheme: const BottomNavigationBarThemeData(
      backgroundColor: AppColors.surface,
      selectedItemColor: AppColors.accent,
      unselectedItemColor: AppColors.textSecondary,
    ),
    floatingActionButtonTheme: const FloatingActionButtonThemeData(
      backgroundColor: AppColors.primary,
      foregroundColor: Colors.white,
    ),
  );
}
