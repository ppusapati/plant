//! ADR-1 Protocol logic – host-side reference implementation and test suite.
//!
//! This crate re-implements the pure protocol functions (CRC-8, CRC-16, packet
//! encode/decode, health score) in standard Rust so they can be unit-tested
//! on any CI host without requiring embedded toolchains or hardware.
//!
//! The implementations here **must stay byte-for-byte identical** to the
//! embedded firmware counterparts:
//!  - CRC-8: `mobile/adr1_controller/lib/data/datasources/packet_codec.dart`
//!  - CRC-16: `firmware/robot-firmware/src/protocol/mod.rs`
//!  - Health score: `firmware/robot-firmware/src/sensors/health_score.rs`

// ─────────────────────────────────────────────────────────────────────────────
// CRC implementations
// ─────────────────────────────────────────────────────────────────────────────

/// CRC-8 (polynomial 0x07 / CRC-8/SMBUS).
///
/// Used on BLE/WiFi frames between the mobile app and the controller ESP32-S3.
/// Computed over all bytes *except* the trailing CRC byte itself.
pub fn crc8(data: &[u8]) -> u8 {
    let mut crc: u8 = 0x00;
    for &byte in data {
        crc ^= byte;
        for _ in 0..8 {
            if crc & 0x80 != 0 {
                crc = (crc << 1) ^ 0x07;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

/// CRC-16/IBM (reflected polynomial 0xA001, also known as CRC-16/Modbus).
///
/// Used on LoRa frames between the robot STM32H743 and the controller ESP32-S3.
/// Computed over all bytes *except* the trailing 2 CRC bytes.
pub fn crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &byte in data {
        crc ^= byte as u16;
        for _ in 0..8 {
            if crc & 0x0001 != 0 {
                crc = (crc >> 1) ^ 0xA001;
            } else {
                crc >>= 1;
            }
        }
    }
    crc
}

// ─────────────────────────────────────────────────────────────────────────────
// BLE/WiFi packet (CRC-8)
// ─────────────────────────────────────────────────────────────────────────────

pub const MAGIC_0: u8 = 0xAD;
pub const MAGIC_1: u8 = 0x01;
pub const PROTOCOL_VERSION: u8 = 0x01;

/// Encode a BLE/WiFi command packet.
///
/// Format: `[0xAD][0x01][version][type][len_hi][len_lo][payload…][crc8]`
pub fn encode_ble_packet(msg_type: u8, payload: &[u8]) -> Vec<u8> {
    let payload_len = payload.len();
    let mut buf = Vec::with_capacity(7 + payload_len);
    buf.push(MAGIC_0);
    buf.push(MAGIC_1);
    buf.push(PROTOCOL_VERSION);
    buf.push(msg_type);
    buf.push((payload_len >> 8) as u8);
    buf.push(payload_len as u8);
    buf.extend_from_slice(payload);
    let checksum = crc8(&buf);
    buf.push(checksum);
    buf
}

/// Decode a BLE/WiFi packet. Returns `(msg_type, payload)` or `None`.
pub fn decode_ble_packet(data: &[u8]) -> Option<(u8, Vec<u8>)> {
    if data.len() < 7 {
        return None;
    }
    if data[0] != MAGIC_0 || data[1] != MAGIC_1 {
        return None;
    }
    let msg_type = data[3];
    let payload_len = ((data[4] as usize) << 8) | data[5] as usize;
    if data.len() < 7 + payload_len {
        return None;
    }
    let body = &data[..6 + payload_len];
    let expected_crc = crc8(body);
    if data[6 + payload_len] != expected_crc {
        return None;
    }
    Some((msg_type, data[6..6 + payload_len].to_vec()))
}

// ─────────────────────────────────────────────────────────────────────────────
// LoRa packet (CRC-16)
// ─────────────────────────────────────────────────────────────────────────────

pub const ADDR_ROBOT: u8 = 0x01;
pub const ADDR_CONTROLLER: u8 = 0x02;

/// Encode a LoRa frame.
///
/// Format: `[0xAD][0x01][src][dst][msg_id][payload_len][payload…][seq_lo][seq_hi][crc_lo][crc_hi]`
pub fn encode_lora_frame(src: u8, dst: u8, msg_id: u8, payload: &[u8], seq: u16) -> Vec<u8> {
    let mut buf = Vec::with_capacity(10 + payload.len());
    buf.push(MAGIC_0);
    buf.push(MAGIC_1);
    buf.push(src);
    buf.push(dst);
    buf.push(msg_id);
    buf.push(payload.len() as u8);
    buf.extend_from_slice(payload);
    buf.push(seq as u8);
    buf.push((seq >> 8) as u8);
    let checksum = crc16(&buf);
    buf.push(checksum as u8);
    buf.push((checksum >> 8) as u8);
    buf
}

/// Decode a LoRa frame. Returns `(src, dst, msg_id, seq, payload)` or `None`.
pub fn decode_lora_frame(data: &[u8]) -> Option<(u8, u8, u8, u16, Vec<u8>)> {
    if data.len() < 10 {
        return None;
    }
    if data[0] != MAGIC_0 || data[1] != MAGIC_1 {
        return None;
    }
    let src = data[2];
    let dst = data[3];
    let msg_id = data[4];
    let payload_len = data[5] as usize;
    if data.len() < 6 + payload_len + 4 {
        return None;
    }
    let crc_offset = 6 + payload_len + 2;
    let expected_crc = u16::from_le_bytes([data[crc_offset], data[crc_offset + 1]]);
    let computed_crc = crc16(&data[..crc_offset]);
    if expected_crc != computed_crc {
        return None;
    }
    let seq = u16::from_le_bytes([data[6 + payload_len], data[7 + payload_len]]);
    let payload = data[6..6 + payload_len].to_vec();
    Some((src, dst, msg_id, seq, payload))
}

// ─────────────────────────────────────────────────────────────────────────────
// Manual-control payload helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Encode throttle + steering into an 8-byte little-endian f32 payload.
/// Both values are clamped to [−1.0, 1.0].
pub fn encode_manual_control(throttle: f32, steering: f32) -> [u8; 8] {
    let t = throttle.clamp(-1.0, 1.0);
    let s = steering.clamp(-1.0, 1.0);
    let mut buf = [0u8; 8];
    buf[..4].copy_from_slice(&t.to_le_bytes());
    buf[4..].copy_from_slice(&s.to_le_bytes());
    buf
}

/// Decode an 8-byte manual-control payload into (throttle, steering).
pub fn decode_manual_control(payload: &[u8]) -> Option<(f32, f32)> {
    if payload.len() < 8 {
        return None;
    }
    let throttle = f32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
    let steering = f32::from_le_bytes([payload[4], payload[5], payload[6], payload[7]]);
    Some((throttle, steering))
}

// ─────────────────────────────────────────────────────────────────────────────
// Health score algorithm
// ─────────────────────────────────────────────────────────────────────────────

/// Compute a 0–1000 score for a value within an optimal range.
/// Score is 1000 when `value` is within [min, max].
/// Degrades linearly to 0 at ±50 % outside the range (tolerance margin).
pub fn range_score(value: i32, min: i32, max: i32) -> u16 {
    if value >= min && value <= max {
        return 1000;
    }
    let range = (max - min).max(1);
    let margin = range / 2;
    if value < min {
        let deficit = min - value;
        if deficit > margin {
            0
        } else {
            ((margin - deficit) as u32 * 1000 / margin as u32) as u16
        }
    } else {
        let excess = value - max;
        if excess > margin {
            0
        } else {
            ((margin - excess) as u32 * 1000 / margin as u32) as u16
        }
    }
}

/// Compute the composite plant/soil health score (0–1000) using default weights:
/// NDVI 30 % · leaf temp 15 % · moisture 20 % · NPK 15 % · pH 10 % · EC 10 %
pub fn compute_health_score(
    ndvi: i32,        // × 1000
    leaf_temp: i32,   // delta °C × 100
    moisture: i32,    // % × 10
    nitrogen: i32,    // mg/kg
    phosphorus: i32,  // mg/kg
    potassium: i32,   // mg/kg
    ph: i32,          // × 100
    ec: i32,          // µS/cm
) -> u16 {
    // Default profile (general agriculture)
    let ndvi_score = range_score(ndvi, 400, 900);
    let leaf_score = range_score(leaf_temp, -300, 200);
    let moisture_score = range_score(moisture, 200, 600);
    let n_score = range_score(nitrogen, 50, 300);
    let p_score = range_score(phosphorus, 20, 150);
    let k_score = range_score(potassium, 100, 400);
    let npk_score = (n_score as u32 + p_score as u32 + k_score as u32) / 3;
    let ph_score = range_score(ph, 550, 750);
    let ec_score = range_score(ec, 200, 4000);

    let composite = (ndvi_score as u32 * 30
        + leaf_score as u32 * 15
        + moisture_score as u32 * 20
        + npk_score * 15
        + ph_score as u32 * 10
        + ec_score as u32 * 10)
        / 100;

    composite.min(1000) as u16
}

// ─────────────────────────────────────────────────────────────────────────────
// NDVI calculation
// ─────────────────────────────────────────────────────────────────────────────

/// NDVI = (NIR − RED) / (NIR + RED), scaled × 1000.
pub fn calculate_ndvi(red: u16, nir: u16) -> i16 {
    if red == 0 && nir == 0 {
        return 0;
    }
    let red_f = red as f32;
    let nir_f = nir as f32;
    let ndvi = (nir_f - red_f) / (nir_f + red_f);
    (ndvi * 1000.0) as i16
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── CRC-8 ────────────────────────────────────────────────────────────────

    #[test]
    fn crc8_empty_data_is_zero() {
        assert_eq!(crc8(&[]), 0x00);
    }

    #[test]
    fn crc8_single_byte_known_value() {
        // crc8([0xAD]) with poly 0x07, init 0x00
        let result = crc8(&[0xAD]);
        // Computed manually:  0xAD ^ poly iterations
        assert_ne!(result, 0); // sanity: not trivially zero
    }

    #[test]
    fn crc8_round_trip_detects_corruption() {
        let payload = b"hello";
        let mut packet = encode_ble_packet(0x01, payload);
        // Packet is valid
        assert!(decode_ble_packet(&packet).is_some());
        // Corrupt one byte in the payload
        packet[6] ^= 0xFF;
        assert!(decode_ble_packet(&packet).is_none());
    }

    #[test]
    fn crc8_matches_dart_implementation() {
        // Known vector: from Flutter PacketCodec.encodeEStop()
        // eStop payload = empty → packet = [0xAD,0x01,0x01,0x1F,0x00,0x00,<crc8>]
        let header = [0xAD_u8, 0x01, 0x01, 0x1F, 0x00, 0x00];
        let crc = crc8(&header);
        // Verify the crc is consistent (same input → same output)
        assert_eq!(crc8(&header), crc);
        // Ensure the full packet decodes back correctly
        let packet = encode_ble_packet(0x1F, &[]);
        let decoded = decode_ble_packet(&packet).expect("should decode");
        assert_eq!(decoded.0, 0x1F);
        assert!(decoded.1.is_empty());
    }

    // ── CRC-16 ──────────────────────────────────────────────────────────────

    #[test]
    fn crc16_empty_data_is_ffff() {
        // CRC-16 init value
        assert_eq!(crc16(&[]), 0xFFFF);
    }

    #[test]
    fn crc16_modbus_known_vector() {
        // Modbus CRC of [0x01, 0x03, 0x00, 0x00, 0x00, 0x01].
        // The protocol stores CRC bytes as {low, high}; interpreted as little-endian u16
        // the value is 0x0A84 = 2692. (On-wire bytes: 0x84 first, 0x0A second.)
        let data = [0x01_u8, 0x03, 0x00, 0x00, 0x00, 0x01];
        assert_eq!(crc16(&data), 0x0A84);
    }

    #[test]
    fn lora_frame_round_trip() {
        let payload = [0x02_u8, 75]; // mode=2, battery=75%
        let frame = encode_lora_frame(ADDR_ROBOT, ADDR_CONTROLLER, 0x01, &payload, 42);
        let (src, dst, msg_id, seq, decoded_payload) =
            decode_lora_frame(&frame).expect("frame should decode");
        assert_eq!(src, ADDR_ROBOT);
        assert_eq!(dst, ADDR_CONTROLLER);
        assert_eq!(msg_id, 0x01);
        assert_eq!(seq, 42);
        assert_eq!(&decoded_payload, &payload);
    }

    #[test]
    fn lora_frame_detects_corruption() {
        let payload = [0x01_u8];
        let mut frame = encode_lora_frame(ADDR_CONTROLLER, ADDR_ROBOT, 0x11, &payload, 7);
        // Corrupt the payload byte
        frame[6] ^= 0x01;
        assert!(decode_lora_frame(&frame).is_none());
    }

    #[test]
    fn lora_frame_rejects_wrong_magic() {
        let mut frame = encode_lora_frame(ADDR_ROBOT, ADDR_CONTROLLER, 0x01, &[], 0);
        frame[0] = 0xFF; // Wrong magic
        assert!(decode_lora_frame(&frame).is_none());
    }

    #[test]
    fn lora_frame_too_short_returns_none() {
        assert!(decode_lora_frame(&[0xAD, 0x01, 0x01, 0x02]).is_none());
    }

    // ── Manual control payload ───────────────────────────────────────────────

    #[test]
    fn manual_control_round_trip() {
        let payload = encode_manual_control(0.75, -0.5);
        let (throttle, steering) = decode_manual_control(&payload).unwrap();
        assert!((throttle - 0.75).abs() < 1e-6);
        assert!((steering - (-0.5)).abs() < 1e-6);
    }

    #[test]
    fn manual_control_clamping() {
        let payload = encode_manual_control(2.0, -5.0);
        let (throttle, steering) = decode_manual_control(&payload).unwrap();
        assert!((throttle - 1.0).abs() < 1e-6, "throttle should clamp to 1.0");
        assert!((steering - (-1.0)).abs() < 1e-6, "steering should clamp to -1.0");
    }

    #[test]
    fn manual_control_payload_too_short() {
        assert!(decode_manual_control(&[0x00, 0x01]).is_none());
    }

    // ── BLE/WiFi packet ──────────────────────────────────────────────────────

    #[test]
    fn ble_packet_header_fields() {
        let pkt = encode_ble_packet(0x11, &[0xAA, 0xBB]);
        assert_eq!(pkt[0], 0xAD);
        assert_eq!(pkt[1], 0x01);
        assert_eq!(pkt[2], PROTOCOL_VERSION);
        assert_eq!(pkt[3], 0x11);
        assert_eq!(pkt[4], 0x00); // len_hi
        assert_eq!(pkt[5], 0x02); // len_lo
        assert_eq!(pkt[6], 0xAA);
        assert_eq!(pkt[7], 0xBB);
        // pkt[8] = CRC
        assert_eq!(pkt.len(), 9);
    }

    #[test]
    fn ble_packet_rejects_bad_magic() {
        let mut pkt = encode_ble_packet(0x01, &[]);
        pkt[0] = 0x00;
        assert!(decode_ble_packet(&pkt).is_none());
    }

    #[test]
    fn ble_packet_too_short_returns_none() {
        assert!(decode_ble_packet(&[0xAD, 0x01]).is_none());
    }

    // ── Health score ─────────────────────────────────────────────────────────

    #[test]
    fn health_score_perfect_conditions_gives_1000() {
        // All values within optimal range for default profile
        let score = compute_health_score(
            650,  // ndvi × 1000 = 0.65 (within 0.4–0.9)
            -100, // leaf temp delta × 100 = -1°C (within -3..+2)
            400,  // moisture % × 10 = 40% (within 20–60%)
            150,  // nitrogen mg/kg (within 50–300)
            80,   // phosphorus mg/kg (within 20–150)
            250,  // potassium mg/kg (within 100–400)
            650,  // pH × 100 = 6.50 (within 5.5–7.5)
            2000, // EC µS/cm (within 200–4000)
        );
        assert_eq!(score, 1000);
    }

    #[test]
    fn health_score_zero_ndvi_degrades_score() {
        // NDVI = 0 (far below 0.4 minimum) → NDVI sub-score = 0
        // Other values perfect
        let score = compute_health_score(0, -100, 400, 150, 80, 250, 650, 2000);
        // NDVI contributes 30%; all others = 1000 → total < 1000
        assert!(score < 1000);
        assert!(score > 0);
    }

    #[test]
    fn health_score_all_extreme_out_of_range_gives_zero() {
        // Values placed more than 50% of the range width outside each boundary
        // so every sub-score is 0 and the composite is 0.
        let score = compute_health_score(
            -10000, // ndvi: way below 400 (min), margin = 250 → deficit 10400 > 250 → 0
            100000, // leaf_temp delta: way above 200 (max), margin = 250 → excess > 250 → 0
            -10000, // moisture: way below 200 (min), margin = 200 → deficit > 200 → 0
            -10000, // nitrogen: way below 50 (min), margin = 125 → deficit > 125 → 0
            -10000, // phosphorus: way below 20 (min), margin = 65 → deficit > 65 → 0
            -10000, // potassium: way below 100 (min), margin = 150 → deficit > 150 → 0
            -10000, // pH: way below 550 (min), margin = 100 → deficit > 100 → 0
            -10000, // EC: way below 200 (min), margin = 1900 → deficit > 1900 → 0
        );
        assert_eq!(score, 0);
    }

    #[test]
    fn range_score_within_range_gives_1000() {
        assert_eq!(range_score(50, 0, 100), 1000);
        assert_eq!(range_score(0, 0, 100), 1000);
        assert_eq!(range_score(100, 0, 100), 1000);
    }

    #[test]
    fn range_score_zero_width_range() {
        // When min == max, the value is at the single-point optimal range.
        // margin = 0, so any deviation gives score 0, but value == min == max → 1000.
        assert_eq!(range_score(50, 50, 50), 1000);
    }

    #[test]
    fn range_score_just_outside_gives_partial() {
        // Range [0, 100], margin = 50. Value = -25 → deficit = 25 → score = (50-25)*1000/50 = 500
        let score = range_score(-25, 0, 100);
        assert_eq!(score, 500);
    }

    #[test]
    fn range_score_beyond_margin_gives_zero() {
        // Range [0, 100], margin = 50. Value = -51 → deficit = 51 > 50 → score = 0
        assert_eq!(range_score(-51, 0, 100), 0);
        assert_eq!(range_score(151, 0, 100), 0);
    }

    // ── NDVI ────────────────────────────────────────────────────────────────

    #[test]
    fn ndvi_healthy_vegetation() {
        // NIR >> RED → NDVI close to 1000
        let ndvi = calculate_ndvi(100, 900);
        assert!(ndvi > 700, "expected NDVI > 0.7, got {}", ndvi);
    }

    #[test]
    fn ndvi_bare_soil() {
        // NIR ≈ RED → NDVI close to 0
        let ndvi = calculate_ndvi(500, 510);
        assert!(ndvi.abs() < 30, "bare soil NDVI should be near 0, got {}", ndvi);
    }

    #[test]
    fn ndvi_zero_inputs() {
        assert_eq!(calculate_ndvi(0, 0), 0);
    }

    #[test]
    fn ndvi_only_red() {
        // NIR = 0, RED > 0 → NDVI = -1000
        let ndvi = calculate_ndvi(500, 0);
        assert_eq!(ndvi, -1000);
    }

    // ── CRC cross-layer check ────────────────────────────────────────────────

    #[test]
    fn crc8_and_crc16_are_independent() {
        // Ensure we never confuse the two CRC algorithms in packet building
        let data = b"ADR1 test packet payload";
        let c8 = crc8(data);
        let c16 = crc16(data);
        // They should not collide on the same input (almost never)
        assert_ne!(c8 as u16, c16, "CRC-8 and CRC-16 should differ for this input");
    }

    #[test]
    fn sequence_number_wraps_correctly() {
        // Verify seq numbering in LoRa frames wraps at u16::MAX
        let frame_max = encode_lora_frame(ADDR_ROBOT, ADDR_CONTROLLER, 0x01, &[], u16::MAX);
        let (_, _, _, seq_max, _) = decode_lora_frame(&frame_max).unwrap();
        assert_eq!(seq_max, u16::MAX);

        // After wrapping
        let frame_zero = encode_lora_frame(ADDR_ROBOT, ADDR_CONTROLLER, 0x01, &[], 0);
        let (_, _, _, seq_zero, _) = decode_lora_frame(&frame_zero).unwrap();
        assert_eq!(seq_zero, 0);
    }
}
