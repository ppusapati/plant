# ADR-1 Circuit Diagrams & Schematics

## 1. Robot Main Board - Power Supply Section

```
                              ROBOT POWER DISTRIBUTION
═══════════════════════════════════════════════════════════════════════════

 LiFePO4 Battery Pack (25.6V / 20Ah)
 ┌──────────────────────┐
 │  8S LiFePO4          │
 │  ┌──┐┌──┐┌──┐┌──┐   │     ┌─────────────────┐
 │  │C1││C2││C3││C4│   │     │  BQ76952        │
 │  └──┘└──┘└──┘└──┘   │     │  Battery Mgmt   │
 │  ┌──┐┌──┐┌──┐┌──┐   ├─────┤  IC             │
 │  │C5││C6││C7││C8│   │Cell │  - Cell balance  │
 │  └──┘└──┘└──┘└──┘   │Lines│  - OVP/UVP      │
 │                      │(8)  │  - OCP/SCP      │
 │  Pack+  Pack-        │     │  - Temp monitor  │
 └───┬──────┬───────────┘     │  I2C→STM32      │
     │      │                  └─────────────────┘
     │      │
     │   ┌──┴──┐ 60A Fuse
     │   │FUSE │
     │   └──┬──┘
     │      │
     │   ┌──┴──────────────┐  E-STOP (NC relay)
     │   │  EMERGENCY STOP │
     │   │  Relay (60A)    │
     │   └──┬──────────────┘
     │      │
     ├──────┤  Main Bus: 25.6V
     │      │
     │      ├──────────────────────────────────────────────────────┐
     │      │                                                      │
     │  ┌───┴───────────┐  ┌───────────────────┐  ┌──────────┐   │
     │  │ INA226        │  │ MPPT Solar Charge │  │ Solar    │   │
     │  │ Current/Power │  │ LT3652            │  │ Panel    │   │
     │  │ Monitor       │  │ Vin: 18V panel    │  │ 50W 18V  │   │
     │  │ I2C→STM32     │  │ Vbat: 25.6V      │  │          │   │
     │  └───┬───────────┘  │ Charge: 2A max    │  └──────────┘   │
     │      │              └───────────────────┘                   │
     │      │                                                      │
     │      ├──────────────┬──────────────┬───────────────┐       │
     │      │              │              │               │       │
     │  ┌───▼───┐     ┌────▼────┐   ┌───▼───┐     ┌────▼────┐  │
     │  │24V Bus│     │12V      │   │5V     │     │3.3V     │  │
     │  │Direct │     │TPS54360B│   │LM2596S│     │TPS563200│  │
     │  │to BLDC│     │Buck     │   │Buck   │     │Buck     │  │
     │  │Drivers│     │24→12V   │   │24→5V  │     │5→3.3V   │  │
     │  │       │     │3.5A Ind │   │5A Ind │     │2A Ind   │  │
     │  └───┬───┘     └────┬────┘   └───┬───┘     └────┬────┘  │
     │      │              │              │               │       │
     │      │         ┌────┘         ┌────┘          ┌────┘      │
     │      │         │              │               │            │
     │      ▼         ▼              ▼               ▼            │
     │  4×BLDC     Vacuum       STM32H743      ESP32-S3         │
     │  Motors     Pump         Sensors         LoRa SX1276     │
     │  Deseeder   Servo        ADCs            BLE/WiFi         │
     │  Motor      Motors       RS485           GNSS             │
     │                                                            │
     └────────────────────────────────────────────────────────────┘
```

---

## 2. Robot Main Board - MCU Core Section

