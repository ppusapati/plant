import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:hive_flutter/hive_flutter.dart';
import 'package:wakelock_plus/wakelock_plus.dart';

import 'core/theme/app_theme.dart';
import 'presentation/screens/dashboard/dashboard_screen.dart';
import 'data/datasources/local_storage.dart';

void main() async {
  WidgetsFlutterBinding.ensureInitialized();

  // Initialize Hive for local storage
  await Hive.initFlutter();
  await LocalStorage.init();

  // Lock to landscape for controller use, allow portrait too
  await SystemChrome.setPreferredOrientations([
    DeviceOrientation.portraitUp,
    DeviceOrientation.landscapeLeft,
    DeviceOrientation.landscapeRight,
  ]);

  // Keep screen on during operation
  await WakelockPlus.enable();

  runApp(const ProviderScope(child: ADR1App()));
}

class ADR1App extends StatelessWidget {
  const ADR1App({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'ADR-1 Controller',
      debugShowCheckedModeBanner: false,
      theme: AppTheme.dark,
      home: const DashboardScreen(),
    );
  }
}
