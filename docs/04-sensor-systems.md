# ADR-1 Sensor Systems & Health Monitoring Document

## 1. Sensor Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                    SENSOR SUBSYSTEM ARCHITECTURE                     │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                    I2C Bus 1 (400kHz)                        │   │
│  │  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐  │   │
│  │  │BME280  │ │BH1750  │ │BNO055  │ │MLX90614│ │VEML6075│  │   │
│  │  │Env.    │ │Light   │ │IMU     │ │IR Temp │ │UV      │  │   │
│  │  │0x76   │ │0x23   │ │0x28   │ │0x5A   │ │0x10   │  │   │
│  │  └────────┘ └────────┘ └────────┘ └────────┘ └────────┘  │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                    I2C Bus 2 (100kHz)                        │   │
│  │  ┌────────┐ ┌────────┐ ┌────────┐                          │   │
│  │  │ADS1115 │ │INA226  │ │INA226  │                          │   │
│  │  │ADC     │ │Batt Mon│ │Motor   │                          │   │
│  │  │0x48   │ │0x40   │ │0x41   │                          │   │
│  │  └────────┘ └────────┘ └────────┘                          │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                    RS485 Bus (Modbus RTU)                    │   │
│  │  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐              │   │
│  │  │NPK    │ │pH      │ │EC      │ │Moisture│              │   │
│  │  │Addr:1 │ │Addr:2  │ │Addr:3  │ │Addr:4  │              │   │
│  │  └────────┘ └────────┘ └────────┘ └────────┘              │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                    Dedicated Interfaces                       │   │
│  │  ┌────────────┐ ┌──────────┐ ┌────────────┐ ┌──────────┐  │   │
│  │  │NDVI Camera │ │TFmini-S  │ │HC-SR04 x4  │ │DS18B20   │  │   │
│  │  │SPI+GPIO    │ │UART      │ │GPIO Trigger │ │1-Wire    │  │   │
│  │  └────────────┘ └──────────┘ └────────────┘ └──────────┘  │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                    Analog Inputs (via ADS1115)                │   │
│  │  ┌────────────┐ ┌──────────┐ ┌────────────┐                │   │
│  │  │Anemometer  │ │Rain Gauge│ │Soil Moist. │                │   │
│  │  │Pulse→Freq  │ │Pulse Cnt │ │Capacitive  │                │   │
│  │  └────────────┘ └──────────┘ └────────────┘                │   │
│  └─────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 2. Plant Health Monitoring Sensors

### 2.1 NDVI Camera System

**Purpose**: Calculate Normalized Difference Vegetation Index to assess crop vigor.

**NDVI Formula**: `NDVI = (NIR - RED) / (NIR + RED)`

| Parameter | Specification |
|-----------|--------------|
| Implementation | Dual OV5640 cameras with optical filters |
| Red filter | Hoya R-62 (620nm bandpass) |
| NIR filter | Hoya IR-72 (720nm longpass) |
| Resolution | 2592×1944 (5MP) downsampled to 640×480 |
| Frame rate | 5 fps (synchronized) |
| Interface | DCMI (Digital Camera Interface) on STM32H7 |
| FOV | 60° diagonal (covers 400mm at 300mm height) |
| Processing | On-MCU NDVI calculation per-pixel |

```
NDVI Interpretation:
  ┌───────────────────────────────────────────────┐
  │ NDVI Value │ Interpretation                   │
  ├────────────┼──────────────────────────────────┤
  │ -1.0 - 0.0 │ Water, bare soil, dead material  │
  │  0.0 - 0.2 │ Barren rock, sand, snow          │
  │  0.2 - 0.4 │ Sparse vegetation, stressed crops│
  │  0.4 - 0.6 │ Moderate vegetation              │
  │  0.6 - 0.8 │ Dense healthy vegetation          │
  │  0.8 - 1.0 │ Very dense/healthy canopy         │
  └────────────┴──────────────────────────────────┘

Camera Mounting:
                    ┌──────────┐
                    │ Robot    │
                    │ Chassis  │
                    └────┬─────┘
                         │ 300mm below chassis
                    ┌────▼─────┐
                    │ Camera   │
                    │ Module   │ ← Downward facing
                    │ RED NIR  │
                    └──────────┘
                         │
                    ┌────▼─────┐
                    │ Ground   │ FOV: 340mm × 260mm at ground
                    │ Coverage │
                    └──────────┘
```

