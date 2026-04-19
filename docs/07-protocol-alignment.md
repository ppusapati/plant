# ADR-1 Protocol Alignment

## 1. Overview

The ADR-1 system uses **two distinct binary protocol layers**:

| Layer | Link | Endpoints | CRC | Max range |
|-------|------|-----------|-----|-----------|
| LoRa transport | Robot STM32 ↔ Controller ESP32-S3 | LoRa SX1276 @ 868 MHz | CRC-16/CCITT | 5 km |
| BLE/WiFi transport | Mobile app ↔ Controller ESP32-S3 | BLE 5.0 / WebSocket | CRC-8 (poly 0x07) | 100 m |

Both layers share the same 2-byte magic (`0xAD 0x01`) and protocol version byte, making packet origin unambiguous at a sniffer or bridge.

---

## 2. LoRa Frame (Robot ↔ Controller)

Used exclusively over the SX1276 868 MHz radio link between the robot's STM32H743 and the handheld controller's ESP32-S3.

```
Offset  Len  Field
──────  ───  ─────────────────────────────────────────
  0      2   SYNC word  0xAD 0x01
  2      1   SRC address (0x01 = Robot, 0x02 = Controller)
  3      1   DST address (0x01 = Robot, 0x02 = Controller)
  4      1   Message ID (see §4)
  5      1   Payload length (0–200 bytes)
  6      n   Payload
  6+n    2   Sequence number (little-endian u16, wraps)
  8+n    2   CRC-16/CCITT (little-endian, computed over bytes 0..8+n-1)
```

**CRC-16 polynomial:** 0xA001 (reflected CRC-16/IBM, same as Modbus)  
**Maximum frame size:** 209 bytes (fits SX1276 256-byte FIFO)

### 2.1 Address constants

| Address | Value | Node |
|---------|-------|------|
| `ADDR_ROBOT` | `0x01` | STM32H743 main board |
| `ADDR_CONTROLLER` | `0x02` | ESP32-S3 handheld controller |

### 2.2 LoRa adaptive rate profiles

| Profile | SF | BW | CR | Data rate | Use case |
|---------|----|----|-----|-----------|----------|
| FAST | SF7 | 500 kHz | 4/5 | 21.9 kbps | Joystick control (< 1 km) |
| NORMAL | SF9 | 250 kHz | 4/6 | 3.5 kbps | Standard telemetry (< 3 km) |
| LONG_RANGE | SF12 | 125 kHz | 4/8 | 0.29 kbps | Emergency / heartbeat (5+ km) |

The controller's `AdaptiveRate` module upgrades to FAST after 10 consecutive successful receptions with RSSI > −80 dBm and downgrades to LONG_RANGE after 3 consecutive failures.

---

## 3. BLE/WiFi Frame (Mobile App ↔ Controller)

Used between the Flutter mobile app and the controller's ESP32-S3 over:
- **BLE 5.0** GATT characteristic `4adr0002-…` (commands) / `4adr0003-…` (telemetry notify)
- **WebSocket** `ws://<controller-ip>:8080/ws` (WiFi/AP mode)

```
Offset  Len  Field
──────  ───  ─────────────────────────────────────────
  0      1   Magic byte 1  0xAD
  1      1   Magic byte 2  0x01
  2      1   Protocol version  0x01
  3      1   Message type (Command ID or Telemetry ID, see §4)
  4      2   Payload length (big-endian u16)
  6      n   Payload
  6+n    1   CRC-8 (poly 0x07, computed over bytes 0..6+n-1)
```

**CRC-8 polynomial:** 0x07 (CRC-8/SMBUS)  
**Maximum payload:** 512 bytes (`kMaxPayloadSize`)

### 3.1 BLE characteristics

| UUID suffix | Direction | Purpose |
|-------------|-----------|---------|
| `4adr0002-…` | App → Controller | Command write (write-without-response) |
| `4adr0003-…` | Controller → App | Telemetry notify |
| `4adr0004-…` | Controller → App | Sensor data notify |
| `4adr0005-…` | App ↔ Controller | Mission plan read/write |

---

## 4. Message IDs

### 4.1 Commands (app / controller → robot)

| ID | Hex | LoRa `MessageId` | BLE/WiFi `CommandId` | Payload |
|----|-----|------------------|----------------------|---------|
| Ping | 0x01 | `Ping` | `ping` | — |
| Manual control | 0x11 | `ManualCommand` | `manualControl` | f32 throttle + f32 steering (little-endian) |
| Emergency stop | 0x1F | `Emergency` | `emergencyStop` | — |
| Set mode | 0x10 | `ModeCommand` | `setMode` | u8 mode |
| Set speed | 0x20 | — | `setSpeed` | f32 speed |
| Start mission | 0x30 | `MissionCommand` | `startMission` | — |
| Pause mission | 0x31 | — | `pauseMission` | — |
| Resume mission | 0x32 | — | `resumeMission` | — |
| Abort mission | 0x33 | — | `abortMission` | — |
| Add waypoint | 0x34 | — | `addWaypoint` | i32 lat_e7, i32 lon_e7, u8 action, f32 speed |
| Clear waypoints | 0x35 | — | `clearWaypoints` | — |
| Return to home | 0x36 | — | `returnToHome` | — |
| Request telemetry | 0x40 | — | `requestTelemetry` | — |
| Request sensors | 0x41 | — | `requestSensorData` | — |
| Deploy probes | 0x70 | — | `deployProbes` | — |
| Retract probes | 0x71 | — | `retractProbes` | — |
| Start deseeding | 0x80 | — | `startDeseeding` | — |
| Stop deseeding | 0x81 | — | `stopDeseeding` | — |

