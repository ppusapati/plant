import 'package:fl_chart/fl_chart.dart';
import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';

import '../../../core/theme/app_theme.dart';
import '../../blocs/telemetry/telemetry_bloc.dart';

class SensorsScreen extends StatelessWidget {
  const SensorsScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return BlocBuilder<TelemetryBloc, TelemetryState>(
      builder: (context, state) {
        return SingleChildScrollView(
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // NDVI Section
              _SectionHeader('Plant Health (NDVI)', Icons.local_florist),
              const SizedBox(height: 8),
              _NdviCard(state),
              const SizedBox(height: 16),

              // Soil Section
              _SectionHeader('Soil Analysis', Icons.terrain),
              const SizedBox(height: 8),
              _SoilCard(state),
              const SizedBox(height: 16),

              // Environment Section
              _SectionHeader('Environment', Icons.wb_cloudy),
              const SizedBox(height: 8),
              _EnvironmentCard(state),
              const SizedBox(height: 16),

              // Battery chart
              _SectionHeader('Battery History', Icons.battery_std),
              const SizedBox(height: 8),
              _BatteryChart(state),
            ],
          ),
        );
      },
    );
  }
}

class _SectionHeader extends StatelessWidget {
  final String title;
  final IconData icon;
  const _SectionHeader(this.title, this.icon);

  @override
  Widget build(BuildContext context) {
    return Row(
      children: [
        Icon(icon, color: AppColors.accent, size: 20),
        const SizedBox(width: 8),
        Text(title, style: Theme.of(context).textTheme.titleMedium),
      ],
    );
  }
}

class _NdviCard extends StatelessWidget {
  final TelemetryState state;
  const _NdviCard(this.state);

  @override
  Widget build(BuildContext context) {
    final ndvi = state.latestNdvi;
    if (ndvi == null) {
      return const Card(
        child: Padding(
          padding: EdgeInsets.all(24),
          child: Center(child: Text('No NDVI data yet', style: TextStyle(color: AppColors.textSecondary))),
        ),
      );
    }

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          children: [
            Row(
              children: [
                Expanded(child: _GaugeValue('Mean NDVI', ndvi.meanNdvi.toStringAsFixed(2), _ndviColor(ndvi.meanNdvi))),
                Expanded(child: _GaugeValue('Min', ndvi.minNdvi.toStringAsFixed(2), _ndviColor(ndvi.minNdvi))),
                Expanded(child: _GaugeValue('Max', ndvi.maxNdvi.toStringAsFixed(2), _ndviColor(ndvi.maxNdvi))),
              ],
            ),
            const SizedBox(height: 16),
            // Distribution bar
            Row(
              children: [
                Expanded(
                  flex: (ndvi.healthyPercent * 10).toInt().clamp(1, 100),
                  child: Container(height: 24, color: AppColors.ndviHealthy,
                    child: Center(child: Text('${ndvi.healthyPercent.toStringAsFixed(0)}%', style: const TextStyle(fontSize: 10, color: Colors.white)))),
                ),
                Expanded(
                  flex: (ndvi.stressedPercent * 10).toInt().clamp(1, 100),
                  child: Container(height: 24, color: AppColors.ndviStressed,
                    child: Center(child: Text('${ndvi.stressedPercent.toStringAsFixed(0)}%', style: const TextStyle(fontSize: 10, color: Colors.white)))),
                ),
                Expanded(
                  flex: (ndvi.barePercent * 10).toInt().clamp(1, 100),
                  child: Container(height: 24, color: AppColors.ndviDead,
                    child: Center(child: Text('${ndvi.barePercent.toStringAsFixed(0)}%', style: const TextStyle(fontSize: 10, color: Colors.white)))),
                ),
              ],
            ),
            const SizedBox(height: 4),
            const Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Text('Healthy', style: TextStyle(fontSize: 10, color: AppColors.ndviHealthy)),
                Text('Stressed', style: TextStyle(fontSize: 10, color: AppColors.ndviStressed)),
                Text('Bare/Dead', style: TextStyle(fontSize: 10, color: AppColors.ndviDead)),
              ],
            ),
          ],
        ),
      ),
    );
  }

  Color _ndviColor(double ndvi) {
    if (ndvi > 0.6) return AppColors.ndviHealthy;
    if (ndvi > 0.3) return AppColors.ndviStressed;
    return AppColors.ndviDead;
  }
}

class _SoilCard extends StatelessWidget {
  final TelemetryState state;
  const _SoilCard(this.state);

