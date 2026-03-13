import 'package:flutter/material.dart';
import '../../core/theme/app_theme.dart';

class BatteryIndicator extends StatelessWidget {
  final double soc;
  final bool charging;

  const BatteryIndicator({super.key, required this.soc, required this.charging});

  @override
  Widget build(BuildContext context) {
    final color = AppColors.batteryColor(soc / 100);
    final icon = charging
        ? Icons.battery_charging_full
        : soc > 80 ? Icons.battery_full
        : soc > 50 ? Icons.battery_5_bar
        : soc > 20 ? Icons.battery_3_bar
        : Icons.battery_1_bar;

    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Icon(icon, color: color, size: 20),
        const SizedBox(width: 2),
        Text(
          '${soc.toStringAsFixed(0)}%',
          style: TextStyle(color: color, fontSize: 12, fontWeight: FontWeight.bold),
        ),
      ],
    );
  }
}
