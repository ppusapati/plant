# Autonomous Deseeder Robot - System Architecture & Requirements

## 1. Project Overview

The Autonomous Deseeder Robot (ADR-1) is a field-deployable agricultural robot designed for:
- **Autonomous deseeding** (weed seed removal) across crop rows
- **Plant health monitoring** via multispectral imaging and NDVI analysis
- **Soil health assessment** through in-situ sensor probes
- **Environmental parameter tracking** (temperature, humidity, light, wind)
- **Real-time telemetry** to a handheld controller via multiple communication links
- **NavIC-based precision navigation** for Indian agricultural contexts

---

## 2. System Block Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        ADR-1 ROBOT SYSTEM                               │
│                                                                         │
│  ┌──────────┐  ┌──────────┐  ┌───────────┐  ┌──────────┐              │
│  │ NavIC/GPS │  │ IMU 9DOF │  │ LiDAR     │  │ Ultrasonic│              │
│  │ Module   │  │ BNO055   │  │ TFmini-S-I│  │ MB1240 x4 │              │
│  └────┬─────┘  └────┬─────┘  └─────┬─────┘  └─────┬─────┘              │
│       │              │              │              │                     │
│       ├──────────────┼──────────────┼──────────────┘                     │
│       │              │              │                                    │
│  ┌────▼──────────────▼──────────────▼────────────────────────────┐      │
│  │                  STM32H743 MAIN MCU                            │      │
│  │  ┌─────────┐ ┌─────────┐ ┌──────────┐ ┌──────────────┐       │      │
│  │  │ Motor   │ │ Sensor  │ │ Comms    │ │ Deseeding    │       │      │
│  │  │ Control │ │ Fusion  │ │ Manager  │ │ Actuator Ctrl│       │      │
│  │  └────┬────┘ └────┬────┘ └────┬─────┘ └──────┬───────┘       │      │
│  └───────┼───────────┼───────────┼───────────────┼───────────────┘      │
│          │           │           │               │                      │
│  ┌───────▼──┐  ┌─────▼────┐  ┌──▼──────────┐  ┌▼─────────────┐        │
│  │ Motor    │  │ Sensor   │  │ Comms       │  │ Deseeding    │        │
│  │ Drivers  │  │ Array    │  │ Subsystem   │  │ Mechanism    │        │
│  │ (4x BLDC)│  │          │  │             │  │              │        │
│  └──────────┘  │ ●Soil NPK│  │ ●LoRa 868  │  │ ●Rotary Hoe  │        │
│                │ ●Soil pH │  │ ●BLE 5.0   │  │ ●Seed Vac    │        │
│                │ ●Moisture│  │ ●WiFi      │  │ ●Servo Arm   │        │
│                │ ●Temp/Hum│  │ ●NavIC     │  │              │        │
│                │ ●NDVI Cam│  │ ●RS485     │  └──────────────┘        │
│                │ ●Light   │  └─────────────┘                          │
│                └──────────┘                                            │
│                                                                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐              │
│  │ Power System │  │ Battery Mgmt │  │ Solar Trickle    │              │
│  │ 48V/24V/5V  │  │ BQ76952      │  │ Charge Panel     │              │
│  └──────────────┘  └──────────────┘  └──────────────────┘              │
└─────────────────────────────────────────────────────────────────────────┘

                              │ LoRa / BLE / WiFi
                              │
                              ▼

┌─────────────────────────────────────────────────────────────────────────┐
│                     HANDHELD CONTROLLER                                  │
│                                                                         │
│  ┌──────────────┐  ┌──────────┐  ┌──────────┐  ┌──────────────┐       │
│  │ ESP32-S3     │  │ 3.5" TFT │  │ LoRa     │  │ Joystick x2  │       │
│  │ Main MCU     │──│ Display  │  │ SX1276   │  │ + Buttons    │       │
│  │              │  │ ILI9488  │  │          │  │              │       │
│  └──────┬───────┘  └──────────┘  └──────────┘  └──────────────┘       │
│         │                                                               │
│  ┌──────▼──────┐  ┌──────────┐  ┌──────────┐  ┌──────────────┐       │
│  │ BLE 5.0     │  │ NavIC    │  │ SD Card  │  │ Haptic       │       │
│  │ (built-in)  │  │ Receiver │  │ Logger   │  │ Feedback     │       │
│  └─────────────┘  └──────────┘  └──────────┘  └──────────────┘       │
│                                                                         │
│  ┌──────────────┐  ┌──────────────┐                                    │
│  │ LiPo 3.7V   │  │ USB-C        │                                    │
│  │ 3000mAh     │  │ Charging     │                                    │
│  └──────────────┘  └──────────────┘                                    │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Functional Requirements