  @override
  Widget build(BuildContext context) {
    final soil = state.latestSoil;
    if (soil == null) {
      return const Card(
        child: Padding(
          padding: EdgeInsets.all(24),
          child: Center(child: Text('No soil data yet\nDeploy probes to sample', textAlign: TextAlign.center, style: TextStyle(color: AppColors.textSecondary))),
        ),
      );
    }

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          children: [
            Row(
              children: [
                Expanded(child: _GaugeValue('N', '${soil.nitrogen.toStringAsFixed(0)} mg/kg', AppColors.info)),
                Expanded(child: _GaugeValue('P', '${soil.phosphorus.toStringAsFixed(0)} mg/kg', AppColors.warning)),
                Expanded(child: _GaugeValue('K', '${soil.potassium.toStringAsFixed(0)} mg/kg', Colors.purple)),
              ],
            ),
            const SizedBox(height: 12),
            Row(
              children: [
                Expanded(child: _GaugeValue('pH', soil.ph.toStringAsFixed(1), _phColor(soil.ph))),
                Expanded(child: _GaugeValue('EC', '${soil.ec.toStringAsFixed(0)} µS', AppColors.info)),
                Expanded(child: _GaugeValue('Moisture', '${soil.moisture.toStringAsFixed(0)}%', AppColors.info)),
              ],
            ),
            const SizedBox(height: 12),
            // Soil health score bar
            Row(
              children: [
                const Text('Health: ', style: TextStyle(fontSize: 12)),
                Expanded(
                  child: LinearProgressIndicator(
                    value: soil.healthScore / 100,
                    backgroundColor: AppColors.surfaceLight,
                    valueColor: AlwaysStoppedAnimation(_scoreColor(soil.healthScore)),
                    minHeight: 8,
                    borderRadius: BorderRadius.circular(4),
                  ),
                ),
                const SizedBox(width: 8),
                Text('${soil.healthScore.toStringAsFixed(0)}', style: TextStyle(color: _scoreColor(soil.healthScore), fontWeight: FontWeight.bold)),
              ],
            ),
          ],
        ),
      ),
    );
  }

  Color _phColor(double ph) {
    if (ph >= 6.0 && ph <= 7.0) return AppColors.success;
    if (ph >= 5.5 && ph <= 7.5) return AppColors.warning;
    return AppColors.error;
  }

  Color _scoreColor(double score) {
    if (score > 70) return AppColors.success;
    if (score > 40) return AppColors.warning;
    return AppColors.error;
  }
}

class _EnvironmentCard extends StatelessWidget {
  final TelemetryState state;
  const _EnvironmentCard(this.state);

  @override
  Widget build(BuildContext context) {
    final env = state.latestEnvironment;
    if (env == null) {
      return const Card(
        child: Padding(
          padding: EdgeInsets.all(24),
          child: Center(child: Text('Waiting for environment data...', style: TextStyle(color: AppColors.textSecondary))),
        ),
      );
    }

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          children: [
            Row(
              children: [
                Expanded(child: _GaugeValue('Air Temp', '${env.temperature.toStringAsFixed(1)}°C', AppColors.info)),
                Expanded(child: _GaugeValue('Humidity', '${env.humidity.toStringAsFixed(0)}%', AppColors.info)),
                Expanded(child: _GaugeValue('Pressure', '${env.pressure.toStringAsFixed(0)} hPa', AppColors.textSecondary)),
              ],
            ),
            const SizedBox(height: 12),
            Row(
              children: [
                Expanded(child: _GaugeValue('Leaf Temp', '${env.leafTemp.toStringAsFixed(1)}°C',
                    env.waterStress ? AppColors.error : AppColors.success)),
                Expanded(child: _GaugeValue('Light', '${env.lightLux.toStringAsFixed(0)} lux', AppColors.warning)),
                Expanded(child: _GaugeValue('UV Index', env.uvIndex.toStringAsFixed(1),
                    env.uvIndex > 8 ? AppColors.error : AppColors.success)),
              ],
            ),
            if (env.waterStress) ...[
              const SizedBox(height: 8),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
                decoration: BoxDecoration(
                  color: AppColors.error.withValues(alpha: 0.15),
                  borderRadius: BorderRadius.circular(8),
                ),
                child: Row(
                  children: [
                    const Icon(Icons.warning, color: AppColors.error, size: 16),
                    const SizedBox(width: 8),
                    Text(
                      'Water stress detected: leaf temp ${env.leafStressDelta.toStringAsFixed(1)}°C above ambient',
                      style: const TextStyle(color: AppColors.error, fontSize: 12),
                    ),
                  ],
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }
}

class _BatteryChart extends StatelessWidget {
  final TelemetryState state;
  const _BatteryChart(this.state);

  @override
  Widget build(BuildContext context) {
    if (state.batteryHistory.isEmpty) {
      return const Card(
        child: Padding(
          padding: EdgeInsets.all(24),
          child: Center(child: Text('No battery history yet', style: TextStyle(color: AppColors.textSecondary))),
        ),
      );
    }

    final spots = state.batteryHistory.asMap().entries.map((e) {
      return FlSpot(e.key.toDouble(), e.value.soc);
    }).toList();

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: SizedBox(
          height: 150,
          child: LineChart(
            LineChartData(
              gridData: const FlGridData(show: false),
              titlesData: const FlTitlesData(show: false),
              borderData: FlBorderData(show: false),
              minY: 0,
              maxY: 100,
              lineBarsData: [
                LineChartBarData(
                  spots: spots,
                  isCurved: true,
                  color: AppColors.accent,
                  barWidth: 2,
                  dotData: const FlDotData(show: false),
                  belowBarData: BarAreaData(
                    show: true,
                    color: AppColors.accent.withValues(alpha: 0.15),
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _GaugeValue extends StatelessWidget {
  final String label;
  final String value;
  final Color color;
  const _GaugeValue(this.label, this.value, this.color);

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        Text(value, style: TextStyle(fontSize: 18, fontWeight: FontWeight.bold, color: color)),
        const SizedBox(height: 2),
        Text(label, style: const TextStyle(fontSize: 11, color: AppColors.textSecondary)),
      ],
    );
  }
}
