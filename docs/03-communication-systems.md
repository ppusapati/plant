# ADR-1 Communication Systems Design Document

## 1. Overview

The ADR-1 employs a multi-layered communication architecture ensuring reliable
command/telemetry links across varying distances and conditions.

```
┌──────────────────────────────────────────────────────────────────┐
│                    COMMUNICATION STACK                            │
│                                                                  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐             │
│  │   Layer 3   │  │   Layer 3   │  │   Layer 3   │             │
│  │ Application │  │ Application │  │ Application │             │
│  │ Telemetry   │  │ Config/Debug│  │ FW Update   │             │
│  │ Commands    │  │ Data Dump   │  │ Bulk Data   │             │
│  ├─────────────┤  ├─────────────┤  ├─────────────┤             │
│  │   Layer 2   │  │   Layer 2   │  │   Layer 2   │             │
│  │ ADR Protocol│  │ GATT Profile│  │ TCP/IP      │             │
│  │ (Custom)    │  │             │  │ HTTP/MQTT   │             │
│  ├─────────────┤  ├─────────────┤  ├─────────────┤             │
│  │   Layer 1   │  │   Layer 1   │  │   Layer 1   │             │
│  │ LoRa 868MHz │  │ BLE 5.0    │  │ WiFi 2.4GHz │             │
│  │ SX1276      │  │ (ESP32/    │  │ (ESP32/     │             │
│  │             │  │  STM32WB)  │  │  ESP8266)   │             │
│  └─────────────┘  └─────────────┘  └─────────────┘             │
│                                                                  │
│  PRIMARY LINK      SECONDARY LINK    TERTIARY LINK              │
│  Range: 5km        Range: 100m       Range: 50m                │
│  Always active      On-demand         On-demand                 │
└──────────────────────────────────────────────────────────────────┘
```

---

## 2. NavIC (IRNSS) Navigation System

### 2.1 System Overview

NavIC (Navigation with Indian Constellation) is India's indigenous satellite
navigation system operated by ISRO, providing positioning over India and
surrounding regions (coverage: 5°S-40°N latitude, 50°E-100°E longitude).

### 2.2 NavIC Constellation
```
                    NavIC Satellite Constellation
                    ============================

        GEO Satellites (Geostationary - 36,000 km)
        ┌─────────────────────────────────────────┐
        │  IRNSS-1C     IRNSS-1F     IRNSS-1G    │
        │  (83°E)       (32.5°E)     (129.5°E)   │
        └─────────────────────────────────────────┘

        GSO Satellites (Geosynchronous - inclined)
        ┌─────────────────────────────────────────────┐
        │  IRNSS-1A*  IRNSS-1B  IRNSS-1D  IRNSS-1E  │
        │  (55°E)     (55°E)    (111.75°E) (111.75°E)│
        │  * Clock failure, IRNSS-1I replacement      │
        └─────────────────────────────────────────────┘

        Signals:
        ├── L5 Band: 1176.45 MHz (Standard Positioning)
        ├── S Band: 2492.028 MHz (Restricted Service)
        └── L1 Band: 1575.42 MHz (added in newer sats)
```

### 2.3 NavIC Module Selection: SkyTraq NavIC-enabled GNSS

| Parameter | Specification |
|-----------|--------------|
| Module | u-blox NEO-M9N or SkyTraq S2525F8-GL-RTK |
| Constellations | NavIC L5 + GPS L1/L5 + GLONASS L1 + Galileo E1 |
| Channels | 32 tracking, 72 acquisition |
| Position accuracy | 5m CEP (NavIC+GPS), <1m with SBAS |
| Update rate | 10 Hz (configurable 1-25 Hz) |
| TTFF | Cold: 30s, Warm: 5s, Hot: 1s |
| Power consumption | 30mW continuous tracking |
| Interface | UART (115200 baud) + I2C |
| Protocol | NMEA 0183 + UBX proprietary |
| Antenna | Active patch antenna with LNA + SAW filter |

### 2.4 NavIC Integration in Robot

