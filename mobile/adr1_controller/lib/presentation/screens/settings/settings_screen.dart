import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';

import '../../../core/theme/app_theme.dart';
import '../../../data/datasources/local_storage.dart';
import '../../blocs/connection/connection_bloc.dart';

class SettingsScreen extends StatefulWidget {
  const SettingsScreen({super.key});

  @override
  State<SettingsScreen> createState() => _SettingsScreenState();
}

class _SettingsScreenState extends State<SettingsScreen> {
  final _ipController = TextEditingController();
  String _connectionType = 'wifi';

  @override
  void initState() {
    super.initState();
    _ipController.text = LocalStorage.getLastRobotAddress() ?? '192.168.4.1';
    _connectionType = LocalStorage.getConnectionType();
  }

  @override
  Widget build(BuildContext context) {
    return SingleChildScrollView(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Connection Settings
          Text('Connection', style: Theme.of(context).textTheme.titleMedium),
          const SizedBox(height: 8),
          Card(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                children: [
                  // Connection type selector
                  SegmentedButton<String>(
                    segments: const [
                      ButtonSegment(value: 'wifi', label: Text('WiFi'), icon: Icon(Icons.wifi)),
                      ButtonSegment(value: 'ble', label: Text('BLE'), icon: Icon(Icons.bluetooth)),
                    ],
                    selected: {_connectionType},
                    onSelectionChanged: (s) {
                      setState(() => _connectionType = s.first);
                      LocalStorage.setConnectionType(_connectionType);
                    },
                  ),
                  const SizedBox(height: 16),

                  if (_connectionType == 'wifi') ...[
                    TextField(
                      controller: _ipController,
                      decoration: const InputDecoration(
                        labelText: 'Robot IP Address',
                        hintText: '192.168.4.1',
                        prefixIcon: Icon(Icons.router),
                        border: OutlineInputBorder(),
                      ),
                      keyboardType: TextInputType.url,
                    ),
                    const SizedBox(height: 12),
                    SizedBox(
                      width: double.infinity,
                      child: ElevatedButton.icon(
                        onPressed: _connectWiFi,
                        icon: const Icon(Icons.link),
                        label: const Text('Connect via WiFi'),
                        style: ElevatedButton.styleFrom(backgroundColor: AppColors.primary),
                      ),
                    ),
                  ] else ...[
                    SizedBox(
                      width: double.infinity,
                      child: ElevatedButton.icon(
                        onPressed: _scanBle,
                        icon: const Icon(Icons.bluetooth_searching),
                        label: const Text('Scan for ADR-1 Robot'),
                        style: ElevatedButton.styleFrom(backgroundColor: AppColors.info),
                      ),
                    ),
                  ],
                  const SizedBox(height: 8),
                  BlocBuilder<ConnectionBloc, ConnectionState>(
                    builder: (context, state) {
                      return switch (state) {
                        Connected(:final type, :final target) =>
                          _StatusRow(Icons.check_circle, 'Connected ($type)', target, AppColors.success),
                        Connecting(:final target) =>
                          _StatusRow(Icons.hourglass_top, 'Connecting...', target, AppColors.warning),
                        ConnectionFailed(:final message) =>
                          _StatusRow(Icons.error, 'Failed', message, AppColors.error),
                        Reconnecting(:final attempt, :final maxAttempts) =>
                          _StatusRow(Icons.refresh, 'Reconnecting $attempt/$maxAttempts', '', AppColors.warning),
                        _ =>
                          _StatusRow(Icons.link_off, 'Disconnected', '', AppColors.textSecondary),
                      };
                    },
                  ),
                ],
              ),
            ),
          ),
          const SizedBox(height: 16),