### 4.2 Telemetry (robot → controller / app)

| ID | Hex | LoRa `MessageId` | BLE/WiFi `TelemetryId` | Payload |
|----|-----|------------------|------------------------|---------|
| Heartbeat | 0x01 | `Heartbeat` | `heartbeat` | u8 mode, u8 battery_soc |
| Position | 0x10 | `NavStatus` (part) | `position` | i32 lat_e7, i32 lon_e7, f32 alt, f32 spd, f32 hdg, u8 sats, f32 hdop, u8 fix |
| Battery | 0x20 | `Telemetry` (part) | `batteryStatus` | f32 V, f32 A, f32 soc, f32 T, f32 W, u16 est_min, u8 charging |
| Soil data | 0x51 | `SoilData` | `soilData` | 14 bytes (see §5.1) |
| Environment | 0x52 | `PlantHealth` (env part) | `environmentData` | 32 bytes (see §5.2) |
| Health score | 0x53 | `PlantHealth` | `healthScore` | i16 ndvi, i16 leaf_temp, u16 height, u16 canopy, u16 score, u8 disease |
| Error report | 0xF0 | — | `errorReport` | u8 code, string message |
| ACK | 0xFE | `Ack` | `ack` | u16 acked_seq |
| NACK | 0xFF | — | `nack` | u16 nacked_seq, u8 reason |

---

## 5. Payload formats

### 5.1 Soil data payload (14 bytes)

```
Offset  Len  Field            Unit / scale
──────  ───  ───────────────  ──────────────────────
  0      2   moisture         % × 10, u16
  2      2   temperature      °C × 100, i16
  4      2   nitrogen         mg/kg, u16
  6      2   phosphorus       mg/kg, u16
  8      2   potassium        mg/kg, u16
  10     2   pH               × 100, u16
  12     2   EC               µS/cm, u16
```

### 5.2 Environment payload (32 bytes)

```
Offset  Len  Field            Unit
──────  ───  ───────────────  ────────
  0      4   temperature      °C, f32
  4      4   humidity         %, f32
  8      4   pressure         hPa, f32
  12     4   light_lux        lux, f32
  16     4   uv_index         (unitless), f32
  20     4   wind_speed       m/s, f32
  24     4   rainfall         mm, f32
  28     4   leaf_temp        °C, f32
```

### 5.3 Manual control payload (8 bytes)

```
Offset  Len  Field            Range      Encoding
──────  ───  ───────────────  ─────────  ─────────────────
  0      4   throttle         −1.0..1.0  f32 little-endian
  4      4   steering         −1.0..1.0  f32 little-endian
```

Values are clamped to [−1.0, 1.0] by `PacketCodec.encodeManualControl()` before encoding.

---

## 6. Bridging role of the controller ESP32-S3

The controller ESP32-S3 acts as a **protocol bridge**:

```
Mobile App  ──BLE/WiFi──►  ESP32-S3  ──LoRa──►  STM32H743 (Robot)
            (CRC-8 frame)            (CRC-16 frame)
```

**Inbound (app → robot):**
1. Receive BLE/WiFi frame from mobile app (CRC-8, verify)
2. Extract command and payload
3. Build LoRa frame (CRC-16) addressed to `ADDR_ROBOT`
4. Transmit via SX1276

**Outbound (robot → app):**
1. Receive LoRa frame from robot (CRC-16, verify)
2. Extract telemetry message and payload
3. Build BLE/WiFi frame (CRC-8)
4. Send via GATT notify or WebSocket push

The controller firmware's `comms/mod.rs` manages the LoRa side; the BLE/WiFi server implementation lives in the ESP32-S3's `esp-wifi` stack and bridges to the same channel structures.

---

## 7. CRC reference implementations

### CRC-8 (poly 0x07) — used on BLE/WiFi frames

```rust
fn crc8(data: &[u8]) -> u8 {
    let mut crc: u8 = 0x00;
    for byte in data {
        crc ^= byte;
        for _ in 0..8 {
            if crc & 0x80 != 0 { crc = (crc << 1) ^ 0x07; }
            else                { crc <<= 1; }
        }
    }
    crc
}
```

```dart
int _crc8(List<int> data) {
  int crc = 0x00;
  for (final byte in data) {
    crc ^= byte;
    for (int i = 0; i < 8; i++) {
      if ((crc & 0x80) != 0) { crc = ((crc << 1) ^ 0x07) & 0xFF; }
      else                   { crc = (crc << 1) & 0xFF; }
    }
  }
  return crc;
}
```

### CRC-16/IBM (poly 0xA001) — used on LoRa frames

```rust
fn crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for byte in data {
        crc ^= *byte as u16;
        for _ in 0..8 {
            if crc & 0x0001 != 0 { crc = (crc >> 1) ^ 0xA001; }
            else                  { crc >>= 1; }
        }
    }
    crc
}
```

---

## 8. Reliability mechanisms

| Mechanism | LoRa layer | BLE/WiFi layer |
|-----------|------------|----------------|
| Sequence numbers | 16-bit wrapping counter, per-packet | — |
| ACK/NACK | Yes, 200 ms timeout, 3 retries | — (TCP/BLE GATT reliable) |
| Duplicate detection | Sequence number cache | — |
| Heartbeat | 1 Hz robot → controller | — (connection state) |
| Link watchdog | 5 s no heartbeat → E-STOP | Auto-reconnect (5 attempts, exp backoff) |
