# ADR-1 Bill of Materials (BOM)

## Part 1: Robot BOM

### 1.1 Main Processing & Computing

| # | Component | Part Number | Qty | Unit Cost (₹) | Total (₹) | Supplier |
|---|-----------|-------------|-----|---------------|-----------|----------|
| 1 | STM32H743VIT6 MCU (480MHz, 2MB Flash) | STM32H743VIT6 | 1 | 850 | 850 | Mouser/Element14 |
| 2 | STM32H743 LQFP-100 breakout (dev phase) | Custom PCB | 1 | 200 | 200 | JLCPCB |
| 3 | 8MHz Crystal Oscillator (HSE) | ABM8-8.000MHZ | 1 | 25 | 25 | LCSC |
| 4 | 32.768kHz Crystal (LSE/RTC) | ABS07-32.768KHZ | 1 | 15 | 15 | LCSC |
| 5 | 16MB QSPI Flash (W25Q128) | W25Q128JVSIQ | 1 | 120 | 120 | LCSC |
| 6 | 8MB SDRAM (IS42S16400J) Industrial | IS42S16400J-7TLI | 1 | 220 | 220 | Mouser |
| 7 | microSD Card Slot | DM3AT-SF-PEJM5 | 1 | 45 | 45 | LCSC |
| 8 | 32GB microSD Card (Industrial) | SFSD032GL2AM1TO | 1 | 450 | 450 | Mouser |

### 1.2 Navigation & Positioning

| # | Component | Part Number | Qty | Unit Cost (₹) | Total (₹) | Supplier |
|---|-----------|-------------|-----|---------------|-----------|----------|
| 9 | NavIC+GPS+GLONASS GNSS Module | u-blox NEO-M9N | 1 | 2,200 | 2,200 | u-blox/Mouser |
| 10 | Active GNSS Antenna (L1+L5) | Taoglas CGGP.25.4.A.02 | 1 | 800 | 800 | Mouser |
| 11 | U.FL to SMA Pigtail (100mm) | - | 1 | 80 | 80 | Amazon |
| 12 | BNO055 9-DOF IMU Module | BNO055 | 1 | 950 | 950 | Adafruit/Mouser |
| 13 | Magnetic Encoder (4096 CPR, AEC-Q100) | ams AS5047P-ATSM | 4 | 450 | 1,800 | Mouser |

### 1.3 Communication Modules

| # | Component | Part Number | Qty | Unit Cost (₹) | Total (₹) | Supplier |
|---|-----------|-------------|-----|---------------|-----------|----------|
| 14 | LoRa Module SX1276 (868MHz) | RFM96W-868S2 | 1 | 350 | 350 | LCSC |
| 15 | SMA Antenna 868MHz (λ/4 GP) | - | 1 | 200 | 200 | Amazon |
| 16 | ESP32-S3-WROOM-1 (WiFi+BLE) | ESP32-S3-WROOM-1-N16R8 | 1 | 450 | 450 | LCSC |
| 17 | ESP32-S3 Antenna (PCB trace) | Integrated | 0 | 0 | 0 | - |
| 18 | MAX3485 RS485 Transceiver | MAX3485ESA+ | 2 | 120 | 240 | LCSC |
| 19 | MCP2562FD CAN Transceiver | MCP2562FD-E/SN | 1 | 85 | 85 | LCSC |

### 1.4 Plant Health Sensors

| # | Component | Part Number | Qty | Unit Cost (₹) | Total (₹) | Supplier |
|---|-----------|-------------|-----|---------------|-----------|----------|
| 20 | AR0234CS Camera Module (Red filter) | AR0234CSSC00SUKA0-DRBR | 1 | 1,200 | 1,200 | Mouser |
| 21 | AR0234CS Camera Module (NIR filter) | AR0234CSSC00SUKA0-DRBR | 1 | 1,200 | 1,200 | Mouser |
| 22 | Hoya R-62 Red Filter (25mm) | R-62 | 1 | 400 | 400 | B&H Photo |
| 23 | Hoya IR-72 NIR Filter (25mm) | IR-72 | 1 | 450 | 450 | B&H Photo |
| 24 | MLX90614ESF-BAA IR Thermometer | MLX90614ESF-BAA | 1 | 550 | 550 | Mouser |
| 25 | TFmini-S-I LiDAR (Industrial) | TFmini-S-I | 1 | 3,500 | 3,500 | Benewake |