### 2.2 Infrared Leaf Temperature Sensor

| Parameter | Specification |
|-----------|--------------|
| Sensor | MLX90614ESF-BAA (medical grade) |
| Range | -40°C to 125°C object temperature |
| Accuracy | ±0.5°C in 0-50°C range |
| FOV | 90° (wide angle for canopy average) |
| Resolution | 0.02°C |
| Interface | I2C (SMBus compatible) |
| Update rate | 10 Hz |

**Diagnostic Use**:
- Leaf temp > Ambient + 5°C → Water stress detected
- Leaf temp < Ambient - 2°C → Active transpiration (healthy)
- Leaf temp anomaly zones → Potential disease hotspot

### 2.3 Plant Height Measurement

| Parameter | Specification |
|-----------|--------------|
| Sensor | TFmini-S LiDAR |
| Range | 0.1m to 12m |
| Accuracy | ±1cm at 0.1-6m |
| Update rate | 100 Hz (set to 10 Hz for power saving) |
| Interface | UART (115200 baud) |
| Wavelength | 850nm (eye-safe Class 1) |
| Beam divergence | 2.3° |

```
Plant Height Calculation:
                    ┌──────────┐
                    │ TFmini-S │ Mounted on chassis, downward
                    └────┬─────┘
                         │ d1 = distance to plant top
                         ▼
                    ┌──────────┐
                    │  Plant   │ h_plant = h_sensor - d1
                    │  Canopy  │
                    └──────────┘
                         │ d2 = distance to ground (calibrated)
                         ▼
                    ──────────── Ground

    Plant height = d2 - d1
    h_sensor is calibrated at startup on bare soil
```

### 2.4 Disease Detection Pipeline

```
Image Acquisition (5 fps)
         │
         ▼
┌──────────────────┐
│ Pre-processing    │
│ - White balance   │
│ - Lens correction │
│ - Crop ROI        │
└────────┬─────────┘
         │
         ▼
┌──────────────────┐     ┌──────────────────┐
│ NDVI Calculation │────►│ Health Map       │
│ Per-pixel        │     │ Color-coded grid │
└────────┬─────────┘     └──────────────────┘
         │
         ▼
┌──────────────────┐
│ Anomaly Detection│
│ - Z-score per    │
│   grid cell      │
│ - Threshold:     │
│   NDVI < 0.3     │
│   in crop zone   │
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│ Alert Generation │
│ - GPS tag        │
│ - NDVI value     │
│ - Image snapshot │
│ - Send to ctrl   │
└──────────────────┘
```

---

## 3. Soil Health Monitoring Sensors

### 3.1 Soil NPK Sensor (RS485 Modbus)

| Parameter | Specification |
|-----------|--------------|
| Model | JXBS-3001-NPK (or equivalent) |
| Nitrogen (N) | 0-1999 mg/kg, ±2% FS |
| Phosphorus (P) | 0-1999 mg/kg, ±2% FS |
| Potassium (K) | 0-1999 mg/kg, ±2% FS |
| Interface | RS485 Modbus RTU, 9600 baud |
| Power | 12-24V DC, <0.5W |
| Probe length | 70mm (insertion depth) |
| Response time | <1 second |
| Operating temp | -20°C to 60°C |
| Protection | IP68 (fully submersible) |

### 3.2 Soil pH Sensor (RS485 Modbus)

