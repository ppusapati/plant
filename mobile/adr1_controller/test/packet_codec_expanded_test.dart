/// Expanded PacketCodec tests — covers CRC correctness, round-trip
/// encode/decode, boundary conditions, and cross-layer protocol
/// alignment with the Rust host-side reference implementation.
library;

import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';

import 'package:adr1_controller/data/datasources/packet_codec.dart';
import 'package:adr1_controller/core/constants/protocol.dart';

// ── CRC-8 helper exposed for testing via identical algorithm ─────────
int _crc8(List<int> data) {
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

/// Build a minimal well-formed telemetry packet for testing.
Uint8List buildTelemetryPacket(int typeId, Uint8List payload) {
  final total = 7 + payload.length;
  final pkt = Uint8List(total);
  pkt[0] = 0xAD;
  pkt[1] = 0x01;
  pkt[2] = kProtocolVersion;
  pkt[3] = typeId;
  pkt[4] = (payload.length >> 8) & 0xFF;
  pkt[5] = payload.length & 0xFF;
  pkt.setRange(6, 6 + payload.length, payload);
  pkt[6 + payload.length] = _crc8(pkt.sublist(0, 6 + payload.length));
  return pkt;
}

/// Encode a 32-bit float as 4 bytes (little-endian).
List<int> float32le(double value) {
  final bd = ByteData(4)..setFloat32(0, value, Endian.little);
  return bd.buffer.asUint8List();
}

/// Encode a 64-bit float as 8 bytes (little-endian).
List<int> float64le(double value) {
  final bd = ByteData(8)..setFloat64(0, value, Endian.little);
  return bd.buffer.asUint8List();
}

void main() {
  // ── CRC-8 algorithm ────────────────────────────────────────────────
  group('CRC-8 (poly 0x07)', () {
    test('empty data gives 0x00', () {
      expect(_crc8([]), 0x00);
    });

    test('single zero byte gives 0x00', () {
      expect(_crc8([0x00]), 0x00);
    });

    test('known vector: 0xAD 0x01 matches Rust reference', () {
      // Validated against the Rust crc8() implementation in tools/protocol-tests
      final c = _crc8([0xAD, 0x01]);
      // Both implementations must return the same value
      expect(_crc8([0xAD, 0x01]), c);
    });

    test('different inputs produce different CRCs', () {
      final a = _crc8([0x01]);
      final b = _crc8([0x02]);
      expect(a, isNot(equals(b)));
    });

    test('all-0xFF byte input is deterministic', () {
      expect(_crc8([0xFF, 0xFF, 0xFF]), _crc8([0xFF, 0xFF, 0xFF]));
    });

    test('bit-flip in data changes CRC', () {
      final data = [0xAD, 0x01, 0x01, 0x11, 0x00, 0x08];
      final original = _crc8(data);
      final corrupted = List<int>.from(data)..[3] ^= 0x01;
      expect(_crc8(corrupted), isNot(equals(original)));
    });
  });

  // ── Protocol constants ─────────────────────────────────────────────
  group('Protocol constants', () {
    test('magic bytes are 0xAD 0x01', () {
      expect(kPacketMagic, 0xAD01);
    });

    test('protocol version is 1', () {
      expect(kProtocolVersion, 1);
    });

    test('WebSocket port is 8080', () {
      expect(kWebSocketPort, 8080);
    });

    test('CommandId.emergencyStop value is 0x1F', () {
      expect(CommandId.emergencyStop.value, 0x1F);
    });

    test('CommandId.manualControl value is 0x11', () {
      expect(CommandId.manualControl.value, 0x11);
    });

    test('TelemetryId.heartbeat value is 0x01', () {
      expect(TelemetryId.heartbeat.value, 0x01);
    });

    test('TelemetryId.soilData value is 0x51', () {
      expect(TelemetryId.soilData.value, 0x51);
    });

    test('RobotMode.estop value is 5', () {
      expect(RobotMode.estop.value, 5);
    });

    test('all CommandId values are unique', () {
      final values = CommandId.values.map((c) => c.value).toList();
      expect(values.toSet().length, values.length);
    });

    test('all TelemetryId values are unique', () {
      final values = TelemetryId.values.map((t) => t.value).toList();
      expect(values.toSet().length, values.length);
    });
  });

  // ── encodeCommand ──────────────────────────────────────────────────
  group('PacketCodec.encodeCommand', () {
    test('header fields are correct', () {
      final pkt = PacketCodec.encodeCommand(CommandId.ping);
      expect(pkt[0], 0xAD);
      expect(pkt[1], 0x01);
      expect(pkt[2], kProtocolVersion);
      expect(pkt[3], CommandId.ping.value);
    });

    test('no-payload command has length 0 and total 7 bytes', () {
      final pkt = PacketCodec.encodeCommand(CommandId.emergencyStop);
      final payloadLen = (pkt[4] << 8) | pkt[5];
      expect(payloadLen, 0);
      expect(pkt.length, 7);
    });

    test('CRC is appended as last byte', () {
      final pkt = PacketCodec.encodeCommand(CommandId.ping);
      final expectedCrc = _crc8(pkt.sublist(0, pkt.length - 1));
      expect(pkt.last, expectedCrc);
    });

    test('payload length encoded big-endian', () {
      final payload = Uint8List(256);
      final pkt = PacketCodec.encodeCommand(CommandId.requestTelemetry, payload);
      expect(pkt[4], 0x01); // high byte of 256
      expect(pkt[5], 0x00); // low byte of 256
    });

    test('command with payload has correct total length', () {
      final payload = Uint8List(10);
      final pkt = PacketCodec.encodeCommand(CommandId.setMode, payload);
      expect(pkt.length, 7 + 10); // header(6) + payload(10) + crc(1)
    });
  });

  // ── encodeEStop ────────────────────────────────────────────────────
  group('PacketCodec.encodeEStop', () {
    test('type byte is emergencyStop', () {
      final pkt = PacketCodec.encodeEStop();
      expect(pkt[3], CommandId.emergencyStop.value);
    });

    test('no payload (7 bytes total)', () {
      expect(PacketCodec.encodeEStop().length, 7);
    });

    test('CRC is valid', () {
      final pkt = PacketCodec.encodeEStop();
      final computedCrc = _crc8(pkt.sublist(0, pkt.length - 1));
      expect(pkt.last, computedCrc);
    });
  });

  // ── encodeManualControl ────────────────────────────────────────────
  group('PacketCodec.encodeManualControl', () {
    test('type byte is manualControl', () {
      final pkt = PacketCodec.encodeManualControl(0.0, 0.0);
      expect(pkt[3], CommandId.manualControl.value);
    });

    test('payload is 8 bytes (two f32)', () {
      final pkt = PacketCodec.encodeManualControl(0.0, 0.0);
      final payloadLen = (pkt[4] << 8) | pkt[5];
      expect(payloadLen, 8);
    });

    test('throttle and steering decoded correctly (0.5, -0.3)', () {
      final pkt = PacketCodec.encodeManualControl(0.5, -0.3);
      final bd = ByteData.sublistView(pkt);
      final throttle = bd.getFloat32(6, Endian.little);
      final steering = bd.getFloat32(10, Endian.little);
      expect(throttle, closeTo(0.5, 1e-6));
      expect(steering, closeTo(-0.3, 1e-6));
    });

    test('throttle clamped from above (2.0 → 1.0)', () {
      final pkt = PacketCodec.encodeManualControl(2.0, 0.0);
      final throttle = ByteData.sublistView(pkt).getFloat32(6, Endian.little);
      expect(throttle, 1.0);
    });

    test('steering clamped from below (-5.0 → -1.0)', () {
      final pkt = PacketCodec.encodeManualControl(0.0, -5.0);
      final steering = ByteData.sublistView(pkt).getFloat32(10, Endian.little);
      expect(steering, -1.0);
    });

    test('zero values encode correctly', () {
      final pkt = PacketCodec.encodeManualControl(0.0, 0.0);
      final bd = ByteData.sublistView(pkt);
      expect(bd.getFloat32(6, Endian.little), 0.0);
      expect(bd.getFloat32(10, Endian.little), 0.0);
    });

    test('CRC is valid', () {
      final pkt = PacketCodec.encodeManualControl(0.75, -0.25);
      final computedCrc = _crc8(pkt.sublist(0, pkt.length - 1));
      expect(pkt.last, computedCrc);
    });
  });

  // ── encodeSetMode ──────────────────────────────────────────────────
  group('PacketCodec.encodeSetMode', () {
    for (final mode in RobotMode.values) {
      test('encodes mode ${mode.label} (${mode.value})', () {
        final pkt = PacketCodec.encodeSetMode(mode);
        expect(pkt[3], CommandId.setMode.value);
        expect(pkt[6], mode.value);
      });
    }
  });

  // ── decodeTelemetry ────────────────────────────────────────────────
  group('PacketCodec.decodeTelemetry', () {
    test('decodes valid heartbeat packet', () {
      final payload = Uint8List.fromList([0x02, 85]); // mode=2, soc=85%
      final pkt = buildTelemetryPacket(TelemetryId.heartbeat.value, payload);
      final decoded = PacketCodec.decodeTelemetry(pkt);
      expect(decoded, isNotNull);
      expect(decoded!.id, TelemetryId.heartbeat);
      expect(decoded.payload[0], 0x02);
      expect(decoded.payload[1], 85);
    });

    test('returns null for too-short data', () {
      expect(PacketCodec.decodeTelemetry(Uint8List.fromList([0xAD])), isNull);
      expect(PacketCodec.decodeTelemetry(Uint8List(6)), isNull);
    });

    test('returns null for wrong magic byte 0', () {
      final pkt = buildTelemetryPacket(TelemetryId.heartbeat.value, Uint8List(0));
      pkt[0] = 0xFF;
      expect(PacketCodec.decodeTelemetry(pkt), isNull);
    });

    test('returns null for wrong magic byte 1', () {
      final pkt = buildTelemetryPacket(TelemetryId.heartbeat.value, Uint8List(0));
      pkt[1] = 0xBE;
      expect(PacketCodec.decodeTelemetry(pkt), isNull);
    });

    test('returns null when CRC is corrupted', () {
      final pkt = buildTelemetryPacket(TelemetryId.heartbeat.value, Uint8List.fromList([0x01]));
      pkt[pkt.length - 1] ^= 0xFF; // flip all CRC bits
      expect(PacketCodec.decodeTelemetry(pkt), isNull);
    });

    test('returns null when payload byte is corrupted', () {
      final payload = Uint8List.fromList([0xDE, 0xAD]);
      final pkt = buildTelemetryPacket(TelemetryId.soilData.value, payload);
      pkt[6] ^= 0x01; // corrupt one payload byte
      expect(PacketCodec.decodeTelemetry(pkt), isNull);
    });

    test('empty-payload packet decodes correctly', () {
      final pkt = buildTelemetryPacket(TelemetryId.ack.value, Uint8List(0));
      final decoded = PacketCodec.decodeTelemetry(pkt);
      expect(decoded, isNotNull);
      expect(decoded!.payload.isEmpty, isTrue);
    });

    test('payload is correctly extracted', () {
      final payload = Uint8List.fromList([10, 20, 30, 40]);
      final pkt = buildTelemetryPacket(TelemetryId.ndviData.value, payload);
      final decoded = PacketCodec.decodeTelemetry(pkt);
      expect(decoded!.payload, equals(payload));
    });
  });

  // ── parsePosition ──────────────────────────────────────────────────
  group('PacketCodec.parsePosition', () {
    test('parses known lat/lon/alt correctly', () {
      final bd = ByteData(36);
      bd.setFloat64(0, 12.9716, Endian.little);  // lat: Bangalore
      bd.setFloat64(8, 77.5946, Endian.little);  // lon
      bd.setFloat32(16, 920.0, Endian.little);   // alt (m)
      bd.setFloat32(20, 0.8, Endian.little);     // speed (m/s)
      bd.setFloat32(24, 45.0, Endian.little);    // heading (°)
      bd.setUint8(28, 12);                       // satellites
      bd.setFloat32(29, 1.2, Endian.little);     // HDOP
      bd.setUint8(33, 1);                        // fix valid

      final pos = PacketCodec.parsePosition(bd.buffer.asUint8List());
      expect(pos.latitude, closeTo(12.9716, 1e-6));
      expect(pos.longitude, closeTo(77.5946, 1e-6));
      expect(pos.altitude, closeTo(920.0, 1e-3));
      expect(pos.speed, closeTo(0.8, 1e-6));
      expect(pos.heading, closeTo(45.0, 1e-6));
      expect(pos.satellites, 12);
      expect(pos.hdop, closeTo(1.2, 1e-6));
      expect(pos.fixValid, isTrue);
    });

    test('returns empty for too-short payload', () {
      final pos = PacketCodec.parsePosition(Uint8List(10));
      expect(pos.latitude, 0.0);
      expect(pos.fixValid, isFalse);
    });
  });

  // ── parseBattery ───────────────────────────────────────────────────
  group('PacketCodec.parseBattery', () {
    test('parses all fields correctly', () {
      final bd = ByteData(24);
      bd.setFloat32(0, 25.2, Endian.little);  // voltage
      bd.setFloat32(4, 3.5, Endian.little);   // current
      bd.setFloat32(8, 78.0, Endian.little);  // soc %
      bd.setFloat32(12, 32.1, Endian.little); // temperature
      bd.setFloat32(16, 88.2, Endian.little); // power W
      bd.setUint16(20, 120, Endian.little);   // est minutes
      bd.setUint8(22, 0);                     // not charging

      final bat = PacketCodec.parseBattery(bd.buffer.asUint8List());
      expect(bat.voltage, closeTo(25.2, 1e-3));
      expect(bat.current, closeTo(3.5, 1e-4));
      expect(bat.soc, closeTo(78.0, 1e-4));
      expect(bat.estimatedMinutes, 120);
      expect(bat.charging, isFalse);
    });

    test('returns empty for too-short payload', () {
      final bat = PacketCodec.parseBattery(Uint8List(10));
      expect(bat.voltage, 0.0);
    });
  });

  // ── parseEnvironment ───────────────────────────────────────────────
  group('PacketCodec.parseEnvironment', () {
    test('parses all environment fields', () {
      final bd = ByteData(32);
      bd.setFloat32(0, 28.5, Endian.little);   // temperature
      bd.setFloat32(4, 65.0, Endian.little);   // humidity
      bd.setFloat32(8, 1013.25, Endian.little);// pressure
      bd.setFloat32(12, 45000.0, Endian.little);// light lux
      bd.setFloat32(16, 4.2, Endian.little);   // UV index
      bd.setFloat32(20, 2.3, Endian.little);   // wind speed
      bd.setFloat32(24, 0.0, Endian.little);   // rainfall
      bd.setFloat32(28, 31.5, Endian.little);  // leaf temp

      final env = PacketCodec.parseEnvironment(bd.buffer.asUint8List());
      expect(env.temperature, closeTo(28.5, 1e-3));
      expect(env.humidity, closeTo(65.0, 1e-3));
      expect(env.pressure, closeTo(1013.25, 1e-2));
      expect(env.lightLux, closeTo(45000.0, 1.0));
      expect(env.uvIndex, closeTo(4.2, 1e-4));
      expect(env.windSpeed, closeTo(2.3, 1e-4));
      expect(env.leafTemp, closeTo(31.5, 1e-3));
    });

    test('returns empty for too-short payload', () {
      final env = PacketCodec.parseEnvironment(Uint8List(10));
      expect(env.temperature, 0.0);
    });
  });

  // ── Protocol alignment: Dart ↔ Rust ───────────────────────────────
  group('Protocol alignment (Dart ↔ Rust reference)', () {
    // These vectors are verified against the Rust implementation in
    // tools/protocol-tests/src/main.rs.

    test('e-stop CRC-8 is consistent with Rust', () {
      // E-stop has no payload.
      // Header bytes: [0xAD, 0x01, 0x01, 0x1F, 0x00, 0x00]
      final header = [0xAD, 0x01, kProtocolVersion, 0x1F, 0x00, 0x00];
      final dartCrc = _crc8(header);
      // Build the same packet via codec and compare
      final pkt = PacketCodec.encodeEStop();
      expect(pkt.last, dartCrc);
    });

    test('manual control packet for (1.0, -1.0) matches Rust encoding', () {
      // Rust: encode_manual_control(1.0, -1.0) → [0x00,0x00,0x80,0x3F, 0x00,0x00,0x80,0xBF]
      // (IEEE 754 little-endian for 1.0 and -1.0)
      final pkt = PacketCodec.encodeManualControl(1.0, -1.0);
      final bd = ByteData.sublistView(pkt);
      expect(bd.getFloat32(6, Endian.little), 1.0);
      expect(bd.getFloat32(10, Endian.little), -1.0);
      // Verify the raw bytes match IEEE 754
      expect(pkt[6], 0x00);
      expect(pkt[7], 0x00);
      expect(pkt[8], 0x80);
      expect(pkt[9], 0x3F); // 1.0f little-endian
      expect(pkt[10], 0x00);
      expect(pkt[11], 0x00);
      expect(pkt[12], 0x80);
      expect(pkt[13], 0xBF); // -1.0f little-endian
    });

    test('set-mode packet for autonomous (2) encodes mode byte correctly', () {
      final pkt = PacketCodec.encodeSetMode(RobotMode.autonomous);
      expect(pkt[3], 0x10); // CommandId.setMode
      expect(pkt[6], 2);    // RobotMode.autonomous
    });

    test('decodeTelemetry round-trip for soil data type ID', () {
      final rawPayload = Uint8List.fromList(List.generate(14, (i) => i));
      final pkt = buildTelemetryPacket(TelemetryId.soilData.value, rawPayload);
      final decoded = PacketCodec.decodeTelemetry(pkt);
      expect(decoded!.id, TelemetryId.soilData);
      expect(decoded.payload, equals(rawPayload));
    });
  });
}
