import 'dart:typed_data';
import 'package:flutter_test/flutter_test.dart';
import 'package:adr1_controller/data/datasources/packet_codec.dart';
import 'package:adr1_controller/core/constants/protocol.dart';

void main() {
  group('PacketCodec', () {
    test('encodes e-stop command with correct header', () {
      final packet = PacketCodec.encodeEStop();

      expect(packet[0], 0xAD);
      expect(packet[1], 0x01);
      expect(packet[2], kProtocolVersion);
      expect(packet[3], CommandId.emergencyStop.value);
      expect(packet.length, 7); // header(6) + crc(1), no payload
    });

    test('encodes manual control with throttle and steering', () {
      final packet = PacketCodec.encodeManualControl(0.5, -0.3);

      expect(packet[0], 0xAD);
      expect(packet[3], CommandId.manualControl.value);
      // payload length = 8 (two float32)
      expect((packet[4] << 8) | packet[5], 8);
    });

    test('clamps manual control values', () {
      final packet = PacketCodec.encodeManualControl(2.0, -5.0);

      // Extract float32 values from payload
      final bd = ByteData.sublistView(packet);
      final throttle = bd.getFloat32(6, Endian.little);
      final steering = bd.getFloat32(10, Endian.little);

      expect(throttle, 1.0);  // clamped from 2.0
      expect(steering, -1.0); // clamped from -5.0
    });

    test('encodes mode change', () {
      final packet = PacketCodec.encodeSetMode(RobotMode.autonomous);

      expect(packet[3], CommandId.setMode.value);
      expect(packet[6], RobotMode.autonomous.value);
    });

    test('round-trips a telemetry packet', () {
      // Create a fake telemetry packet
      final payload = Uint8List.fromList([0x01, 0x02, 0x03]);
      final packet = ByteData(7 + payload.length);
      packet.setUint8(0, 0xAD);
      packet.setUint8(1, 0x01);
      packet.setUint8(2, kProtocolVersion);
      packet.setUint8(3, TelemetryId.heartbeat.value);
      packet.setUint16(4, payload.length, Endian.big);
      for (int i = 0; i < payload.length; i++) {
        packet.setUint8(6 + i, payload[i]);
      }
      // Compute CRC
      final bytes = packet.buffer.asUint8List();
      int crc = 0;
      for (int i = 0; i < 6 + payload.length; i++) {
        crc ^= bytes[i];
        for (int j = 0; j < 8; j++) {
          if ((crc & 0x80) != 0) {
            crc = ((crc << 1) ^ 0x07) & 0xFF;
          } else {
            crc = (crc << 1) & 0xFF;
          }
        }
      }
      packet.setUint8(6 + payload.length, crc);

      final decoded = PacketCodec.decodeTelemetry(bytes);
      expect(decoded, isNotNull);
      expect(decoded!.id, TelemetryId.heartbeat);
      expect(decoded.payload, payload);
    });

    test('rejects invalid magic bytes', () {
      final bad = Uint8List.fromList([0xFF, 0xFF, 0x01, 0x01, 0x00, 0x00, 0x00]);
      expect(PacketCodec.decodeTelemetry(bad), isNull);
    });

    test('rejects too-short packets', () {
      final bad = Uint8List.fromList([0xAD, 0x01, 0x01]);
      expect(PacketCodec.decodeTelemetry(bad), isNull);
    });
  });
}