```
                         STM32H743VIT6 MCU CONNECTIONS
═══════════════════════════════════════════════════════════════════════════

                                    VDD=3.3V
                                      │
                             ┌────────┴────────┐
                             │  100nF ×10      │ Decoupling per VDD pin
                             │  (ceramic)       │
                             └────────┬────────┘
                                      │
    8MHz HSE              ┌───────────┴───────────────────────────┐
    ┌─────┐               │                                       │
    │ XTAL├───OSC_IN      │         STM32H743VIT6                 │
    │ 8MHz├───OSC_OUT     │         (LQFP-100)                    │
    └──┬──┘               │                                       │
    2×20pF                │  Cortex-M7 @ 480MHz                   │
    to GND                │  2MB Flash, 1MB SRAM                  │
                          │  FPU, DSP, ART Accelerator            │
    32.768kHz LSE         │                                       │
    ┌─────┐               │                                       │
    │ XTAL├───OSC32_IN    │  ┌────────────────────────────────┐  │
    │32.768├──OSC32_OUT   │  │ Pin Assignments:                │  │
    └──┬──┘               │  │                                │  │
    2×6.8pF               │  │ PA0-PA7:  UART/TIM/ADC         │  │
    to GND                │  │ PA8-PA15: USB/TIM/SPI           │  │
                          │  │ PB0-PB7:  I2C/SPI/TIM          │  │
    NRST ─── 100nF ── GND│  │ PB8-PB15: CAN/I2C/TIM          │  │
          │               │  │ PC0-PC7:  ADC/SPI/SDMMC        │  │
          10kΩ to VDD     │  │ PC8-PC15: SDMMC/UART           │  │
                          │  │ PD0-PD7:  CAN/UART/FMC         │  │
    BOOT0 ── 10kΩ ── GND │  │ PD8-PD15: UART/FMC             │  │
                          │  │ PE0-PE15: FMC/TIM/DCMI         │  │
                          │  │                                │  │
                          │  └────────────────────────────────┘  │
                          └───────────────────────────────────────┘

    Debug/Programming:
    ┌──────────┐
    │ ST-LINK  │
    │ V3 (SWD) │
    │          │
    │ SWDIO ───┼──── PA13
    │ SWCLK ───┼──── PA14
    │ SWO   ───┼──── PB3
    │ NRST  ───┼──── NRST
    │ GND   ───┼──── GND
    │ 3.3V  ───┼──── VDD (target detect)
    └──────────┘
```

---

## 3. Robot - Peripheral Connections Detail