```
                                    ┌──────────────┐
   Active Patch Antenna ──────────►│ LNA + SAW    │
   (25x25mm ceramic)               │ Filter       │
   Mounted on sensor mast          │ (1176 MHz)   │
   Ground plane: 70x70mm           └──────┬───────┘
                                          │ Coax (U.FL)
                                   ┌──────▼───────┐
                                   │ NavIC GNSS   │
                                   │ Module       │
                                   │              │
                                   │ UART TX/RX───┼──► STM32 UART4
                                   │ PPS──────────┼──► STM32 TIM input
                                   │ I2C──────────┼──► Config bus
                                   │ RESET────────┼──► GPIO control
                                   └──────────────┘
```

### 2.5 NavIC Data Processing Pipeline

```
NMEA Sentences:
  $GNGGA → Position fix (lat, lon, alt, fix quality, HDOP)
  $GNRMC → Speed, heading, date/time
  $GNGSA → Active satellites, DOP values
  $GNGSV → Satellites in view (per constellation)
  $GIGSV → NavIC-specific satellites in view

Processing:
  1. Parse NMEA at 10Hz
  2. Extract NavIC-specific satellites from $GIGSV
  3. Kalman filter fusion with IMU (BNO055)
  4. Wheel odometry correction
  5. Output: Fused position at 50Hz to navigation controller
```

---

## 3. LoRa Communication (Primary Link)

### 3.1 Hardware: Semtech SX1276

| Parameter | Specification |
|-----------|--------------|
| Frequency | 865-867 MHz (India ISM band) |
| Tx power | +2 to +20 dBm (configurable) |
| Sensitivity | -148 dBm (SF12, 125kHz BW) |
| Modulation | LoRa CSS (Chirp Spread Spectrum) |
| Bandwidth | 125/250/500 kHz |
| Spreading factor | SF7-SF12 |
| Coding rate | 4/5 to 4/8 |
| Interface | SPI (10 MHz max) |
| Link budget | 168 dB (at SF12, 20dBm) |

### 3.2 LoRa Configuration Profiles

| Profile | SF | BW | CR | Data Rate | Range | Use Case |
|---------|----|----|-----|-----------|-------|----------|
| FAST | SF7 | 500kHz | 4/5 | 21.9 kbps | 1 km | Joystick control (low latency) |
| NORMAL | SF9 | 250kHz | 4/6 | 3.5 kbps | 3 km | Standard telemetry |
| LONG_RANGE | SF12 | 125kHz | 4/8 | 0.29 kbps | 5+ km | Emergency/heartbeat only |
| ADAPTIVE | Auto | Auto | Auto | Variable | Variable | Auto-select based on RSSI |

### 3.3 LoRa Antenna Design

```
Robot: λ/4 Ground Plane Antenna (868 MHz)
============================================

                 │ λ/4 = 86.4mm
                 │ (1.5mm brass rod)
                 │
                 │
    ─────────────┼─────────────  Ground plane
    ╲           │           ╱    (4x radials, 45° angle)
      ╲         │         ╱      Each radial: 86.4mm
        ╲       │       ╱
          ╲     │     ╱
            ╲   │   ╱
              ╲ │ ╱
    ────────────┴────────────
                │
           SMA connector
           to SX1276

    Gain: 2.15 dBi
    VSWR: <1.5:1
    Impedance: 50Ω
    Polarization: Vertical

Controller: PCB Trace Antenna (868 MHz)
========================================

    ┌──────────────────────────────────────┐
    │  Meander Line Inverted-F Antenna     │
    │                                      │
    │  ┌─┐ ┌─┐ ┌─┐ ┌─┐ ┌─┐ ┌─┐         │
    │  │ │ │ │ │ │ │ │ │ │ │ │         │
    │  │ └─┘ └─┘ └─┘ └─┘ └─┘ │         │
    │  │                       │         │
    │  └───────────────────────┘         │
    │  ▲Feed point      ▲Ground          │
    │                                      │
    │  Keep-out zone: 10mm around antenna  │
    │  On 1.6mm FR4, ground plane cutout   │
    └──────────────────────────────────────┘

    Gain: 0.5 dBi (sufficient for 2km link)
    Size: 40x15mm PCB area
```

### 3.4 LoRa Protocol Stack

