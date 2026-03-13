import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../core/constants/protocol.dart';
import '../../../core/theme/app_theme.dart';
import '../../blocs/connection/connection_bloc.dart' as conn;
import '../../blocs/telemetry/telemetry_bloc.dart';
import '../../providers/providers.dart';
import '../../widgets/battery_indicator.dart';
import '../../widgets/connection_badge.dart';
import '../../widgets/telemetry_card.dart';
import '../control/control_screen.dart';
import '../map/map_screen.dart';
import '../sensors/sensors_screen.dart';
import '../settings/settings_screen.dart';

class DashboardScreen extends ConsumerStatefulWidget {
  const DashboardScreen({super.key});

  @override
  ConsumerState<DashboardScreen> createState() => _DashboardScreenState();
}

class _DashboardScreenState extends ConsumerState<DashboardScreen> {
  int _currentIndex = 0;

  @override
  Widget build(BuildContext context) {
    final connectionBloc = ref.watch(connectionBlocProvider);
    final telemetryBloc = ref.watch(telemetryBlocProvider);
    final controlBloc = ref.watch(controlBlocProvider);

    return MultiBlocProvider(
      providers: [
        BlocProvider.value(value: connectionBloc),
        BlocProvider.value(value: telemetryBloc),
        BlocProvider.value(value: controlBloc),
      ],
      child: Scaffold(
        appBar: AppBar(
          title: const Text('ADR-1 Controller'),
          leading: Padding(
            padding: const EdgeInsets.all(8),
            child: Image.asset(
              'assets/icons/app_icon.png',
              errorBuilder: (_, __, ___) => const Icon(Icons.agriculture, color: AppColors.accent),
            ),
          ),
          actions: [
            BlocBuilder<TelemetryBloc, TelemetryState>(
              builder: (context, state) => BatteryIndicator(
                soc: state.telemetry.battery.soc,
                charging: state.telemetry.battery.charging,
              ),
            ),
            const SizedBox(width: 8),
            BlocBuilder<conn.ConnectionBloc, conn.ConnectionState>(
              builder: (context, state) => ConnectionBadge(state: state),
            ),
            const SizedBox(width: 16),
          ],
        ),
        body: IndexedStack(
          index: _currentIndex,
          children: const [
            _DashboardHome(),
            ControlScreen(),
            MapScreen(),
            SensorsScreen(),
            SettingsScreen(),
          ],
        ),
        bottomNavigationBar: NavigationBar(
          selectedIndex: _currentIndex,
          onDestinationSelected: (i) => setState(() => _currentIndex = i),
          destinations: const [
            NavigationDestination(icon: Icon(Icons.dashboard), label: 'Dashboard'),
            NavigationDestination(icon: Icon(Icons.gamepad), label: 'Control'),
            NavigationDestination(icon: Icon(Icons.map), label: 'Map'),
            NavigationDestination(icon: Icon(Icons.sensors), label: 'Sensors'),
            NavigationDestination(icon: Icon(Icons.settings), label: 'Settings'),
          ],
        ),
      ),
    );
  }
}

class _DashboardHome extends StatelessWidget {
  const _DashboardHome();