```
                    STM32H743 PERIPHERAL MAP
═══════════════════════════════════════════════════════════════════════════

 ┌──────────────────────────────────────────────────────────────────────┐
 │                        STM32H743VIT6                                  │
 │                                                                      │
 │ UART1 (PA9/PA10) ─────────────── NavIC GNSS Module (TX/RX)          │
 │   + PPS input (PA0, TIM2_CH1)    115200 baud                        │
 │                                                                      │
 │ UART2 (PD5/PD6) ──┬── MAX3485 ── RS485 Bus (Soil Sensors)          │
 │   + DE/RE (PD4)    │   9600 baud, Modbus RTU                        │
 │                     │                                                │
 │ UART3 (PB10/PB11) ──── ESP32-S3 (AT commands / custom protocol)     │
 │                         921600 baud                                  │
 │                                                                      │
 │ UART5 (PD2/PC12) ─────── TFmini-S-I LiDAR (Industrial)             │
 │                           115200 baud                                │
 │                                                                      │
 │ SPI1 (PA5/PA6/PA7) ────── SX1276 LoRa Module                       │
 │   + NSS (PA4)              ┌────────────────┐                        │
 │   + DIO0 (PC4, EXTI)      │ SX1276         │                        │
 │   + DIO1 (PC5, EXTI)      │ SCK  ← PA5    │                        │
 │   + RESET (PB0)           │ MISO → PA6    │                        │
 │                            │ MOSI ← PA7    │                        │
 │                            │ NSS  ← PA4    │                        │
 │                            │ DIO0 → PC4    │                        │
 │                            │ RST  ← PB0    │                        │
 │                            └────────────────┘                        │
 │                                                                      │
 │ SPI2 (PB13/PB14/PB15) ─── QSPI Flash (W25Q128)                    │
 │   + CS (PB12)              16MB external storage                    │
 │                                                                      │
 │ I2C1 (PB6/PB7) ──── Sensor Bus 1 (400kHz)                          │
 │   ┌──────────────────────────────────────────────┐                   │
 │   │  BME280(0x76) ── BNO055(0x28) ── BH1750(0x23)│                  │
 │   │  MLX90614(0x5A) ── VEML6075(0x10)             │                  │
 │   └──────────────────────────────────────────────┘                   │
 │   4.7kΩ pull-ups to 3.3V on SDA/SCL                                │
 │                                                                      │
 │ I2C2 (PB10/PB11 alt) ── Sensor Bus 2 (100kHz)                      │
 │   ┌──────────────────────────────────────────────┐                   │
 │   │  ADS1115(0x48) ── INA226(0x40) ── INA226(0x41)│                 │
 │   └──────────────────────────────────────────────┘                   │
 │                                                                      │
 │ CAN1 (PD0/PD1) ──── MCP2562FD ──── CAN Bus                        │
 │   ┌──────────────┐    ┌───────────┐                                 │
 │   │ CAN_TX (PD1)│───│TXD     CANH├──── CAN_H ── 120Ω ──┐         │
 │   │ CAN_RX (PD0)│───│RXD     CANL├──── CAN_L ────────── │         │
 │   └──────────────┘    │  MCP2562  │                 ┌─────┘         │
 │                       │  VDD=3.3V │                 │ To Motor      │
 │                       └───────────┘                 │ Controllers   │
 │                                                     │ & BMS         │
 │ SPI4 (PE2-5, AS5047P) ── Magnetic Encoder FL (AEC-Q100)             │
 │ SPI5 (PF7-9, AS5047P) ── Magnetic Encoder FR (-40°C to +125°C)    │
 │ SPI6 (PG12-14, AS5047P)── Magnetic Encoder RL (4096 CPR)           │
 │ TIM4 (PB6, ABI mode)  ── Magnetic Encoder RR (AS5047P ABI out)    │
 │                                                                      │
 │ TIM8 (PC6/PC7/PC8/PC9) ── PWM for Servo Motors                     │
 │   CH1: Probe deploy servo                                           │
 │   CH2: Probe tilt servo                                             │
 │                                                                      │
 │ Analog/UART (MaxBotix MB1240 Industrial IP67, -40°C to +85°C):      │
 │   PE0: Sensor 1 Analog (Front)                                      │
 │   PE2: Sensor 2 Analog (Right)                                      │
 │   PE4: Sensor 3 Analog (Rear)                                       │
 │   PE6: Sensor 4 Analog (Left)                                       │
 │                                                                      │
 │ DCMI (PE0-PE15 shared) ── AR0234CS Camera (NDVI, Industrial)       │
 │   D0-D7, HSYNC, VSYNC, PCLK, XCLK                                 │
 │                                                                      │
 │ 1-Wire (PB1, software bit-bang) ── DS18B20 Soil Temp               │
 │                                                                      │
 │ ADC1 (PA0-PA3) ── Analog inputs via ADS1115 (I2C ADC preferred)    │
 │                                                                      │
 │ GPIO Status:                                                        │
 │   PC0: LED_STATUS (Green)                                           │
 │   PC1: LED_ERROR (Red)                                              │
 │   PC2: LED_COMMS (Blue)                                             │
 │   PC3: BUZZER (PWM)                                                 │
 │   PD3: E-STOP_SENSE (input, pull-up)                                │
 │   PD7: RELAY_CTRL (output, motor power relay)                       │
 └──────────────────────────────────────────────────────────────────────┘
```

---

## 4. LoRa SX1276 Detailed Circuit