### 3.1 Deseeding Operations
| Requirement | Specification |
|-------------|--------------|
| Deseeding method | Mechanical rotary hoe + vacuum seed extraction |
| Working width | 400mm per pass |
| Target depth | 10-50mm adjustable |
| Speed | 0.3-1.5 km/h during deseeding |
| Seed detection | NDVI camera + AI weed classification (edge inference) |
| Accuracy | >90% weed seed identification |

### 3.2 Plant Health Monitoring
| Parameter | Sensor | Range | Accuracy |
|-----------|--------|-------|----------|
| NDVI | Dual-band camera (Red + NIR) | 0.0-1.0 | ±0.02 |
| Leaf temperature | MLX90614 IR thermometer | -40 to 125°C | ±0.5°C |
| Canopy coverage | Downward RGB camera | 0-100% | ±5% |
| Plant height | TFmini-S-I LiDAR (Industrial) | 0.1-12m | ±1cm |
| Disease detection | Multispectral analysis | RGB+NIR | Visual AI |

### 3.3 Soil Health Monitoring
| Parameter | Sensor | Range | Accuracy |
|-----------|--------|-------|----------|
| Soil moisture | Capacitive probe (no corrosion) | 0-100% VWC | ±3% |
| Soil temperature | DS18B20 waterproof | -55 to 125°C | ±0.5°C |
| Soil NPK | RS485 NPK sensor | N:0-1999mg/kg | ±2% FS |
| Soil pH | RS485 pH probe | 3.0-9.0 pH | ±0.1 pH |
| Soil EC | Conductivity probe | 0-20000 µS/cm | ±2% |

### 3.4 Environmental Monitoring
| Parameter | Sensor | Range |
|-----------|--------|-------|
| Air temperature | BME280 | -40 to 85°C |
| Humidity | BME280 | 0-100% RH |
| Barometric pressure | BME280 | 300-1100 hPa |
| Light intensity | BH1750 | 1-65535 lux |
| UV index | VEML6075 | 0-15 UV index |
| Wind speed | Anemometer (pulse) | 0-60 m/s |
| Rainfall | Tipping bucket | 0.2mm/tip |

### 3.5 Navigation & Positioning
| Requirement | Specification |
|-------------|--------------|
| Primary GNSS | NavIC (IRNSS) L5/S-band |
| Secondary GNSS | GPS L1 + GLONASS |
| Position accuracy | <1m CEP (NavIC), <2.5m (GPS standalone) |
| RTK support | Optional RTK base station for cm-level |
| Update rate | 10 Hz navigation solution |
| IMU | BNO055 9-DOF (accel+gyro+mag) |
| Odometry | Wheel encoders (600 PPR per wheel) |
| Obstacle avoidance | 4x MB1240 ultrasonic (Industrial IP67) + TFmini-S-I LiDAR |

### 3.6 Communication Links
| Link | Technology | Range | Data Rate | Purpose |
|------|-----------|-------|-----------|---------|
| Primary | LoRa 868MHz (SX1276) | 2-5 km | 0.3-50 kbps | Telemetry & commands |
| Secondary | BLE 5.0 | 100m | 2 Mbps | Close-range config/debug |
| Tertiary | WiFi 802.11n | 50m | 72 Mbps | Firmware updates, data dump |
| Sensor bus | RS485 (Modbus RTU) | 1200m | 9600-115200 baud | Soil sensor array |
| Internal | CAN 2.0B | N/A | 1 Mbps | Motor controllers |

---

## 4. Non-Functional Requirements

| Category | Requirement |
|----------|-------------|
| Operating temperature | -10°C to 55°C |
| Ingress protection | IP65 (dust-tight, water jet protected) |
| Battery life (robot) | 6-8 hours continuous operation |
| Battery life (controller) | 12+ hours |
| Weight (robot) | <25 kg (without battery), <35 kg loaded |
| Charging time | <3 hours (fast charge), solar trickle continuous |
| MTBF | >5000 hours |
| Firmware update | OTA via WiFi or USB-C |

---

## 5. Software Architecture

