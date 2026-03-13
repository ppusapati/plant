import 'dart:async';
import 'dart:typed_data';

import 'package:logger/logger.dart';

import '../../core/constants/protocol.dart';
import '../../domain/repositories/robot_repository.dart';
import '../datasources/robot_connection.dart';
import '../datasources/packet_codec.dart';
import '../models/telemetry.dart';
import '../models/sensor_data.dart';

class RobotRepositoryImpl implements RobotRepository {
  final _logger = Logger();

  RobotConnection? _connection;
  StreamSubscription? _dataSub;

  final _telemetryController = StreamController<RobotTelemetry>.broadcast();
  final _sensorController = StreamController<SensorUpdate>.broadcast();
  RobotTelemetry _currentTelemetry = RobotTelemetry.empty();

  @override
  Stream<RobotTelemetry> get telemetryStream => _telemetryController.stream;

  @override
  Stream<SensorUpdate> get sensorStream => _sensorController.stream;

  @override
  RobotTelemetry get currentTelemetry => _currentTelemetry;

  @override
  bool get isConnected => _connection?.isConnected ?? false;

  @override
  Future<void> connectWiFi(String address, {int port = kWebSocketPort}) async {
    await disconnect();
    _connection = WiFiConnection(address: address, port: port);
    await _connection!.connect();
    _startListening();
  }

  @override
  Future<void> connectBle(String deviceId) async {
    await disconnect();
    _connection = BleConnection(deviceId: deviceId);
    await _connection!.connect();
    _startListening();
  }

  @override
  Future<void> disconnect() async {
    await _dataSub?.cancel();
    _connection?.dispose();
    _connection = null;
    _currentTelemetry = _currentTelemetry.copyWith(connected: false);
    _telemetryController.add(_currentTelemetry);
  }

  void _startListening() {
    _currentTelemetry = _currentTelemetry.copyWith(connected: true);
    _telemetryController.add(_currentTelemetry);

    _dataSub = _connection!.dataStream.listen(
      _handlePacket,
      onError: (e) {
        _logger.e('Data stream error: $e');
        _currentTelemetry = _currentTelemetry.copyWith(connected: false);
        _telemetryController.add(_currentTelemetry);
      },
    );
  }

  void _handlePacket(Uint8List data) {
    final packet = PacketCodec.decodeTelemetry(data);
    if (packet == null) return;

    switch (packet.id) {
      case TelemetryId.position:
        _currentTelemetry = _currentTelemetry.copyWith(
          timestamp: DateTime.now(),
          position: PacketCodec.parsePosition(packet.payload),
        );
        _telemetryController.add(_currentTelemetry);

      case TelemetryId.batteryStatus:
        _currentTelemetry = _currentTelemetry.copyWith(
          timestamp: DateTime.now(),
          battery: PacketCodec.parseBattery(packet.payload),
        );
        _telemetryController.add(_currentTelemetry);

      case TelemetryId.environmentData:
        final env = PacketCodec.parseEnvironment(packet.payload);
        _sensorController.add(SensorUpdate.environment(env));

      case TelemetryId.heartbeat:
        if (packet.payload.isNotEmpty) {
          _currentTelemetry = _currentTelemetry.copyWith(
            mode: packet.payload[0],
            rssi: packet.payload.length > 1
                ? (packet.payload[1] as int) - 256 // signed
                : 0,
          );
          _telemetryController.add(_currentTelemetry);
        }

      default:
        _logger.d('Unhandled telemetry: ${packet.id}');
    }
  }

  @override
  Future<void> sendManualControl(double throttle, double steering) async {
    final data = PacketCodec.encodeManualControl(throttle, steering);
    await _connection?.send(data);
  }

  @override
  Future<void> sendEmergencyStop() async {
    final data = PacketCodec.encodeEStop();
    await _connection?.send(data);
    _logger.w('EMERGENCY STOP sent');
  }

  @override
  Future<void> setMode(RobotMode mode) async {
    final data = PacketCodec.encodeSetMode(mode);
    await _connection?.send(data);
  }

  @override
  Future<void> sendCommand(CommandId cmd, [Uint8List? payload]) async {
    final data = PacketCodec.encodeCommand(cmd, payload);
    await _connection?.send(data);
  }

  @override
  void dispose() {
    disconnect();
    _telemetryController.close();
    _sensorController.close();
  }
}

/// Tagged union for sensor data updates
sealed class SensorUpdate {
  const SensorUpdate();
  factory SensorUpdate.ndvi(NdviData data) = NdviUpdate;
  factory SensorUpdate.soil(SoilData data) = SoilUpdate;
  factory SensorUpdate.environment(EnvironmentData data) = EnvironmentUpdate;
}

class NdviUpdate extends SensorUpdate {
  final NdviData data;
  const NdviUpdate(this.data);
}

class SoilUpdate extends SensorUpdate {
  final SoilData data;
  const SoilUpdate(this.data);
}

class EnvironmentUpdate extends SensorUpdate {
  final EnvironmentData data;
  const EnvironmentUpdate(this.data);
}