```
                    SX1276 LoRa Module Circuit
═══════════════════════════════════════════════════════════════════════════

    3.3V ──┬───────────────────────────────────────────┐
           │                                           │
         100nF                                       100nF
           │                                           │
           ├───────────── VCC ─────┐                   │
           │                       │                   │
           │              ┌────────┴────────┐          │
    PA5 ───┤── SCK  ─────│ SCK             │          │
    PA6 ───┤── MISO ─────│ MISO   SX1276  │          │
    PA7 ───┤── MOSI ─────│ MOSI   RFM96W  │          │
    PA4 ───┤── NSS  ─────│ NSS            │          │
           │              │                │          │
    PC4 ───┤── DIO0 ─────│ DIO0 (TxDone/ │          │
           │              │       RxDone)  │          │
    PC5 ───┤── DIO1 ─────│ DIO1 (Timeout)│          │
           │              │                │          │
    PB0 ───┤── RESET ────│ NRESET        │          │
           │              │                │          │
           │              │           ANT ─┼──── SMA ──── Antenna
           │              │                │     50Ω
           │              │           GND ─┼──── GND
           │              └────────────────┘
           │
          GND

    Matching Network (if using bare SX1276 IC):
    ┌─────────────────────────────────────────┐
    │  RF_OUT ── 1.2nH ──┬── 1.5pF ── GND   │
    │                     │                   │
    │                   3.9nH                 │
    │                     │                   │
    │                     ├── 1.8pF ── GND   │
    │                     │                   │
    │                   SMA Connector         │
    └─────────────────────────────────────────┘
    (Note: RFM96W module includes matching network)
```

---

## 5. RS485 Soil Sensor Interface

```
                    RS485 Interface Circuit
═══════════════════════════════════════════════════════════════════════════

    STM32 UART2                MAX3485              RS485 Bus
    ┌──────────┐          ┌────────────┐          ┌─────────────┐
    │          │          │            │          │             │
    │ PD5(TX) ─┼───────── │DI        A ├──── A ──┤─ Soil NPK   │
    │          │          │            │   │      │  (Addr: 1)  │
    │ PD6(RX) ─┼───────── │RO        B ├──── B ──┤─ Soil pH    │
    │          │          │            │   │      │  (Addr: 2)  │
    │ PD4(DE) ─┼───┬───── │DE          │   │      │─ Soil EC    │
    │          │   │      │            │   │      │  (Addr: 3)  │
    │          │   └───── │/RE         │   │      │─ Soil Moist │
    │          │          │            │   │      │  (Addr: 4)  │
    │          │          │  VCC=3.3V  │   │      └─────────────┘
    └──────────┘          └──────┬─────┘   │
                                 │          │     Termination:
                              100nF        120Ω   A ── 120Ω ── B
                                 │          │     (at both ends)
                                GND        GND

    Bias Resistors:
    VCC ── 560Ω ── A (pull-up, ensures idle state)
    GND ── 560Ω ── B (pull-down)

    ESD Protection:
    A ── TVS Diode (SMBJ6.0A) ── GND
    B ── TVS Diode (SMBJ6.0A) ── GND
```

---

## 6. CAN Bus Motor Control Interface

```
                    CAN Bus Interface Circuit
═══════════════════════════════════════════════════════════════════════════

    STM32 FDCAN1            MCP2562FD              CAN Bus
    ┌──────────┐         ┌────────────┐         ┌──────────────────┐
    │          │         │            │         │                  │
    │ PD1(TX) ─┼──────── │TXD    CANH ├──── H ──┤── Motor Drv FL  │
    │          │         │            │    │     │   (ODrive #1)   │
    │ PD0(RX) ─┼──────── │RXD    CANL ├──── L ──┤── Motor Drv FR  │
    │          │         │            │    │     │   (ODrive #2)   │
    │          │         │  VDD=3.3V  │    │     │── Motor Drv RL  │
    │          │         │  VIO=3.3V  │    │     │   (ODrive #3)   │
    │          │         │  STBY=VDD  │    │     │── Motor Drv RR  │
    └──────────┘         └──────┬─────┘    │     │   (ODrive #4)   │
                                │           │     │── BMS BQ76952   │
                             100nF        120Ω   │── Deseeder Drv  │
                                │           │     └──────────────────┘
                               GND         │
                                      Termination:
                                      CANH ── 120Ω ── CANL
                                      (at each end of bus)

    Common Mode Choke (optional, for EMI):
    CANH ──╡╞── CANH_bus
    CANL ──╡╞── CANL_bus
           CM Choke (744231091)
```