### 1.5 Soil Health Sensors

| # | Component | Part Number | Qty | Unit Cost (₹) | Total (₹) | Supplier |
|---|-----------|-------------|-----|---------------|-----------|----------|
| 26 | RS485 NPK Sensor (N,P,K) | JXBS-3001-NPK-RS | 1 | 3,500 | 3,500 | AliExpress |
| 27 | RS485 Soil pH Sensor | JXBS-3001-PH-RS | 1 | 2,800 | 2,800 | AliExpress |
| 28 | RS485 Soil EC Sensor | JXBS-3001-EC-RS | 1 | 2,500 | 2,500 | AliExpress |
| 29 | Capacitive Soil Moisture Probe | SEN0193 v2.0 | 3 | 250 | 750 | DFRobot |
| 30 | DS18B20 Waterproof Probe (1m) | DS18B20-WP | 2 | 150 | 300 | Amazon |

### 1.6 Environmental Sensors

| # | Component | Part Number | Qty | Unit Cost (₹) | Total (₹) | Supplier |
|---|-----------|-------------|-----|---------------|-----------|----------|
| 31 | BME280 (Temp/Humidity/Pressure) | BME280 | 1 | 350 | 350 | Mouser |
| 32 | BH1750 Light Sensor | BH1750FVI | 1 | 120 | 120 | LCSC |
| 33 | VEML6075 UV Sensor | VEML6075 | 1 | 180 | 180 | Mouser |
| 34 | Anemometer (3-cup, pulse output) | WH-SP-WS01 | 1 | 1,200 | 1,200 | Amazon |
| 35 | Tipping Bucket Rain Gauge | WH-SP-RG | 1 | 800 | 800 | Amazon |

### 1.7 Obstacle Detection

| # | Component | Part Number | Qty | Unit Cost (₹) | Total (₹) | Supplier |
|---|-----------|-------------|-----|---------------|-----------|----------|
| 36 | MaxBotix Industrial Ultrasonic (IP67) | MB1240 XL-MaxSonar-EZ4 | 4 | 2,500 | 10,000 | Mouser |

### 1.8 Motor & Drive System

| # | Component | Part Number | Qty | Unit Cost (₹) | Total (₹) | Supplier |
|---|-----------|-------------|-----|---------------|-----------|----------|
| 37 | BLDC Motor 24V 150W (with gearbox) | MY1016Z2 | 4 | 2,500 | 10,000 | AliExpress |
| 38 | BLDC Motor Driver (FOC, CAN bus) | ODrive S1 (or SimpleFOC) | 4 | 3,500 | 14,000 | ODrive |
| 39 | Deseeding Rotary Motor (brushed 24V) | RS-775 | 1 | 450 | 450 | Amazon |
| 40 | Deseeding Motor Driver (H-bridge) | BTS7960 | 1 | 350 | 350 | Amazon |
| 41 | Vacuum Pump Motor (12V DC) | 370 pump | 1 | 600 | 600 | AliExpress |
| 42 | Servo Motor (probe deployment) | MG996R | 2 | 350 | 700 | Amazon |
| 43 | HTD-5M Timing Belt (500mm) | HTD5M-500-15 | 4 | 180 | 720 | Amazon |
| 44 | HTD-5M Pulley (20T, 12mm bore) | HTD5M-20T | 8 | 120 | 960 | Amazon |

### 1.9 Power System