          // Robot Settings
          Text('Robot Configuration', style: Theme.of(context).textTheme.titleMedium),
          const SizedBox(height: 8),
          Card(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                children: [
                  _SettingsTile(
                    'Max Speed',
                    '${LocalStorage.getMaxSpeed().toStringAsFixed(1)} m/s',
                    Icons.speed,
                    trailing: SizedBox(
                      width: 200,
                      child: Slider(
                        value: LocalStorage.getMaxSpeed(),
                        min: 0.1,
                        max: 2.0,
                        divisions: 19,
                        onChanged: (v) {
                          LocalStorage.setMaxSpeed(v);
                          setState(() {});
                        },
                      ),
                    ),
                  ),
                  const Divider(),
                  _SettingsTile(
                    'Telemetry Rate',
                    '${LocalStorage.getTelemetryRateHz()} Hz',
                    Icons.show_chart,
                    trailing: DropdownButton<int>(
                      value: LocalStorage.getTelemetryRateHz(),
                      items: [1, 5, 10, 20].map((hz) =>
                        DropdownMenuItem(value: hz, child: Text('$hz Hz')),
                      ).toList(),
                      onChanged: (v) {
                        if (v != null) {
                          LocalStorage.setTelemetryRateHz(v);
                          setState(() {});
                        }
                      },
                    ),
                  ),
                  const Divider(),
                  SwitchListTile(
                    title: const Text('Haptic Feedback'),
                    subtitle: const Text('Vibrate on events'),
                    secondary: const Icon(Icons.vibration),
                    value: LocalStorage.getVibrationEnabled(),
                    onChanged: (v) {
                      LocalStorage.setVibrationEnabled(v);
                      setState(() {});
                    },
                  ),
                ],
              ),
            ),
          ),
          const SizedBox(height: 16),

          // About
          Text('About', style: Theme.of(context).textTheme.titleMedium),
          const SizedBox(height: 8),
          const Card(
            child: Padding(
              padding: EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text('ADR-1 Controller v1.0.0', style: TextStyle(fontWeight: FontWeight.bold)),
                  SizedBox(height: 4),
                  Text('Autonomous Deseeder Robot Controller', style: TextStyle(color: AppColors.textSecondary)),
                  SizedBox(height: 8),
                  Text('All ICs: Industrial Grade (-40°C to +85°C)', style: TextStyle(fontSize: 12, color: AppColors.textSecondary)),
                  Text('Protocol: ADR-1 Binary v1 over WiFi/BLE', style: TextStyle(fontSize: 12, color: AppColors.textSecondary)),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }

  void _connectWiFi() {
    final addr = _ipController.text.trim();
    if (addr.isEmpty) return;
    LocalStorage.setLastRobotAddress(addr);
    context.read<ConnectionBloc>().add(ConnectWiFi(addr));
  }

  void _scanBle() {
    // BLE scan would use flutter_blue_plus to discover ADR-1 devices
    ScaffoldMessenger.of(context).showSnackBar(
      const SnackBar(content: Text('Scanning for ADR-1 robots via BLE...')),
    );
  }

  @override
  void dispose() {
    _ipController.dispose();
    super.dispose();
  }
}

class _StatusRow extends StatelessWidget {
  final IconData icon;
  final String title;
  final String subtitle;
  final Color color;
  const _StatusRow(this.icon, this.title, this.subtitle, this.color);

  @override
  Widget build(BuildContext context) {
    return Row(
      children: [
        Icon(icon, color: color, size: 20),
        const SizedBox(width: 8),
        Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(title, style: TextStyle(color: color, fontSize: 13)),
            if (subtitle.isNotEmpty)
              Text(subtitle, style: const TextStyle(fontSize: 11, color: AppColors.textSecondary)),
          ],
        ),
      ],
    );
  }
}

class _SettingsTile extends StatelessWidget {
  final String title;
  final String subtitle;
  final IconData icon;
  final Widget? trailing;
  const _SettingsTile(this.title, this.subtitle, this.icon, {this.trailing});

  @override
  Widget build(BuildContext context) {
    return ListTile(
      leading: Icon(icon),
      title: Text(title),
      subtitle: Text(subtitle),
      trailing: trailing,
      contentPadding: EdgeInsets.zero,
    );
  }
}
