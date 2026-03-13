import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:logger/logger.dart';
import 'package:web_socket_channel/web_socket_channel.dart';

import '../../core/constants/protocol.dart';

/// Abstraction over WiFi (WebSocket) and BLE connections to the robot
abstract class RobotConnection {
  Stream<Uint8List> get dataStream;
  bool get isConnected;
  Future<void> connect();
  Future<void> disconnect();
  Future<void> send(Uint8List data);
  void dispose();
}

/// WiFi connection via WebSocket to ESP32-S3 AP
class WiFiConnection implements RobotConnection {
  final String address;
  final int port;
  final _logger = Logger();

  WebSocketChannel? _channel;
  final _dataController = StreamController<Uint8List>.broadcast();
  bool _connected = false;

  WiFiConnection({required this.address, this.port = kWebSocketPort});

  @override
  Stream<Uint8List> get dataStream => _dataController.stream;

  @override
  bool get isConnected => _connected;

  @override
  Future<void> connect() async {
    try {
      final uri = Uri.parse('ws://$address:$port/ws');
      _channel = WebSocketChannel.connect(uri);
      await _channel!.ready;
      _connected = true;
      _logger.i('WiFi connected to $address:$port');

      _channel!.stream.listen(
        (data) {
          if (data is List<int>) {
            _dataController.add(Uint8List.fromList(data));
          } else if (data is String) {
            _dataController.add(Uint8List.fromList(utf8.encode(data)));
          }
        },
        onError: (error) {
          _logger.e('WiFi stream error: $error');
          _connected = false;
        },
        onDone: () {
          _logger.w('WiFi connection closed');
          _connected = false;
        },
      );
    } catch (e) {
      _logger.e('WiFi connection failed: $e');
      _connected = false;
      rethrow;
    }
  }

  @override
  Future<void> disconnect() async {
    await _channel?.sink.close();
    _connected = false;
  }

  @override
  Future<void> send(Uint8List data) async {
    if (!_connected || _channel == null) return;
    _channel!.sink.add(data);
  }

  @override
  void dispose() {
    disconnect();
    _dataController.close();
  }
}

/// BLE connection to ESP32-S3
class BleConnection implements RobotConnection {
  final String deviceId;
  final _logger = Logger();

  BluetoothDevice? _device;
  BluetoothCharacteristic? _commandChar;
  BluetoothCharacteristic? _telemetryChar;
  final _dataController = StreamController<Uint8List>.broadcast();
  bool _connected = false;
  StreamSubscription? _notifySub;

  BleConnection({required this.deviceId});

  @override
  Stream<Uint8List> get dataStream => _dataController.stream;

  @override
  bool get isConnected => _connected;

  @override
  Future<void> connect() async {
    try {
      _device = BluetoothDevice.fromId(deviceId);
      await _device!.connect(timeout: const Duration(seconds: 10));

      final services = await _device!.discoverServices();
      final adrService = services.firstWhere(
        (s) => s.uuid.toString() == kBleServiceUuid,
        orElse: () => throw Exception('ADR-1 BLE service not found'),
      );

      _commandChar = adrService.characteristics.firstWhere(
        (c) => c.uuid.toString() == kBleCharCommand,
      );
      _telemetryChar = adrService.characteristics.firstWhere(
        (c) => c.uuid.toString() == kBleCharTelemetry,
      );

      // Subscribe to telemetry notifications
      await _telemetryChar!.setNotifyValue(true);
      _notifySub = _telemetryChar!.onValueReceived.listen((value) {
        _dataController.add(Uint8List.fromList(value));
      });

      _connected = true;
      _logger.i('BLE connected to $deviceId');
    } catch (e) {
      _logger.e('BLE connection failed: $e');
      _connected = false;
      rethrow;
    }
  }

  @override
  Future<void> disconnect() async {
    await _notifySub?.cancel();
    await _device?.disconnect();
    _connected = false;
  }

  @override
  Future<void> send(Uint8List data) async {
    if (!_connected || _commandChar == null) return;
    // BLE MTU is typically 20 bytes; chunk if needed
    if (data.length <= 20) {
      await _commandChar!.write(data, withoutResponse: true);
    } else {
      for (int i = 0; i < data.length; i += 20) {
        final end = (i + 20 < data.length) ? i + 20 : data.length;
        await _commandChar!.write(data.sublist(i, end), withoutResponse: true);
      }
    }
  }

  @override
  void dispose() {
    disconnect();
    _dataController.close();
  }
}