| # | Component | Part Number | Qty | Unit Cost (₹) | Total (₹) | Supplier |
|---|-----------|-------------|-----|---------------|-----------|----------|
| 45 | LiFePO4 Battery Pack 25.6V 20Ah | Custom (8S) | 1 | 12,000 | 12,000 | Local |
| 46 | BQ76952 Battery Management IC | BQ76952PFBR | 1 | 450 | 450 | Mouser |
| 47 | 50W Solar Panel (18V) | Mono PERC | 1 | 2,500 | 2,500 | Amazon |
| 48 | MPPT Solar Charge Controller IC | LT3652 | 1 | 380 | 380 | Mouser |
| 49 | 24V→5V Buck Converter (5A) | LM2596S-5.0 | 2 | 120 | 240 | Mouser |
| 50 | 24V→3.3V Buck Converter (2A) | TPS563200DDCR | 2 | 60 | 120 | LCSC |
| 51 | 24V→12V Buck Converter (Industrial) | TPS54360BDDAR | 1 | 180 | 180 | Mouser |

### 1.10 Analog Signal Conditioning

| # | Component | Part Number | Qty | Unit Cost (₹) | Total (₹) | Supplier |
|---|-----------|-------------|-----|---------------|-----------|----------|
| 52 | ADS1115 16-bit ADC (I2C) | ADS1115IDGSR | 2 | 280 | 560 | LCSC |
| 53 | INA226 Current/Power Monitor | INA226AIDGSR | 2 | 180 | 360 | LCSC |
| 54 | ACS712-20A Hall Current Sensor | ACS712ELCTR-20A | 4 | 150 | 600 | Amazon |

### 1.11 PCB & Connectors

| # | Component | Part Number | Qty | Unit Cost (₹) | Total (₹) | Supplier |
|---|-----------|-------------|-----|---------------|-----------|----------|
| 55 | Robot Main PCB (6-layer, 150×120mm) | Custom | 5 | 800 | 4,000 | JLCPCB |
| 56 | IP67 Circular Connector 12-pin | SP13 | 4 | 250 | 1,000 | Amazon |
| 57 | IP67 Circular Connector 4-pin | SP13 | 6 | 180 | 1,080 | Amazon |
| 58 | SMA Connector (panel mount) | SMA-KFD | 2 | 50 | 100 | LCSC |
| 59 | USB-C Connector (IP67 cap) | USB4125-GF-A | 1 | 80 | 80 | LCSC |
| 60 | JST-XH 2/3/4/6 pin connectors | Various | 20 | 10 | 200 | Amazon |
| 61 | 120Ω CAN Termination Resistor | - | 2 | 2 | 4 | LCSC |
| 62 | 120Ω RS485 Termination Resistor | - | 2 | 2 | 4 | LCSC |

### 1.12 Mechanical & Structural

| # | Component | Part Number | Qty | Unit Cost (₹) | Total (₹) | Supplier |
|---|-----------|-------------|-----|---------------|-----------|----------|
| 63 | Aluminum 6061-T6 Frame Kit | Custom CNC | 1 | 8,000 | 8,000 | Local |
| 64 | ASA Body Panels (injection mold) | Custom | 1 set | 5,000 | 5,000 | Local |
| 65 | Pneumatic Wheels (200×80mm) | Agricultural | 4 | 600 | 2,400 | Amazon |
| 66 | Sealed Ball Bearing 6001-2RS | 6001-2RS | 8 | 60 | 480 | Amazon |
| 67 | Coil Spring (suspension) | Custom | 4 | 200 | 800 | Local |
| 68 | Carbon Fiber Tube (20mm sensor mast) | CF-20 | 1 | 400 | 400 | Amazon |
| 69 | EPDM Gasket Material (1m roll) | 3mm thick | 1 | 300 | 300 | Amazon |
| 70 | IP68 Cable Glands (assorted) | PG7/PG9/PG11 | 15 | 30 | 450 | Amazon |
| 71 | A4-80 SS Fastener Kit | M3/M4/M5 assorted | 1 | 800 | 800 | Amazon |
| 72 | Deseeding Blade Set (Boron Steel) | Custom | 1 set | 2,000 | 2,000 | Local |