---

## 7. BLDC Motor Driver (Per Motor)

```
                    BLDC Motor Driver Circuit (Simplified)
═══════════════════════════════════════════════════════════════════════════

    24V Bus ──┬─────────────────────────────────────────────────┐
              │                                                  │
           100µF    ┌──────────────────────────────────┐        │
           63V      │        ODrive S1                  │        │
              │     │   (or custom FOC board)           │        │
              └─────┤ VIN                               │        │
                    │                                   │        │
    CAN_H ─────────┤ CANH        Motor Phase A ────────┼──── A  │
    CAN_L ─────────┤ CANL        Motor Phase B ────────┼──── B  │
                    │             Motor Phase C ────────┼──── C  │
    Encoder A ─────┤ ENC_A                              │   BLDC │
    Encoder B ─────┤ ENC_B       GND ──────────────────┼──── GND│
                    │                                   │        │
                    │  Config via CAN:                  │        │
                    │  - Current limit: 15A             │        │
                    │  - Velocity mode                  │        │
                    │  - PID: Kp=0.5, Ki=0.01          │        │
                    └──────────────────────────────────┘        │
                                                                 │
                                                                GND

    Shunt Current Sensing (if custom driver):
    Phase A ── 10mΩ ── ACS712ELCTR-20A ── ADC (Industrial -40°C to +85°C)
                       0.1V/A
```

---

## 8. Sensor Mast Wiring

```
                    Sensor Mast Wiring Diagram
═══════════════════════════════════════════════════════════════════════════

    GNSS Active Antenna ─── U.FL ─── Coax ─── SMA ─── GNSS Module
    (Top of mast)                                     (on PCB)

    Anemometer ─── 2-wire ─── PG7 Gland ──┬── VCC (5V)
    (Top of mast)                          ├── Signal → GPIO (PC6)
                                           └── GND

    Rain Gauge ─── 2-wire ─── PG7 Gland ──┬── Signal → GPIO (PC7)
    (Side of mast)                         └── GND

    Weather Shield (BME280 + BH1750 + VEML6075):
    ┌──────────────────┐
    │  I2C Breakout    │
    │  ┌──────┐        │
    │  │BME280│ ── I2C ┼── 4-wire ── PG9 Gland ── I2C1 Bus
    │  └──────┘        │   (SDA, SCL, VCC, GND)
    │  ┌──────┐        │
    │  │BH1750│ ── I2C ┤
    │  └──────┘        │
    │  ┌──────┐        │
    │  │VEML  │ ── I2C ┤
    │  │6075  │        │
    │  └──────┘        │
    └──────────────────┘
    Housed in Stevenson screen (radiation shield)
```

---

## 9. Controller - Complete Circuit