| Parameter | Specification |
|-----------|--------------|
| Range | 3.0 - 9.0 pH |
| Resolution | 0.01 pH |
| Accuracy | ±0.1 pH |
| Interface | RS485 Modbus RTU |
| Calibration | 2-point (pH 4.0 and pH 7.0 buffer) |
| Probe type | Solid-state (no glass electrode, field-rugged) |
| Lifetime | >2 years continuous soil contact |

### 3.3 Soil Moisture Sensor (Capacitive)

| Parameter | Specification |
|-----------|--------------|
| Type | FDR (Frequency Domain Reflectometry) |
| Range | 0-100% VWC (Volumetric Water Content) |
| Accuracy | ±3% VWC (calibrated for loam) |
| Depth | Probe at 100mm, 200mm, 300mm (3-level) |
| Interface | Analog output → ADS1115 16-bit ADC |
| Advantage | No corrosion (unlike resistive), long life |

### 3.4 Soil Temperature

| Parameter | Specification |
|-----------|--------------|
| Sensor | DS18B20 (waterproof, stainless steel probe) |
| Range | -55°C to 125°C |
| Accuracy | ±0.5°C (-10°C to 85°C) |
| Resolution | 12-bit (0.0625°C) |
| Interface | 1-Wire (parasitic or powered) |
| Probe length | 50mm stainless steel |
| Cable length | 1m (shielded) |

### 3.5 Soil Electrical Conductivity (EC)

| Parameter | Specification |
|-----------|--------------|
| Range | 0-20,000 µS/cm |
| Accuracy | ±2% FS |
| Interface | RS485 Modbus RTU |
| Compensation | Auto temperature compensation |
| Use | Salinity assessment, fertilizer concentration |

### 3.6 Soil Probe Deployment Mechanism

```
        ┌──────────────┐
        │  Servo Motor  │ ← MG996R (high torque, 15kg·cm)
        │  (Z-axis)     │
        └──────┬───────┘
               │
        ┌──────▼───────┐
        │ Linear Rail   │ ← 100mm travel, ball bearing
        │ (SBR12)       │
        └──────┬───────┘
               │
        ┌──────▼───────┐
        │ Probe Array   │
        │ ┌───┐┌───┐   │
        │ │NPK││pH │   │
        │ └───┘└───┘   │
        │ ┌───┐┌───┐   │
        │ │EC ││Mst│   │
        │ └───┘└───┘   │
        │ ┌───┐         │
        │ │Tmp│         │
        │ └───┘         │
        └──────────────┘

Deployment Sequence:
1. Robot stops at sampling point
2. Servo lowers probe array (2s descent)
3. Probes inserted 70-100mm into soil
4. Wait 5s for sensor stabilization
5. Read all sensors (3 readings, average)
6. Servo retracts probes
7. Robot continues to next waypoint
8. Probes cleaned by vibration motor during transit
```

---

## 4. Environmental Monitoring Sensors

### 4.1 BME280 (Temperature, Humidity, Pressure)

| Parameter | Range | Accuracy | Resolution |
|-----------|-------|----------|------------|
| Temperature | -40 to 85°C | ±1.0°C | 0.01°C |
| Humidity | 0-100% RH | ±3% RH | 0.008% RH |
| Pressure | 300-1100 hPa | ±1 hPa | 0.18 Pa |
| Interface | I2C (0x76) | | |
| Power | 3.6 µA @ 1Hz | | |
| Sampling | Forced mode, 1 Hz | | |

### 4.2 BH1750 Light Intensity

| Parameter | Specification |
|-----------|--------------|
| Range | 1-65535 lux |
| Resolution | 1 lux (H-resolution mode) |
| Accuracy | ±20% |
| Spectral response | Close to human eye (V(λ)) |
| Interface | I2C (0x23) |
| Use | PAR estimation, light stress detection |

### 4.3 VEML6075 UV Sensor

| Parameter | Specification |
|-----------|--------------|
| Channels | UVA (365nm), UVB (330nm) |
| UV Index | Calculated 0-15 |
| Interface | I2C (0x10) |
| Use | UV stress assessment on crops |