### Robot BOM Subtotal

| Category | Cost (₹) |
|----------|---------|
| Processing & Computing | 1,925 |
| Navigation & Positioning | 5,830 |
| Communication Modules | 1,325 |
| Plant Health Sensors | 6,550 |
| Soil Health Sensors | 9,850 |
| Environmental Sensors | 2,650 |
| Obstacle Detection | 10,000 |
| Motor & Drive System | 27,780 |
| Power System | 15,870 |
| Analog Signal Conditioning | 1,520 |
| PCB & Connectors | 6,468 |
| Mechanical & Structural | 20,630 |
| **ROBOT TOTAL** | **₹110,398** |
| **USD Equivalent (~₹83)** | **~$1,330** |

---

## Part 2: Handheld Controller BOM

### 2.1 Main Processing

| # | Component | Part Number | Qty | Unit Cost (₹) | Total (₹) | Supplier |
|---|-----------|-------------|-----|---------------|-----------|----------|
| 1 | ESP32-S3-WROOM-1-N16R8 | ESP32-S3-WROOM-1 | 1 | 450 | 450 | LCSC |
| 2 | 32.768kHz Crystal (RTC) | ABS07-32.768KHZ | 1 | 15 | 15 | LCSC |

### 2.2 Display

| # | Component | Part Number | Qty | Unit Cost (₹) | Total (₹) | Supplier |
|---|-----------|-------------|-----|---------------|-----------|----------|
| 3 | 3.5" TFT LCD 480×320 Industrial | NHD-3.5-320240MF-ATXI#-1 | 1 | 2,200 | 2,200 | Mouser |
| 4 | Gorilla Glass 3 cover lens | Custom | 1 | 500 | 500 | Local |

### 2.3 Communication

| # | Component | Part Number | Qty | Unit Cost (₹) | Total (₹) | Supplier |
|---|-----------|-------------|-----|---------------|-----------|----------|
| 5 | LoRa Module SX1276 (868MHz) | RFM96W-868S2 | 1 | 350 | 350 | LCSC |
| 6 | NavIC GNSS Module | u-blox MAX-M10S | 1 | 1,400 | 1,400 | Mouser |
| 7 | GNSS Chip Antenna | Taoglas PC104 | 1 | 350 | 350 | Mouser |
| 8 | BLE 5.0 (integrated in ESP32-S3) | - | 0 | 0 | 0 | - |
| 9 | WiFi (integrated in ESP32-S3) | - | 0 | 0 | 0 | - |

### 2.4 Input Devices

| # | Component | Part Number | Qty | Unit Cost (₹) | Total (₹) | Supplier |
|---|-----------|-------------|-----|---------------|-----------|----------|
| 10 | Analog Joystick (Alps RKJXV) | RKJXV122400R | 2 | 180 | 360 | Mouser |
| 11 | Tactile Buttons (6mm, waterproof) | TS-1187A | 8 | 15 | 120 | LCSC |
| 12 | Emergency Stop Button (mushroom) | LA38-11ZS | 1 | 180 | 180 | Amazon |
| 13 | Mode Selector (rotary, 4-pos) | SR1712F-0104 | 1 | 60 | 60 | LCSC |

### 2.5 Feedback

| # | Component | Part Number | Qty | Unit Cost (₹) | Total (₹) | Supplier |
|---|-----------|-------------|-----|---------------|-----------|----------|
| 14 | Haptic Motor (LRA) | LRA-0832 | 1 | 120 | 120 | Mouser |
| 15 | DRV2605L Haptic Driver | DRV2605LDGSR | 1 | 150 | 150 | LCSC |
| 16 | Piezo Buzzer (3V, SMD) | CMT-8540S | 1 | 30 | 30 | LCSC |
| 17 | Status LEDs (RGB, Industrial) | APA102-2020-8 | 4 | 25 | 100 | LCSC |