```
                    HANDHELD CONTROLLER CIRCUIT
═══════════════════════════════════════════════════════════════════════════

    USB-C ──── MCP73871-2CCI ── LiPo 3.7V ──── TPS63020 ──── 3.3V Rail
    5V input   Charge+LoadShare  3000mAh       Buck-Boost     System
               Industrial(-40/+85°C)           Industrial(-40/+85°C)

    3.3V Rail ──┬──────────────────────────────────────────────────┐
                │                                                   │
         ┌──────▼──────────────────────────────────────────┐       │
         │              ESP32-S3-WROOM-1                    │       │
         │              (N16R8: 16MB Flash, 8MB PSRAM)      │       │
         │                                                  │       │
         │  GPIO1  ── SX1276 SCK  (SPI2)                   │       │
         │  GPIO2  ── SX1276 MISO                          │       │
         │  GPIO3  ── SX1276 MOSI                          │       │
         │  GPIO4  ── SX1276 NSS                           │       │
         │  GPIO5  ── SX1276 DIO0 (interrupt)              │       │
         │  GPIO6  ── SX1276 RESET                         │       │
         │                                                  │       │
         │  GPIO7  ── ILI9488 SCK  (SPI3, 40MHz)          │       │
         │  GPIO8  ── ILI9488 MOSI                         │       │
         │  GPIO9  ── ILI9488 DC (Data/Command)            │       │
         │  GPIO10 ── ILI9488 CS                           │       │
         │  GPIO11 ── ILI9488 RST                          │       │
         │  GPIO12 ── ILI9488 LED (backlight PWM)          │       │
         │                                                  │       │
         │  GPIO13 ── NavIC GNSS TX (UART1)                │       │
         │  GPIO14 ── NavIC GNSS RX                        │       │
         │  GPIO15 ── NavIC PPS                            │       │
         │                                                  │       │
         │  GPIO16 ── Joystick L X-axis (ADC)              │       │
         │  GPIO17 ── Joystick L Y-axis (ADC)              │       │
         │  GPIO18 ── Joystick R X-axis (ADC)              │       │
         │  GPIO19 ── Joystick R Y-axis (ADC)              │       │
         │  GPIO20 ── Joystick L Button                    │       │
         │  GPIO21 ── Joystick R Button                    │       │
         │                                                  │       │
         │  GPIO35 ── Button MODE (pull-up)                │       │
         │  GPIO36 ── Button HOME (pull-up)                │       │
         │  GPIO37 ── Button L1 (pull-up)                  │       │
         │  GPIO38 ── Button L2 (pull-up)                  │       │
         │  GPIO39 ── Button R1 (pull-up)                  │       │
         │  GPIO40 ── Button R2 (pull-up)                  │       │
         │  GPIO41 ── Button MENU (pull-up)                │       │
         │  GPIO42 ── E-STOP (NMI, pull-up, active low)   │       │
         │                                                  │       │
         │  GPIO43 ── DRV2605L SDA (I2C, haptic)          │       │
         │  GPIO44 ── DRV2605L SCL                         │       │
         │                                                  │       │
         │  GPIO45 ── APA102 Data (4 LEDs, SPI, Industrial) │       │
         │  GPIO46 ── Buzzer (PWM)                         │       │
         │  GPIO47 ── Battery ADC (voltage divider)        │       │
         │  GPIO48 ── SD Card CS (SPI shared with display) │       │
         │                                                  │       │
         │  Built-in BLE 5.0                               │       │
         │  Built-in WiFi 2.4GHz                           │       │
         │  Built-in USB (GPIO19/GPIO20 for USB-Serial)    │       │
         └──────────────────────────────────────────────────┘       │
                                                                    │
    ┌─────────────────────────────────────────────────────────────┐ │
    │ Joystick Circuit (×2):                                       │ │
    │                                                              │ │
    │ VCC(3.3V) ──┐      Alps RKJXV                              │ │
    │              │    ┌──────────┐                               │ │
    │              ├────┤ VCC   X ─├──── 100nF ──┬── ADC_X        │ │
    │              │    │          │              │                │ │
    │             10kΩ  │       Y ─├──── 100nF ──┬── ADC_Y        │ │
    │              │    │          │              │                │ │
    │  GPIO ──── 10kΩ──┤ SW    GND├──── GND      │                │ │
    │  (pull-up)  │    └──────────┘              │                │ │
    │             GND                          (LP filter          │ │
    │                                           fc = 160Hz)       │ │
    └─────────────────────────────────────────────────────────────┘ │
                                                                    │
    ┌─────────────────────────────────────────────────────────────┐ │
    │ Battery Monitoring:                                          │ │
    │                                                              │ │
    │ VBAT ── 100kΩ ──┬── 100kΩ ── GND                           │ │
    │                  │                                           │ │
    │                  ├── 100nF ── GND                            │ │
    │                  │                                           │ │
    │                  └── ADC (GPIO47)                             │ │
    │                                                              │ │
    │ V_adc = VBAT × (100k / (100k+100k)) = VBAT / 2             │ │
    │ At full charge: 4.2V × 0.5 = 2.1V (within ADC range)       │ │
    └─────────────────────────────────────────────────────────────┘ │
                                                                    GND
```