```
┌──────────────────────────────────────────────────┐
│ Application Layer                                 │
│ - Message serialization (packed binary)           │
│ - Telemetry formatting                           │
│ - Command parsing                                │
├──────────────────────────────────────────────────┤
│ Reliability Layer                                 │
│ - Sequence numbering (16-bit wrap)               │
│ - ACK/NACK with 200ms timeout                    │
│ - Retry: 3 attempts with backoff                 │
│ - Duplicate detection (seq number cache)         │
├──────────────────────────────────────────────────┤
│ Framing Layer                                     │
│ ┌──────┬─────┬─────┬───────┬─────────┬─────┬───┐│
│ │SYNC  │SRC  │DST  │MSG_ID │PAYLOAD  │SEQ  │CRC││
│ │0xAD01│1B   │1B   │1B     │0-200B   │2B   │2B ││
│ └──────┴─────┴─────┴───────┴─────────┴─────┴───┘│
│ - CRC-16/CCITT over entire frame                 │
│ - Max frame: 209 bytes (LoRa FIFO: 256B)        │
├──────────────────────────────────────────────────┤
│ Radio Layer (SX1276 Driver)                       │
│ - SPI register access                            │
│ - FIFO management                                │
│ - IRQ handling (TxDone, RxDone, Timeout)         │
│ - Frequency hopping (3 channels, FHSS)           │
│ - LBT (Listen Before Talk) for regulatory        │
└──────────────────────────────────────────────────┘
```

### 3.5 Frequency Hopping Scheme

```
India ISM Band: 865-867 MHz (ETSI-like, 25mW ERP limit exceeded with
                              special license for agricultural IoT)

Channel Plan:
  CH0: 865.20 MHz  ─┐
  CH1: 866.00 MHz   ├── Hop sequence: CH0→CH1→CH2→CH0...
  CH2: 866.80 MHz  ─┘   Hop interval: 400ms

  Sync: PPS-aligned hop timing (NavIC 1PPS signal)
  Fallback: If no PPS, use crystal-synced 32.768kHz RTC
```

---

## 4. BLE 5.0 Communication (Secondary Link)

### 4.1 Implementation

**Robot side**: STM32WB55 companion chip (or external BLE module)
**Controller side**: ESP32-S3 integrated BLE 5.0

### 4.2 GATT Service Profile

```
ADR-1 GATT Profile
├── Device Information Service (0x180A)
│   ├── Manufacturer Name: "ADR-1"
│   ├── Model Number: "R1.0" / "C1.0"
│   ├── Firmware Version: "1.0.0"
│   └── Hardware Version: "Rev A"
│
├── Robot Telemetry Service (UUID: 0xAD01)
│   ├── Position Characteristic (Notify, 16B)
│   │   └── lat(f32) + lon(f32) + alt(f32) + hdop(f32)
│   ├── Sensor Data Characteristic (Notify, 32B)
│   │   └── soil_moisture + soil_temp + soil_npk + soil_ph + ...
│   ├── Plant Health Characteristic (Notify, 12B)
│   │   └── ndvi(f32) + leaf_temp(f32) + disease_score(f32)
│   ├── Battery Status Characteristic (Notify, 8B)
│   │   └── voltage(f32) + soc(u8) + current(f32)
│   └── Motor Status Characteristic (Notify, 16B)
│       └── rpm[4](u16) + current[4](f32)
│
├── Robot Command Service (UUID: 0xAD02)
│   ├── Drive Command Characteristic (Write, 8B)
│   │   └── linear_vel(f32) + angular_vel(f32)
│   ├── Mode Command Characteristic (Write, 1B)
│   │   └── 0=IDLE, 1=MANUAL, 2=AUTO, 3=RTH, 4=ESTOP
│   ├── Waypoint Characteristic (Write, 12B)
│   │   └── lat(f32) + lon(f32) + action(u32)
│   └── Config Characteristic (Write, varies)
│       └── Key-value configuration pairs
│
├── OTA Update Service (UUID: 0xAD03)
│   ├── FW Version Characteristic (Read)
│   ├── FW Data Characteristic (Write, 244B MTU)
│   └── FW Control Characteristic (Write/Notify)
│       └── START, DATA, VERIFY, APPLY, STATUS
│
└── Diagnostics Service (UUID: 0xAD04)
    ├── Error Log Characteristic (Read/Notify)
    ├── Debug Console Characteristic (Write/Notify)
    └── Self-Test Characteristic (Write/Read)
```

