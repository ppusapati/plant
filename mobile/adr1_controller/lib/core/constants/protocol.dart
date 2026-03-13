/// ADR-1 Communication Protocol Constants
///
/// Binary protocol for WiFi (WebSocket) and BLE communication
/// between mobile app and ESP32-S3 on the robot.
library;

/// Packet header magic bytes
const int kPacketMagic = 0xAD01;

/// Protocol version
const int kProtocolVersion = 1;

/// Maximum packet payload size
const int kMaxPayloadSize = 512;

/// Default WebSocket port on ESP32-S3
const int kWebSocketPort = 8080;

/// BLE Service UUID for ADR-1 robot
const String kBleServiceUuid = '4adr0001-1234-5678-abcd-ef0123456789';

/// BLE Characteristic UUIDs
const String kBleCharCommand = '4adr0002-1234-5678-abcd-ef0123456789';
const String kBleCharTelemetry = '4adr0003-1234-5678-abcd-ef0123456789';
const String kBleCharSensor = '4adr0004-1234-5678-abcd-ef0123456789';
const String kBleCharMission = '4adr0005-1234-5678-abcd-ef0123456789';

/// Command IDs (Mobile → Robot)
enum CommandId {
  ping(0x01),
  setMode(0x10),
  manualControl(0x11),
  emergencyStop(0x1F),
  setSpeed(0x20),
  startMission(0x30),
  pauseMission(0x31),
  resumeMission(0x32),
  abortMission(0x33),
  addWaypoint(0x34),
  clearWaypoints(0x35),
  returnToHome(0x36),
  requestTelemetry(0x40),
  requestSensorData(0x41),
  requestHealthReport(0x42),
  setSensorInterval(0x50),
  calibrateIMU(0x60),
  calibrateGNSS(0x61),
  deployProbes(0x70),
  retractProbes(0x71),
  startDeseeding(0x80),
  stopDeseeding(0x81);

  const CommandId(this.value);
  final int value;
}

/// Telemetry IDs (Robot → Mobile)
enum TelemetryId {
  heartbeat(0x01),
  position(0x10),
  velocity(0x11),
  attitude(0x12),
  batteryStatus(0x20),
  motorStatus(0x21),
  sensorHealth(0x30),
  obstacleMap(0x31),
  missionProgress(0x40),
  ndviData(0x50),
  soilData(0x51),
  environmentData(0x52),
  healthScore(0x53),
  errorReport(0xF0),
  ack(0xFE),
  nack(0xFF);

  const TelemetryId(this.value);
  final int value;
}

/// Robot operating modes
enum RobotMode {
  idle(0, 'Idle', '⏸'),
  manual(1, 'Manual', '🕹'),
  autonomous(2, 'Autonomous', '🤖'),
  deseeding(3, 'Deseeding', '🌿'),
  returnHome(4, 'Return Home', '🏠'),
  estop(5, 'E-STOP', '🛑'),
  charging(6, 'Charging', '🔋'),
  calibrating(7, 'Calibrating', '🔧');

  const RobotMode(this.value, this.label, this.icon);
  final int value;
  final String label;
  final String icon;
}

/// Connection types
enum ConnectionType { wifi, ble, none }