### 5.1 Robot Firmware (STM32H743 - Rust)
```
┌─────────────────────────────────────────────────────┐
│                 Application Layer                    │
│  ┌──────────┐ ┌──────────┐ ┌───────────────────┐   │
│  │ Mission  │ │ Health   │ │ Deseeding         │   │
│  │ Planner  │ │ Monitor  │ │ Controller        │   │
│  └──────────┘ └──────────┘ └───────────────────┘   │
├─────────────────────────────────────────────────────┤
│                 Middleware Layer                      │
│  ┌──────────┐ ┌──────────┐ ┌───────────────────┐   │
│  │ Sensor   │ │ Nav/     │ │ Communication     │   │
│  │ Fusion   │ │ Path Plan│ │ Manager           │   │
│  └──────────┘ └──────────┘ └───────────────────┘   │
├─────────────────────────────────────────────────────┤
│                    HAL Layer                          │
│  ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐ │
│  │UART │ │SPI  │ │I2C  │ │CAN  │ │ADC  │ │PWM  │ │
│  └─────┘ └─────┘ └─────┘ └─────┘ └─────┘ └─────┘ │
├─────────────────────────────────────────────────────┤
│          Embassy-rs Async Runtime (RTOS)             │
└─────────────────────────────────────────────────────┘
```

### 5.2 Controller Firmware (ESP32-S3 - Rust)
```
┌─────────────────────────────────────────────────────┐
│                 Application Layer                    │
│  ┌──────────┐ ┌──────────┐ ┌───────────────────┐   │
│  │ UI/      │ │ Telemetry│ │ Robot Command      │   │
│  │ Display  │ │ Viewer   │ │ Interface          │   │
│  └──────────┘ └──────────┘ └───────────────────┘   │
├─────────────────────────────────────────────────────┤
│                 Middleware Layer                      │
│  ┌──────────┐ ┌──────────┐ ┌───────────────────┐   │
│  │ Protocol │ │ Data     │ │ Map/Waypoint       │   │
│  │ Handler  │ │ Logger   │ │ Manager            │   │
│  └──────────┘ └──────────┘ └───────────────────┘   │
├─────────────────────────────────────────────────────┤
│                    HAL Layer                          │
│  ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐         │
│  │SPI  │ │I2C  │ │UART │ │ADC  │ │GPIO │         │
│  └─────┘ └─────┘ └─────┘ └─────┘ └─────┘         │
├─────────────────────────────────────────────────────┤
│           esp-hal / esp-wifi Runtime                  │
└─────────────────────────────────────────────────────┘
```

---

## 6. Communication Protocol

### 6.1 LoRa Packet Format
```
┌──────┬──────┬──────┬────────┬─────────┬──────┬──────┐
│ SYNC │ SRC  │ DST  │ MSG_ID │ PAYLOAD │ SEQ  │ CRC  │
│ 2B   │ 1B   │ 1B   │ 1B     │ 0-200B  │ 2B   │ 2B   │
└──────┴──────┴──────┴────────┴─────────┴──────┴──────┘
```

### 6.2 Message Types
| ID | Type | Direction | Description |
|----|------|-----------|-------------|
| 0x01 | HEARTBEAT | Robot→Ctrl | Status alive signal (1Hz) |
| 0x02 | TELEMETRY | Robot→Ctrl | Full sensor telemetry (5Hz) |
| 0x03 | PLANT_HEALTH | Robot→Ctrl | NDVI, leaf temp, disease (1Hz) |
| 0x04 | SOIL_DATA | Robot→Ctrl | NPK, pH, moisture, EC (0.2Hz) |
| 0x05 | NAV_STATUS | Robot→Ctrl | Position, heading, speed (10Hz) |
| 0x10 | MANUAL_CMD | Ctrl→Robot | Joystick drive commands |
| 0x11 | WAYPOINT | Ctrl→Robot | Set navigation waypoint |
| 0x12 | MODE_CMD | Ctrl→Robot | Auto/Manual/Pause/RTH |
| 0x13 | CONFIG | Ctrl→Robot | Configuration parameters |
| 0x20 | ACK | Both | Acknowledgment |
| 0x21 | NACK | Both | Negative acknowledgment |
| 0xFE | EMERGENCY | Both | Emergency stop |
| 0xFF | OTA_DATA | Ctrl→Robot | Firmware update chunk |

---

## 7. Safety Systems

1. **Emergency Stop**: Hardware interrupt (NMI) on robot, dedicated button on controller
2. **Geofence**: NavIC-based virtual boundary, robot stops at perimeter
3. **Obstacle Detection**: Ultrasonic + LiDAR fusion, auto-stop at 0.5m
4. **Watchdog Timer**: Hardware WDT (IWDG) resets MCU if firmware hangs
5. **Communication Loss**: Auto-stop after 3s of no heartbeat from controller
6. **Battery Protection**: BMS with under-voltage lockout, over-current protection
7. **Motor Current Limiting**: Per-motor current sensing with software cutoff
8. **Tilt Protection**: IMU-based rollover detection, motors disabled >30° tilt