### 4.3 BLE Connection Parameters

| Parameter | Value | Justification |
|-----------|-------|---------------|
| Connection interval | 15ms (min) | Low latency for live sensor data |
| Slave latency | 0 | Always responsive |
| Supervision timeout | 2000ms | Handles brief obstructions |
| MTU | 247 bytes | Max throughput per packet |
| PHY | 2M PHY (BLE 5.0) | Double throughput vs BLE 4.x |
| Tx Power | +8 dBm | 100m range in open field |

---

## 5. WiFi Communication (Tertiary Link)

### 5.1 Implementation

**Robot side**: ESP32-S3 (co-processor) or ESP-AT module on UART
**Controller side**: ESP32-S3 integrated WiFi

### 5.2 WiFi Modes

| Mode | Use Case | Configuration |
|------|----------|---------------|
| AP Mode (Robot) | Direct connection for data dump | SSID: ADR1_XXXX, WPA2-PSK |
| STA Mode | Connect to farm WiFi for cloud upload | Enterprise WPA2 support |
| AP+STA | Simultaneous controller link + cloud | Dual interface on ESP32-S3 |

### 5.3 WiFi Services

```
Robot WiFi Services:
├── HTTP Server (port 80)
│   ├── /api/status       → JSON robot status
│   ├── /api/sensors      → JSON all sensor data
│   ├── /api/map          → JSON waypoints + coverage map
│   ├── /api/logs         → Download log files
│   └── /api/config       → GET/POST configuration
│
├── MQTT Client (when STA connected)
│   ├── adr1/{id}/telemetry   → Periodic sensor push
│   ├── adr1/{id}/alerts      → Alert notifications
│   ├── adr1/{id}/command     → Remote commands (subscribe)
│   └── adr1/{id}/ota         → OTA update trigger
│
└── mDNS: adr1-{serial}.local
```

---

## 6. RS485/Modbus RTU (Sensor Bus)

### 6.1 Bus Configuration

```
STM32 UART2 ──► MAX3485 ──► RS485 Bus (A/B twisted pair)
                                │
                    ┌───────────┼───────────┬────────────┐
                    │           │           │            │
              ┌─────▼─┐  ┌─────▼─┐  ┌─────▼─┐  ┌──────▼──┐
              │NPK    │  │pH     │  │EC     │  │Soil    │
              │Sensor │  │Sensor │  │Sensor │  │5-in-1  │
              │Addr:1 │  │Addr:2 │  │Addr:3 │  │Addr:4  │
              └───────┘  └───────┘  └───────┘  └────────┘
```

| Parameter | Value |
|-----------|-------|
| Baud rate | 9600 (default for soil sensors) |
| Data format | 8N1 |
| Protocol | Modbus RTU |
| Termination | 120Ω at each end |
| Max devices | 32 (Modbus limit) |
| Cable | Shielded twisted pair, max 100m |
| Polling interval | 5 seconds per sensor cycle |

### 6.2 Modbus Register Map (Soil NPK Sensor Example)

| Register | Address | Description | Unit | Scale |
|----------|---------|-------------|------|-------|
| Moisture | 0x0000 | Soil moisture | % | ×0.1 |
| Temperature | 0x0001 | Soil temp | °C | ×0.1 |
| EC | 0x0002 | Conductivity | µS/cm | ×1 |
| pH | 0x0003 | Soil pH | pH | ×0.01 |
| Nitrogen | 0x0004 | N content | mg/kg | ×1 |
| Phosphorus | 0x0005 | P content | mg/kg | ×1 |
| Potassium | 0x0006 | K content | mg/kg | ×1 |

---

## 7. CAN Bus (Internal Motor Control)

### 7.1 CAN 2.0B Configuration

