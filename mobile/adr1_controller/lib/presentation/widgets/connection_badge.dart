import 'package:flutter/material.dart';
import '../../core/theme/app_theme.dart';
import '../blocs/connection/connection_bloc.dart' as conn;

class ConnectionBadge extends StatelessWidget {
  final conn.ConnectionState state;

  const ConnectionBadge({super.key, required this.state});

  @override
  Widget build(BuildContext context) {
    final (icon, color, label) = switch (state) {
      conn.Connected(:final type) => (
        type == conn.ConnectionType.wifi ? Icons.wifi : Icons.bluetooth_connected,
        AppColors.success,
        'Connected',
      ),
      conn.Connecting() => (Icons.sync, AppColors.warning, 'Connecting'),
      conn.Reconnecting() => (Icons.refresh, AppColors.warning, 'Reconnecting'),
      conn.ConnectionFailed() => (Icons.error_outline, AppColors.error, 'Failed'),
      _ => (Icons.link_off, AppColors.textSecondary, 'Offline'),
    };

    return Tooltip(
      message: label,
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
        decoration: BoxDecoration(
          color: color.withValues(alpha: 0.15),
          borderRadius: BorderRadius.circular(12),
          border: Border.all(color: color.withValues(alpha: 0.5)),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(icon, color: color, size: 14),
            const SizedBox(width: 4),
            Text(label, style: TextStyle(color: color, fontSize: 11)),
          ],
        ),
      ),
    );
  }
}
