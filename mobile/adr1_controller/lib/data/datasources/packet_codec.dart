import 'dart:typed_data';
import '../../core/constants/protocol.dart';
import '../models/telemetry.dart';
import '../models/sensor_data.dart';

/// Binary packet encoder/decoder for ADR-1 protocol
///
/// Packet format:
///   [0xAD][0x01][version][type][length_hi][length_lo][payload...][crc8]
class PacketCodec {
  /// Encode a command to send to the robot
  static Uint8List encodeCommand(CommandId cmd, [Uint8List? payload]) {
    final payloadLen = payload?.length ?? 0;
    final packet = ByteData(7 + payloadLen);

    packet.setUint8(0, 0xAD);
    packet.setUint8(1, 0x01);
    packet.setUint8(2, kProtocolVersion);
    packet.setUint8(3, cmd.value);
    packet.setUint16(4, payloadLen, Endian.big);

    if (payload != null) {
      for (int i = 0; i < payloadLen; i++) {
        packet.setUint8(6 + i, payload[i]);
      }
    }

    // CRC8 over all bytes except the CRC itself
    final bytes = packet.buffer.asUint8List();
    packet.setUint8(6 + payloadLen, _crc8(bytes.sublist(0, 6 + payloadLen)));

    return bytes;
  }

  /// Encode manual control command: throttle (-1.0 to 1.0), steering (-1.0 to 1.0)
  static Uint8List encodeManualControl(double throttle, double steering) {
    final payload = ByteData(8);
    payload.setFloat32(0, throttle.clamp(-1.0, 1.0), Endian.little);
    payload.setFloat32(4, steering.clamp(-1.0, 1.0), Endian.little);
    return encodeCommand(CommandId.manualControl, payload.buffer.asUint8List());
  }

  /// Encode emergency stop command
  static Uint8List encodeEStop() => encodeCommand(CommandId.emergencyStop);

  /// Encode mode change command
  static Uint8List encodeSetMode(RobotMode mode) {
    return encodeCommand(CommandId.setMode, Uint8List.fromList([mode.value]));
  }

  /// Decode incoming telemetry packet
  static TelemetryPacket? decodeTelemetry(Uint8List data) {
    if (data.length < 7) return null;
    if (data[0] != 0xAD || data[1] != 0x01) return null;

    final type = data[3];
    final payloadLen = (data[4] << 8) | data[5];
    if (data.length < 7 + payloadLen) return null;

    // Verify CRC
    final expectedCrc = _crc8(data.sublist(0, 6 + payloadLen));
    if (data[6 + payloadLen] != expectedCrc) return null;

    final payload = data.sublist(6, 6 + payloadLen);
    final telemetryId = TelemetryId.values.firstWhere(
      (t) => t.value == type,
      orElse: () => TelemetryId.heartbeat,
    );

    return TelemetryPacket(id: telemetryId, payload: payload);
  }

  /// Parse position telemetry payload
  static PositionData parsePosition(Uint8List payload) {
    if (payload.length < 36) return PositionData.empty();
    final bd = ByteData.sublistView(payload);
    return PositionData(
      latitude: bd.getFloat64(0, Endian.little),
      longitude: bd.getFloat64(8, Endian.little),
      altitude: bd.getFloat32(16, Endian.little),
      speed: bd.getFloat32(20, Endian.little),
      heading: bd.getFloat32(24, Endian.little),
      satellites: bd.getUint8(28),
      hdop: bd.getFloat32(29, Endian.little),
      fixValid: bd.getUint8(33) == 1,
    );
  }

  /// Parse battery telemetry payload
  static BatteryData parseBattery(Uint8List payload) {
    if (payload.length < 24) return BatteryData.empty();
    final bd = ByteData.sublistView(payload);
    return BatteryData(
      voltage: bd.getFloat32(0, Endian.little),
      current: bd.getFloat32(4, Endian.little),
      soc: bd.getFloat32(8, Endian.little),
      temperature: bd.getFloat32(12, Endian.little),
      powerWatts: bd.getFloat32(16, Endian.little),
      estimatedMinutes: bd.getUint16(20, Endian.little),
      charging: bd.getUint8(22) == 1,
    );
  }

  /// Parse environment telemetry payload
  static EnvironmentData parseEnvironment(Uint8List payload) {
    if (payload.length < 32) return EnvironmentData.empty();
    final bd = ByteData.sublistView(payload);
    return EnvironmentData(
      timestamp: DateTime.now(),
      temperature: bd.getFloat32(0, Endian.little),
      humidity: bd.getFloat32(4, Endian.little),
      pressure: bd.getFloat32(8, Endian.little),
      lightLux: bd.getFloat32(12, Endian.little),
      uvIndex: bd.getFloat32(16, Endian.little),
      windSpeed: bd.getFloat32(20, Endian.little),
      rainfall: bd.getFloat32(24, Endian.little),
      leafTemp: bd.getFloat32(28, Endian.little),
    );
  }

  /// CRC-8 (polynomial 0x07)
  static int _crc8(List<int> data) {
    int crc = 0x00;
    for (final byte in data) {
      crc ^= byte;
      for (int i = 0; i < 8; i++) {
        if ((crc & 0x80) != 0) {
          crc = ((crc << 1) ^ 0x07) & 0xFF;
        } else {
          crc = (crc << 1) & 0xFF;
        }
      }
    }
    return crc;
  }
}

/// Decoded telemetry packet
class TelemetryPacket {
  final TelemetryId id;
  final Uint8List payload;

  const TelemetryPacket({required this.id, required this.payload});
}