| Parameter | Value |
|-----------|-------|
| Bit rate | 500 kbps |
| Termination | 120Ω at each end of bus |
| Transceiver | MCP2562FD (robot), integrated in STM32 |
| Max nodes | 6 (4 motor drivers + main MCU + BMS) |
| Protocol | Custom, CANopen-inspired |

### 7.2 CAN Message IDs

| CAN ID | Direction | Description | DLC |
|--------|-----------|-------------|-----|
| 0x100 | MCU→Motor | Motor 1 (FL) speed command | 8B |
| 0x101 | MCU→Motor | Motor 2 (FR) speed command | 8B |
| 0x102 | MCU→Motor | Motor 3 (RL) speed command | 8B |
| 0x103 | MCU→Motor | Motor 4 (RR) speed command | 8B |
| 0x180 | Motor→MCU | Motor 1 status (RPM, current, temp) | 8B |
| 0x181 | Motor→MCU | Motor 2 status | 8B |
| 0x182 | Motor→MCU | Motor 3 status | 8B |
| 0x183 | Motor→MCU | Motor 4 status | 8B |
| 0x200 | MCU→Motor | Deseeding motor speed | 8B |
| 0x280 | Motor→MCU | Deseeding motor status | 8B |
| 0x300 | BMS→MCU | Battery status | 8B |
| 0x301 | BMS→MCU | Cell voltages | 8B |
| 0x7FF | MCU→All | Emergency stop broadcast | 0B |

---

## 8. Communication Failover Strategy

```
┌──────────────────────────────────────────────────┐
│              LINK PRIORITY ENGINE                  │
│                                                    │
│  Normal Operation:                                │
│    LoRa ──► Primary for all telemetry/commands    │
│    BLE  ──► Idle (standby)                        │
│    WiFi ──► Idle (standby)                        │
│                                                    │
│  LoRa RSSI < -130 dBm (weak signal):             │
│    LoRa ──► Switch to LONG_RANGE profile          │
│    BLE  ──► Activate if in range                  │
│                                                    │
│  LoRa link lost (no ACK for 3s):                  │
│    LoRa ──► Continue retry with LONG_RANGE        │
│    BLE  ──► Promote to primary if available       │
│    Robot ──► Enter HOLD position                  │
│                                                    │
│  All links lost (10s):                            │
│    Robot ──► Execute RTH (Return to Home)         │
│    Robot ──► Log all data to SD card              │
│    Robot ──► Beacon LoRa LONG_RANGE every 5s      │
│                                                    │
│  WiFi activation triggers:                        │
│    - User button press on controller              │
│    - Data download request                        │
│    - Firmware update initiation                   │
│    - Cloud sync when farm WiFi in range           │
└──────────────────────────────────────────────────┘
```

---

## 9. Regulatory Compliance

| Regulation | Frequency | Requirement | ADR-1 Compliance |
|------------|-----------|-------------|-----------------|
| India ISM | 865-867 MHz | 25 mW ERP, <1% duty cycle | LoRa: 20dBm PA + LBT |
| India ISM | 2.4 GHz | 4W EIRP | WiFi/BLE within limits |
| WPC (India) | NavIC L5 | Receive only | Compliant |
| EMC | All | IS 13252 (CISPR 22) | PCB design for EMI |
| FCC Part 15 | All | (for export) | Layout designed for compliance |

---

## 10. Link Budget Analysis

### LoRa Link Budget (Worst Case: SF12, 125kHz BW)

```
Transmit Power:         +20 dBm
Tx Antenna Gain:        +2.15 dBi (robot ground plane)
Cable/Connector Loss:   -1.5 dB
                        ─────────
EIRP:                   +20.65 dBm

Free Space Path Loss:   -112.4 dB (at 5km, 868MHz)
Atmospheric Loss:       -1 dB
Foliage Loss:           -10 dB (crop canopy)
Fading Margin:          -10 dB
                        ─────────
Received Power:         -112.75 dBm

Rx Antenna Gain:        +0.5 dBi (controller PCB antenna)
Rx Sensitivity:         -148 dBm (SF12, 125kHz)
                        ─────────
Link Margin:            +35.75 dB ✓ (excellent)
```