---

## 10. ESD & Protection Circuits

```
                    Protection Circuits (Applied Globally)
═══════════════════════════════════════════════════════════════════════════

    USB-C ESD Protection:
    ┌──────────────────────────────────────┐
    │  USB_D+ ──┤TPD4E05U06├── GND       │
    │  USB_D- ──┤          ├── GND       │
    │  USB_CC1 ─┤          ├── GND       │
    │  USB_CC2 ─┤          ├── GND       │
    └──────────────────────────────────────┘

    Antenna ESD:
    ┌──────────────────────────────────────┐
    │  LoRa ANT ── TVS (CDSOD323-T05C) ── GND │
    │  GNSS ANT ── TVS (CDSOD323-T05C) ── GND │
    └──────────────────────────────────────┘

    Power Input Protection:
    ┌──────────────────────────────────────┐
    │  24V IN ── Schottky ── PMOS ── Load │
    │            (reverse    (overvoltage  │
    │             polarity)   crowbar)     │
    │                                      │
    │  Gate: Zener 28V + 100kΩ to source  │
    └──────────────────────────────────────┘

    Motor Driver Protection:
    ┌──────────────────────────────────────┐
    │  Motor Phase ── TVS (P6SMB27A) ── GND│
    │  (each phase)   27V clamping          │
    │                                      │
    │  Flyback diode across motor:         │
    │  Motor+ ──|◄|── Motor-              │
    │           (Schottky SS34)            │
    └──────────────────────────────────────┘
```

---

## 11. PCB Layer Stack-Up

### Robot Main Board (6-Layer)

```
Layer 1 (TOP):     Signal + Components (1.0 oz Cu)
                   ───────────────────────────────
Prepreg:           7628 (0.2mm)
                   ───────────────────────────────
Layer 2 (GND):     Ground Plane (1.0 oz Cu)
                   ───────────────────────────────
Core:              FR4 (0.3mm)
                   ───────────────────────────────
Layer 3 (SIG):     Signal routing (0.5 oz Cu)
                   ───────────────────────────────
Prepreg:           2116 (0.12mm)
                   ───────────────────────────────
Layer 4 (PWR):     Power Planes (24V, 5V, 3.3V) (0.5 oz Cu)
                   ───────────────────────────────
Core:              FR4 (0.3mm)
                   ───────────────────────────────
Layer 5 (GND):     Ground Plane (1.0 oz Cu)
                   ───────────────────────────────
Prepreg:           7628 (0.2mm)
                   ───────────────────────────────
Layer 6 (BOT):     Signal + Components (1.0 oz Cu)

Total Thickness:   ~1.6mm
Impedance:         50Ω single-ended, 100Ω differential
```

### Controller Board (4-Layer)

```
Layer 1 (TOP):     Signal + Components (1.0 oz Cu)
                   ───────────────────────────────
Prepreg:           2116 (0.12mm)
                   ───────────────────────────────
Layer 2 (GND):     Ground Plane (1.0 oz Cu)
                   ───────────────────────────────
Core:              FR4 (0.8mm)
                   ───────────────────────────────
Layer 3 (PWR):     Power Plane (3.3V) (1.0 oz Cu)
                   ───────────────────────────────
Prepreg:           2116 (0.12mm)
                   ───────────────────────────────
Layer 4 (BOT):     Signal + Components (1.0 oz Cu)

Total Thickness:   ~1.2mm
Impedance:         50Ω single-ended
```