### 4.4 Wind Speed (Anemometer)

| Parameter | Specification |
|-----------|--------------|
| Type | 3-cup anemometer with reed switch |
| Range | 0-60 m/s |
| Resolution | 0.1 m/s |
| Output | Pulse frequency (1 pulse/revolution) |
| Interface | GPIO interrupt → frequency counter |
| Calibration | V(m/s) = frequency × 0.0875 + 0.35 |

### 4.5 Rainfall Sensor

| Parameter | Specification |
|-----------|--------------|
| Type | Tipping bucket rain gauge |
| Resolution | 0.2mm per tip |
| Output | Pulse (reed switch closure) |
| Interface | GPIO interrupt → pulse counter |
| Measurement | mm/hour calculated over 10-min windows |

---

## 5. Navigation & Obstacle Sensors

### 5.1 BNO055 IMU (9-DOF)

| Parameter | Specification |
|-----------|--------------|
| Accelerometer | ±2/4/8/16g, 14-bit |
| Gyroscope | ±125/250/500/1000/2000 °/s, 16-bit |
| Magnetometer | ±1300/2600 µT (XY), ±2500 µT (Z) |
| Fusion output | Quaternion, Euler angles, linear accel |
| Update rate | 100 Hz (fusion mode) |
| Interface | I2C (0x28) |
| Heading accuracy | ±2.5° (after calibration) |
| Operating mode | NDOF (Nine Degrees of Freedom fusion) |

### 5.2 Ultrasonic Obstacle Detection (HC-SR04 × 4)

```
Sensor Placement (Top View):
                    FRONT
              ┌──────US1──────┐
              │      ▲        │
              │    /   \      │
         US4 ◄│  /       \    │► US2
              │ Robot Body    │
              │               │
              │               │
              └──────US3──────┘
                    REAR

US1: Front center (0°)    - Obstacle ahead
US2: Right side (90°)     - Row tracking
US3: Rear center (180°)   - Reversing safety
US4: Left side (270°)     - Row tracking
```

| Parameter | Specification |
|-----------|--------------|
| Range | 2cm - 400cm |
| Accuracy | ±3mm |
| Beam angle | 15° cone |
| Interface | GPIO trigger + echo (timer capture) |
| Scan rate | 10 Hz per sensor (40 Hz total, round-robin) |

### 5.3 Wheel Encoders

| Parameter | Specification |
|-----------|--------------|
| Type | Incremental optical encoder |
| Resolution | 600 PPR (2400 CPR with quadrature) |
| Interface | STM32 Timer in encoder mode (TIM1-TIM4) |
| Odometry | Distance = (pulses / CPR) × π × wheel_diameter |
| Update rate | Continuous, sampled at 100 Hz |

---

## 6. Power Monitoring Sensors

### 6.1 Battery Monitoring (INA226)

| Parameter | Specification |
|-----------|--------------|
| Voltage range | 0-36V (bus), ±81.92mV (shunt) |
| Shunt resistor | 10mΩ (0.1% precision) |
| Current range | ±8.192A |
| Power calculation | On-chip (V × I) |
| Resolution | 16-bit voltage, 16-bit current |
| Interface | I2C (0x40) |
| Alert | Programmable over/under voltage/current |

### 6.2 Motor Current Sensing

| Parameter | Specification |
|-----------|--------------|
| Method | ACS712-20A Hall-effect current sensor per motor |
| Range | ±20A |
| Sensitivity | 100 mV/A |
| Interface | Analog → ADS1115 ADC |
| Use | Overcurrent protection, motor stall detection |

---

## 7. Sensor Fusion Pipeline

