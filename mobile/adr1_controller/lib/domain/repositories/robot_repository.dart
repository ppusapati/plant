import 'dart:typed_data';

import '../../core/constants/protocol.dart';
import '../../data/models/telemetry.dart';
import '../../data/repositories/robot_repository_impl.dart';

/// Domain contract for robot communication
abstract class RobotRepository {
  Stream<RobotTelemetry> get telemetryStream;
  Stream<SensorUpdate> get sensorStream;
  RobotTelemetry get currentTelemetry;
  bool get isConnected;

  Future<void> connectWiFi(String address, {int port});
  Future<void> connectBle(String deviceId);
  Future<void> disconnect();

  Future<void> sendManualControl(double throttle, double steering);
  Future<void> sendEmergencyStop();
  Future<void> setMode(RobotMode mode);
  Future<void> sendCommand(CommandId cmd, [Uint8List? payload]);

  void dispose();
}
