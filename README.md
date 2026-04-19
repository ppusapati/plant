# ADR-1 — Autonomous Deseeder Robot

ADR-1 is an autonomous agricultural robot designed to perform precision soil sampling and deseeding across crop fields in India. It integrates NavIC (IRNSS) positioning, multi-spectral plant-health imaging, real-time soil analysis, and a LoRa + BLE/WiFi control architecture.

```
┌───────────────────────────────────────────────────────────────┐
│  Mobile App (Flutter)                                          │
│  BLE / WiFi (ADR Protocol v1, CRC-8)                         │
├───────────────────────────────────────────────────────────────┤
│  Handheld Controller  ·  ESP32-S3-WROOM-1                     │
│  LoRa SX1276 868 MHz (ADR Protocol v1, CRC-16)               │
├───────────────────────────────────────────────────────────────┤
│  Robot Main Board  ·  STM32H743VIT6 + ESP32-S3               │
│  NavIC GNSS · IMU · 4× BLDC · Cameras · Soil Sensors         │
└───────────────────────────────────────────────────────────────┘
```

---

## Repository layout

```
plant/
├── README.md                    ← you are here
├── docs/                        ← design documents
│   ├── 01-system-architecture.md
│   ├── 02-mechanical-design.md
│   ├── 03-communication-systems.md
│   ├── 04-sensor-systems.md
│   ├── 05-bill-of-materials.md
│   ├── 06-circuit-diagrams.md
│   └── 07-protocol-alignment.md
├── firmware/
│   ├── robot-firmware/          ← STM32H743 Embassy-rs firmware
│   └── controller-firmware/     ← ESP32-S3 esp-hal + Embassy firmware
├── mobile/
│   └── adr1_controller/         ← Flutter mobile app (iOS + Android)
├── hardware/
│   ├── robot-mainboard/         ← KiCad schematics & PCB
│   └── controller-board/        ← KiCad schematics & PCB
└── tools/
    └── protocol-tests/          ← Host-side Rust tests for protocol logic
```

---

## Prerequisites

### All platforms
- Git

### Robot firmware (`firmware/robot-firmware`)
| Tool | Version | Install |
|------|---------|---------|
| Rust | stable + nightly | `curl https://sh.rustup.rs -sSf \| sh` |
| ARM target | `thumbv7em-none-eabihf` | `rustup target add thumbv7em-none-eabihf` |
| probe-rs | latest | `cargo install probe-rs --features cli` |
| ST-LINK V3 | — | Hardware debugger |

### Controller firmware (`firmware/controller-firmware`)
| Tool | Version | Install |
|------|---------|---------|
| Rust | stable | `curl https://sh.rustup.rs -sSf \| sh` |
| ESP toolchain | latest | `cargo install espup && espup install` |
| espflash | latest | `cargo install espflash` |

### Mobile app (`mobile/adr1_controller`)
| Tool | Version | Install |
|------|---------|---------|
| Flutter SDK | ≥ 3.2 | https://flutter.dev/docs/get-started |
| Dart SDK | bundled with Flutter | — |
| Android SDK / Xcode | current | Android Studio / macOS + Xcode |

### Protocol tests (`tools/protocol-tests`)
| Tool | Version | Install |
|------|---------|---------|
| Rust | stable | `curl https://sh.rustup.rs -sSf \| sh` |

---

## Build and flash

### Robot firmware

```bash
cd firmware/robot-firmware

# Build (release)
cargo build --release

# Flash and monitor via ST-LINK V3 + probe-rs
cargo run --release
```

The `.cargo/config.toml` runner is set to `probe-rs run --chip STM32H743VITx`.
Connect SWD pins: SWDIO → PA13, SWCLK → PA14, SWO → PB3.

### Controller firmware

```bash
# Source the Xtensa toolchain environment (once per shell session)
. ~/.espup/export.sh          # or '. $HOME/.cargo/env' after espup install

cd firmware/controller-firmware

# Build
cargo build --release

# Flash via USB (auto-detected by espflash)
cargo run --release
```

The `.cargo/config.toml` runner is `espflash flash --monitor`.
Hold the BOOT button on the ESP32-S3 DevKit while connecting USB to enter download mode if auto-detection fails.

### Mobile app

```bash
cd mobile/adr1_controller

# Install dependencies
flutter pub get

# Analyze
flutter analyze

# Run tests
flutter test

# Run on connected device or emulator
flutter run
```

---

## Development workflow

### Protocol tests (host, no hardware needed)

```bash
cd tools/protocol-tests
cargo test
```

Covers: CRC-8, CRC-16, packet encode/decode, health score algorithm.

### Lint

```bash
# Firmware (check only, no linker required)
cd firmware/robot-firmware && cargo check --target thumbv7em-none-eabihf
cd firmware/controller-firmware && cargo check   # requires Xtensa toolchain

# Flutter
cd mobile/adr1_controller && flutter analyze
```

### Hardware connections (quick reference)

| Peripheral | STM32H743 pins |
|------------|----------------|
| GNSS UART  | PA9 (TX) / PA10 (RX) |
| RS485 Soil | PD5/PD6 + PD4 (DE/RE) |
| ESP32-S3 UART | PB10/PB11 |
| LoRa SX1276 SPI | PA5-PA7 + PA4 (CS) + PC4 (DIO0) |
| I2C Sensors | PB6/PB7 (BME280, BNO055, …) |
| CAN Bus | PD0/PD1 → MCP2562FD |
| E-STOP sense | PD3 |
| Relay ctrl | PD7 |

---

## Communication architecture

See [`docs/07-protocol-alignment.md`](docs/07-protocol-alignment.md) for the full two-layer protocol description.

In brief:
- **LoRa link** (robot ↔ controller, 5 km range): custom binary frame, CRC-16/CCITT
- **BLE/WiFi link** (mobile app ↔ controller, 100 m): ADR protocol v1, CRC-8

---

## Operating modes

| Mode | Value | Description |
|------|-------|-------------|
| Idle | 0 | Standby, motors off |
| Manual | 1 | Joystick control via controller |
| Autonomous | 2 | Waypoint-following mission |
| Deseeding | 3 | Active soil sampling / deseeding |
| Return Home | 4 | Auto-navigate to home position |
| E-STOP | 5 | Emergency stop, all outputs off |
| Charging | 6 | Solar charging, comms only |
| Calibrating | 7 | IMU / GNSS calibration routine |

---

## Safety

- Hardware E-STOP button on controller (60 A relay, NC contact)
- Software watchdog (Embassy watchdog task, 5 s timeout)
- Obstacle detection (4 × MB1240 ultrasonic, emergency-stop at < 500 mm)
- Tilt protection (BNO055, cut motors at > 30°)
- Geofence enforcement (EKF position vs. configured boundary polygon)
- Communication loss: robot halts autonomy after 5 s without heartbeat

---

## License

MIT — see individual component directories for details.