### 2.6 Power

| # | Component | Part Number | Qty | Unit Cost (₹) | Total (₹) | Supplier |
|---|-----------|-------------|-----|---------------|-----------|----------|
| 18 | LiPo Battery 3.7V 3000mAh (Industrial) | 103565 | 1 | 350 | 350 | Amazon |
| 19 | MCP73871 Charge + Load Share | MCP73871-2CCI | 1 | 120 | 120 | Mouser |
| 21 | 3.3V LDO (500mA, low noise) | AP2112K-3.3 | 2 | 15 | 30 | LCSC |
| 22 | TPS63020 Buck-Boost (3.3V, 4A) | TPS63020DSJR | 1 | 180 | 180 | LCSC |
| 23 | USB-C Connector | USB4125-GF-A | 1 | 80 | 80 | LCSC |

### 2.7 Storage

| # | Component | Part Number | Qty | Unit Cost (₹) | Total (₹) | Supplier |
|---|-----------|-------------|-----|---------------|-----------|----------|
| 24 | microSD Card Slot | DM3AT-SF-PEJM5 | 1 | 45 | 45 | LCSC |
| 25 | 16GB microSD Card | SanDisk Industrial | 1 | 250 | 250 | Amazon |

### 2.8 PCB & Enclosure

| # | Component | Part Number | Qty | Unit Cost (₹) | Total (₹) | Supplier |
|---|-----------|-------------|-----|---------------|-----------|----------|
| 26 | Controller PCB (4-layer, 140×80mm) | Custom | 5 | 400 | 2,000 | JLCPCB |
| 27 | PC/ABS Housing (upper) | Custom injection | 1 | 1,200 | 1,200 | Local |
| 28 | PA6-GF Housing (lower) | Custom injection | 1 | 1,200 | 1,200 | Local |
| 29 | TPE Grip Overmold | Custom | 2 | 300 | 600 | Local |
| 30 | Silicone Button Pad | Custom | 1 | 400 | 400 | Local |

### Controller BOM Subtotal

| Category | Cost (₹) |
|----------|---------|
| Main Processing | 465 |
| Display | 2,700 |
| Communication | 2,100 |
| Input Devices | 720 |
| Feedback | 400 |
| Power | 730 |
| Storage | 295 |
| PCB & Enclosure | 5,400 |
| **CONTROLLER TOTAL** | **₹13,225** |
| **USD Equivalent (~₹83)** | **~$159** |

---

## Part 3: Combined Project Cost Summary

| Item | Cost (₹) | Cost ($) |
|------|---------|---------|
| Robot (1 unit) | 110,398 | ~1,330 |
| Controller (1 unit) | 13,225 | ~159 |
| Assembly & Testing (est.) | 15,000 | ~181 |
| Miscellaneous (wires, tools, consumables) | 5,000 | ~60 |
| **TOTAL PROJECT COST** | **₹143,623** | **~$1,730** |

---

## Part 4: Tool & Equipment Requirements

| Tool | Purpose | Cost (₹) |
|------|---------|---------|
| Soldering Station (Hakko FX-888D) | SMD soldering | 8,000 |
| Hot Air Station | BGA/QFP rework | 3,500 |
| Digital Multimeter (Fluke 87V) | Testing | 12,000 |
| Logic Analyzer (Saleae Logic 8) | Protocol debug | 15,000 |
| ST-Link V3 | STM32 programming/debug | 3,000 |
| ESP-Prog | ESP32 programming/debug | 800 |
| Oscilloscope (Rigol DS1054Z) | Signal analysis | 25,000 |
| 3D Printer (Creality Ender 3) | Prototyping enclosures | 15,000 |
| **TOOLS TOTAL** | | **₹82,300** |

*Note: Tools are one-time investment, not per-unit cost.*