  @override
  Widget build(BuildContext context) {
    return BlocBuilder<TelemetryBloc, TelemetryState>(
      builder: (context, state) {
        final t = state.telemetry;
        final mode = RobotMode.values.firstWhere(
          (m) => m.value == t.mode,
          orElse: () => RobotMode.idle,
        );

        return SingleChildScrollView(
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // Mode banner
              Container(
                width: double.infinity,
                padding: const EdgeInsets.symmetric(vertical: 12, horizontal: 16),
                decoration: BoxDecoration(
                  color: _modeColor(mode).withValues(alpha: 0.15),
                  borderRadius: BorderRadius.circular(12),
                  border: Border.all(color: _modeColor(mode), width: 1.5),
                ),
                child: Row(
                  children: [
                    Text(mode.icon, style: const TextStyle(fontSize: 28)),
                    const SizedBox(width: 12),
                    Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(mode.label, style: Theme.of(context).textTheme.titleLarge?.copyWith(color: _modeColor(mode))),
                        Text(
                          t.connected ? 'Connected • RSSI ${t.rssi} dBm' : 'Disconnected',
                          style: TextStyle(color: t.connected ? AppColors.textSecondary : AppColors.error),
                        ),
                      ],
                    ),
                  ],
                ),
              ),
              const SizedBox(height: 16),

              // Quick stats grid
              GridView.count(
                crossAxisCount: 2,
                shrinkWrap: true,
                physics: const NeverScrollableScrollPhysics(),
                mainAxisSpacing: 12,
                crossAxisSpacing: 12,
                childAspectRatio: 1.6,
                children: [
                  TelemetryCard(
                    title: 'Battery',
                    value: '${t.battery.soc.toStringAsFixed(0)}%',
                    subtitle: '${t.battery.voltage.toStringAsFixed(1)}V  ${t.battery.current.toStringAsFixed(1)}A',
                    icon: Icons.battery_std,
                    color: AppColors.batteryColor(t.battery.soc / 100),
                  ),
                  TelemetryCard(
                    title: 'Speed',
                    value: '${t.position.speed.toStringAsFixed(1)} m/s',
                    subtitle: 'Heading: ${t.position.heading.toStringAsFixed(0)}°',
                    icon: Icons.speed,
                    color: AppColors.info,
                  ),
                  TelemetryCard(
                    title: 'GNSS',
                    value: '${t.position.satellites} sats',
                    subtitle: 'HDOP: ${t.position.hdop.toStringAsFixed(1)}',
                    icon: Icons.satellite_alt,
                    color: t.position.fixValid ? AppColors.success : AppColors.warning,
                  ),
                  TelemetryCard(
                    title: 'Obstacles',
                    value: '${(t.obstacles.minDistance / 10).toStringAsFixed(0)} cm',
                    subtitle: 'F:${(t.obstacles.frontMm / 10).toStringAsFixed(0)} R:${(t.obstacles.rightMm / 10).toStringAsFixed(0)} B:${(t.obstacles.rearMm / 10).toStringAsFixed(0)} L:${(t.obstacles.leftMm / 10).toStringAsFixed(0)}',
                    icon: Icons.radar,
                    color: t.obstacles.minDistance < 500 ? AppColors.error : AppColors.success,
                  ),
                ],
              ),
              const SizedBox(height: 16),

              // Environment summary
              if (state.latestEnvironment != null) ...[
                Text('Environment', style: Theme.of(context).textTheme.titleMedium),
                const SizedBox(height: 8),
                Card(
                  child: Padding(
                    padding: const EdgeInsets.all(16),
                    child: Row(
                      mainAxisAlignment: MainAxisAlignment.spaceAround,
                      children: [
                        _EnvChip(Icons.thermostat, '${state.latestEnvironment!.temperature.toStringAsFixed(1)}°C'),
                        _EnvChip(Icons.water_drop, '${state.latestEnvironment!.humidity.toStringAsFixed(0)}%'),
                        _EnvChip(Icons.light_mode, '${state.latestEnvironment!.lightLux.toStringAsFixed(0)} lx'),
                        _EnvChip(Icons.air, '${state.latestEnvironment!.windSpeed.toStringAsFixed(1)} m/s'),
                        _EnvChip(Icons.wb_sunny, 'UV ${state.latestEnvironment!.uvIndex.toStringAsFixed(1)}'),
                      ],
                    ),
                  ),
                ),
              ],

              const SizedBox(height: 16),

              // Mission progress (if active)
              if (t.mission != null) ...[
                Text('Mission Progress', style: Theme.of(context).textTheme.titleMedium),
                const SizedBox(height: 8),
                Card(
                  child: Padding(
                    padding: const EdgeInsets.all(16),
                    child: Column(
                      children: [
                        LinearProgressIndicator(
                          value: t.mission!.progressPercent / 100,
                          backgroundColor: AppColors.surfaceLight,
                          valueColor: const AlwaysStoppedAnimation(AppColors.accent),
                          minHeight: 8,
                          borderRadius: BorderRadius.circular(4),
                        ),
                        const SizedBox(height: 8),
                        Row(
                          mainAxisAlignment: MainAxisAlignment.spaceBetween,
                          children: [
                            Text('WP ${t.mission!.currentWaypoint}/${t.mission!.totalWaypoints}'),
                            Text('${t.mission!.areaCoveredSqM.toStringAsFixed(0)} m²'),
                            Text('${(t.mission!.elapsedSeconds / 60).toStringAsFixed(0)} min'),
                          ],
                        ),
                      ],
                    ),
                  ),
                ),
              ],
            ],
          ),
        );
      },
    );
  }

  Color _modeColor(RobotMode mode) => switch (mode) {
    RobotMode.idle => AppColors.textSecondary,
    RobotMode.manual => AppColors.info,
    RobotMode.autonomous => AppColors.accent,
    RobotMode.deseeding => AppColors.primary,
    RobotMode.returnHome => Colors.purple,
    RobotMode.estop => AppColors.error,
    RobotMode.charging => AppColors.warning,
    RobotMode.calibrating => AppColors.info,
  };
}

class _EnvChip extends StatelessWidget {
  final IconData icon;
  final String value;
  const _EnvChip(this.icon, this.value);

  @override
  Widget build(BuildContext context) {
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        Icon(icon, size: 20, color: AppColors.textSecondary),
        const SizedBox(height: 4),
        Text(value, style: const TextStyle(fontSize: 12)),
      ],
    );
  }
}
