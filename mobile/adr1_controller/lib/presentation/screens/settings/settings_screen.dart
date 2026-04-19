import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';

import '../../../core/constants/protocol.dart';
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

  Future<void> _scanBle() async {
    // Verify Bluetooth is available and on
    if (!await FlutterBluePlus.isAvailable) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('Bluetooth is not available on this device')),
        );
      }
      return;
    }

    if (await FlutterBluePlus.adapterState.first != BluetoothAdapterState.on) {
      await FlutterBluePlus.turnOn();
      try {
        await FlutterBluePlus.adapterState
            .where((s) => s == BluetoothAdapterState.on)
            .first
            .timeout(const Duration(seconds: 5));
      } on TimeoutException {
        if (mounted) {
          ScaffoldMessenger.of(context).showSnackBar(
            const SnackBar(content: Text('Please enable Bluetooth and try again')),
          );
        }
        return;
      }
    }

    if (!mounted) return;

    // Show scan sheet; it manages scan lifecycle internally
    await showModalBottomSheet<void>(
      context: context,
      isScrollControlled: true,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(16)),
      ),
      builder: (_) => _BleScanSheet(
        onDeviceSelected: (deviceId, deviceName) {
          context.read<ConnectionBloc>().add(ConnectBle(deviceId, deviceName));
        },
      ),
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

// ── BLE Scan Sheet ────────────────────────────────────────────────────

/// Bottom sheet that starts a BLE scan and lists discovered ADR-1 robots.
/// Filters by the ADR-1 service UUID so only matching devices are shown.
class _BleScanSheet extends StatefulWidget {
  /// Called when the user selects a device.  Arguments: deviceId, deviceName.
  final void Function(String deviceId, String deviceName) onDeviceSelected;

  const _BleScanSheet({required this.onDeviceSelected});

  @override
  State<_BleScanSheet> createState() => _BleScanSheetState();
}

class _BleScanSheetState extends State<_BleScanSheet> {
  StreamSubscription<List<ScanResult>>? _scanSub;
  StreamSubscription<bool>? _isScanSub;
  final List<ScanResult> _results = [];
  bool _scanning = false;

  static const _scanDuration = Duration(seconds: 15);

  @override
  void initState() {
    super.initState();
    _startScan();
  }

  Future<void> _startScan() async {
    setState(() {
      _scanning = true;
      _results.clear();
    });

    // Filter to ADR-1 service UUID so unrelated BLE devices are not shown
    await FlutterBluePlus.startScan(
      withServices: [Guid(kBleServiceUuid)],
      timeout: _scanDuration,
    );

    _scanSub = FlutterBluePlus.onScanResults.listen((results) {
      if (mounted) {
        setState(() {
          _results
            ..clear()
            ..addAll(results);
        });
      }
    });

    _isScanSub = FlutterBluePlus.isScanning.listen((scanning) {
      if (mounted) {
        setState(() => _scanning = scanning);
      }
    });
  }

  Future<void> _stopScan() async {
    await FlutterBluePlus.stopScan();
  }

  @override
  void dispose() {
    _scanSub?.cancel();
    _isScanSub?.cancel();
    FlutterBluePlus.stopScan();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      height: MediaQuery.of(context).size.height * 0.55,
      child: Column(
        children: [
          // Handle bar
          Padding(
            padding: const EdgeInsets.symmetric(vertical: 8),
            child: Container(
              width: 40,
              height: 4,
              decoration: BoxDecoration(
                color: AppColors.textSecondary.withOpacity(0.4),
                borderRadius: BorderRadius.circular(2),
              ),
            ),
          ),
          // Title row
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
            child: Row(
              children: [
                const Icon(Icons.bluetooth_searching, color: AppColors.info),
                const SizedBox(width: 8),
                const Text(
                  'ADR-1 Robots nearby',
                  style: TextStyle(fontSize: 17, fontWeight: FontWeight.bold),
                ),
                const Spacer(),
                if (_scanning)
                  const SizedBox(
                    width: 20,
                    height: 20,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  )
                else
                  IconButton(
                    icon: const Icon(Icons.refresh),
                    tooltip: 'Scan again',
                    onPressed: _startScan,
                  ),
                if (_scanning)
                  IconButton(
                    icon: const Icon(Icons.stop),
                    tooltip: 'Stop scanning',
                    onPressed: _stopScan,
                  ),
              ],
            ),
          ),
          const Divider(height: 1),
          // Device list
          Expanded(
            child: _results.isEmpty
                ? Center(
                    child: Padding(
                      padding: const EdgeInsets.all(24),
                      child: _scanning
                          ? Column(
                              mainAxisSize: MainAxisSize.min,
                              children: [
                                const CircularProgressIndicator(),
                                const SizedBox(height: 16),
                                Text(
                                  'Scanning for ADR-1 robots…',
                                  style: TextStyle(color: AppColors.textSecondary),
                                ),
                              ],
                            )
                          : Column(
                              mainAxisSize: MainAxisSize.min,
                              children: [
                                const Icon(Icons.bluetooth_disabled,
                                    size: 48, color: AppColors.textSecondary),
                                const SizedBox(height: 12),
                                const Text(
                                  'No ADR-1 robots found.',
                                  style: TextStyle(fontWeight: FontWeight.bold),
                                ),
                                const SizedBox(height: 4),
                                Text(
                                  'Make sure the robot is powered on\nand within BLE range (≤ 10 m).',
                                  textAlign: TextAlign.center,
                                  style: TextStyle(
                                      color: AppColors.textSecondary, fontSize: 13),
                                ),
                                const SizedBox(height: 16),
                                ElevatedButton.icon(
                                  onPressed: _startScan,
                                  icon: const Icon(Icons.refresh),
                                  label: const Text('Scan again'),
                                ),
                              ],
                            ),
                    ),
                  )
                : ListView.builder(
                    itemCount: _results.length,
                    itemBuilder: (_, i) {
                      final r = _results[i];
                      final name = r.device.platformName.isNotEmpty
                          ? r.device.platformName
                          : 'ADR-1 Robot';
                      final id = r.device.remoteId.str;
                      return ListTile(
                        leading: const CircleAvatar(
                          backgroundColor: AppColors.primary,
                          child: Icon(Icons.precision_manufacturing,
                              color: Colors.white, size: 20),
                        ),
                        title: Text(name),
                        subtitle: Text(id,
                            style:
                                const TextStyle(fontSize: 11, color: AppColors.textSecondary)),
                        trailing: Text(
                          '${r.rssi} dBm',
                          style: TextStyle(
                            color: r.rssi > -70 ? AppColors.success : AppColors.warning,
                            fontSize: 12,
                          ),
                        ),
                        onTap: () {
                          widget.onDeviceSelected(id, name);
                          Navigator.of(context).pop();
                        },
                      );
                    },
                  ),
          ),
        ],
      ),
    );
  }
}