```
┌─────────────────────────────────────────────────────────────────┐
│                    SENSOR FUSION ENGINE                           │
│                                                                  │
│  Navigation Fusion (Extended Kalman Filter):                     │
│  ┌────────┐  ┌────────┐  ┌────────┐  ┌────────┐              │
│  │ NavIC  │  │ IMU    │  │ Wheel  │  │ LiDAR  │              │
│  │ 10Hz   │  │ 100Hz  │  │ Encoder│  │ 10Hz   │              │
│  │ pos,vel│  │ acc,gyr│  │ 100Hz  │  │ range  │              │
│  └───┬────┘  └───┬────┘  └───┬────┘  └───┬────┘              │
│      │           │           │           │                     │
│      └───────────┼───────────┼───────────┘                     │
│                  │           │                                  │
│           ┌──────▼───────────▼──────┐                          │
│           │   Extended Kalman       │                          │
│           │   Filter (EKF)          │                          │
│           │   State: [x,y,θ,v,ω]   │                          │
│           │   Output: 50Hz          │                          │
│           └─────────────────────────┘                          │
│                                                                  │
│  Health Fusion (Weighted Average):                               │
│  ┌────────┐  ┌────────┐  ┌────────┐  ┌────────┐              │
│  │ NDVI   │  │ Leaf   │  │ Soil   │  │ Environ│              │
│  │ Camera │  │ Temp   │  │ Probes │  │ Sensors│              │
│  │ 5Hz    │  │ 10Hz   │  │ 0.2Hz  │  │ 1Hz    │              │
│  └───┬────┘  └───┬────┘  └───┬────┘  └───┬────┘              │
│      │           │           │           │                     │
│      └───────────┼───────────┼───────────┘                     │
│                  │           │                                  │
│           ┌──────▼───────────▼──────┐                          │
│           │   Health Score          │                          │
│           │   Algorithm             │                          │
│           │   Output: Composite     │                          │
│           │   health index 0-100    │                          │
│           └─────────────────────────┘                          │
│                                                                  │
│  Composite Health Score Formula:                                 │
│  H = 0.30×NDVI_norm + 0.15×LeafTemp_score +                    │
│      0.20×SoilMoist_score + 0.15×NPK_score +                   │
│      0.10×pH_score + 0.10×EC_score                              │
│                                                                  │
│  Where each sub-score is normalized to 0-100 based on           │
│  crop-specific optimal ranges stored in configuration.           │
└─────────────────────────────────────────────────────────────────┘
```

---

## 8. Sensor Sampling Schedule

| Sensor | Sample Rate | Priority | Bus | Power (mW) |
|--------|------------|----------|-----|------------|
| NavIC GNSS | 10 Hz | Critical | UART4 | 30 |
| BNO055 IMU | 100 Hz | Critical | I2C1 | 12 |
| Wheel encoders | 100 Hz | Critical | TIM1-4 | 0 |
| HC-SR04 ×4 | 10 Hz each | High | GPIO | 75 |
| TFmini-S LiDAR | 10 Hz | High | UART5 | 120 |
| NDVI Camera | 5 Hz | Medium | DCMI+SPI | 250 |
| MLX90614 IR | 10 Hz | Medium | I2C1 | 8 |
| BME280 | 1 Hz | Low | I2C1 | 0.004 |
| BH1750 | 1 Hz | Low | I2C1 | 0.2 |
| VEML6075 | 1 Hz | Low | I2C1 | 0.1 |
| Anemometer | 1 Hz | Low | GPIO | 5 |
| Rain gauge | Event | Low | GPIO | 0 |
| Soil NPK | 0.2 Hz | Low | RS485 | 500 |
| Soil pH | 0.2 Hz | Low | RS485 | 300 |
| Soil EC | 0.2 Hz | Low | RS485 | 300 |
| Soil moisture | 0.2 Hz | Low | ADC | 10 |
| DS18B20 | 0.2 Hz | Low | 1-Wire | 1.5 |
| INA226 Batt | 10 Hz | High | I2C2 | 1 |
| ACS712 Motors | 100 Hz | High | ADC | 40 |
| **Total** | | | | **~1653** |
