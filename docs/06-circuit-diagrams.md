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

## 11. GNSS NEO-M9N Module Circuit

```
                    GNSS NEO-M9N Receiver Circuit
═══════════════════════════════════════════════════════════════════════════

    3.3V ──┬──────────────────────────────────────────────────────┐
           │                                                       │
         100nF  10µF                                             100nF
           │     │                                                 │
           ├─────┤                                                 │
           │     │                                                 │
           │  ┌──┴──────────────────────────────────────────┐     │
           │  │              u-blox NEO-M9N                  │     │
           │  │              (GPS+GLONASS+NavIC)              │     │
           │  │                                              │     │
    PA9 ───┤──│ TX (UART1_TX) ──── RXD                      │     │
    PA10 ──┤──│ RX (UART1_RX) ──── TXD                      │     │
           │  │                                              │     │
    PA0 ───┤──│ PPS (TIM2_CH1) ──── TIMEPULSE               │     │
           │  │                    (1Hz, rising edge)        │     │
           │  │                                              │     │
           │  │ VCC_IO ──── 3.3V                             │     │
           │  │ V_BCKP ──── 3.3V (via Schottky + 100nF)     │     │
           │  │                                              │     │
           │  │ RF_IN ─────────────────────────────────┐     │     │
           │  │                                        │     │     │
           │  │ GND (×8 thermal pads) ── GND plane     │     │     │
           │  └────────────────────────────────────────┘     │     │
           │                                                 │     │
           │                    SAW Filter                    │     │
           │  ┌──────────────────────────────────────────┐   │     │
           │  │  RF_IN ── SAW (B39162B3725U410) ── LNA   │   │     │
           │  │           (1575.42 MHz L1 band)          │   │     │
           │  └──────────────────────────────────────────┘   │     │
           │                         │                        │     │
           │                    ┌────┴────┐                  │     │
           │                    │  LNA    │                  │     │
           │                    │  (PSA4-5043+)              │     │
           │                    │  Gain: 18dB               │     │
           │                    │  NF: 0.75dB               │     │
           │                    └────┬────┘                  │     │
           │                         │                        │     │
           │                    U.FL Connector                │     │
           │                         │                        │     │
           │                    Coax Cable (50Ω)             │     │
           │                         │                        │     │
           │                    SMA Panel Mount              │     │
           │                         │                        │     │
           │                    Active GNSS Antenna           │     │
           │                    (Taoglas CGGP.25.4.A.02)     │     │
           │                    L1+L5 Band                    │     │
           │                                                  │     │
           │  Backup Battery:                                 │     │
           │  V_BCKP ── BAT54S ──┬── CR1220 (3V coin cell)  │     │
           │                      │                           │     │
           │                   100nF                          │     │
           │                      │                           │     │
          GND                   GND                         GND   GND

    Configuration (via UBX protocol at 115200 baud):
    ┌──────────────────────────────────────────┐
    │  GNSS Constellations: GPS + GLONASS +    │
    │  NavIC + Galileo + BeiDou                │
    │  Update Rate: 25Hz (single constellation)│
    │               10Hz (multi-constellation)  │
    │  TIMEPULSE: 1PPS, 100ns accuracy         │
    │  Protocol: UBX binary + NMEA 4.11        │
    │  SBAS: GAGAN (Indian SBAS)               │
    └──────────────────────────────────────────┘
```

---

## 12. BNO055 9-DOF IMU Circuit

```
                    BNO055 Inertial Measurement Unit Circuit
═══════════════════════════════════════════════════════════════════════════

    3.3V ──┬──────────────────────────────────────────────────┐
           │                                                   │
         100nF  100nF  4.7µF                                 │
           │     │      │                                      │
           ├─────┤──────┤                                      │
           │                                                   │
           │     ┌──────────────────────────────────┐          │
           ├─────┤ VDD                    BNO055    │          │
           │     │                    (LGA-28 pkg)   │          │
           │     │                                   │          │
    PB6 ───┤──── │ SDA (I2C1)   Address: 0x28       │          │
    PB7 ───┤──── │ SCL (I2C1)   (COM3=LOW → 0x28)   │          │
           │     │              (COM3=HIGH → 0x29)  │          │
           │     │                                   │          │
           │     │ INT ─── PC13 (EXTI, active LOW)  │          │
           │     │         10kΩ pull-up to 3.3V      │          │
           │     │                                   │          │
           │     │ NRESET ── PB2                     │          │
           │     │           10kΩ pull-up + 100nF    │          │
           │     │                                   │          │
           │     │ PS1 = LOW  ── GND (I2C mode)     │          │
           │     │ PS0 = LOW  ── GND                │          │
           │     │ COM3 = LOW ── GND (Addr 0x28)    │          │
           │     │                                   │          │
           │     │ BOOT = LOW ── GND (normal mode)  │          │
           │     │                                   │          │
           │     │ XTAL:                             │          │
           │     │ XIN32 ──┐                         │          │
           │     │         │ 32.768kHz               │          │
           │     │ XOUT32 ─┘ Crystal                 │          │
           │     │         2×15pF to GND             │          │
           │     │                                   │          │
           │     │ GND (thermal pad) ── GND plane   │          │
           │     └──────────────────────────────────┘          │
           │                                                   │
           │  I2C Pull-ups (shared with I2C1 bus):             │
           │  SDA ── 4.7kΩ ── 3.3V                            │
           │  SCL ── 4.7kΩ ── 3.3V                            │
           │                                                   │
          GND                                                 GND

    Sensor Fusion Modes:
    ┌────────────────────────────────────────────┐
    │  Mode: NDOF (9-DOF absolute orientation)   │
    │  Accelerometer: ±4g, 14-bit, 100Hz        │
    │  Gyroscope: ±2000°/s, 16-bit, 100Hz       │
    │  Magnetometer: ±1300µT, 100Hz             │
    │  Fusion Output: Quaternion + Euler + Gravity│
    │  Calibration: Auto-calibration with save   │
    │  Operating Temp: -40°C to +85°C            │
    └────────────────────────────────────────────┘

    Placement Notes:
    ┌────────────────────────────────────────────┐
    │  - Mount at geometric center of robot      │
    │  - Orient X-axis forward, Y-axis left      │
    │  - Keep ≥15mm from motors/magnets          │
    │  - Isolate from vibration (rubber mount)   │
    │  - Route I2C away from power traces        │
    └────────────────────────────────────────────┘
```

---

## 13. BQ76952 Battery Management System Circuit

```
                    BQ76952 8S LiFePO4 BMS Circuit
═══════════════════════════════════════════════════════════════════════════

    Cell 8+ (Pack+) ─── 100Ω ── VC8 ──┐
    Cell 8- / Cell 7+ ── 100Ω ── VC7 ──┤
    Cell 7- / Cell 6+ ── 100Ω ── VC6 ──┤
    Cell 6- / Cell 5+ ── 100Ω ── VC5 ──┤      ┌─────────────────────────┐
    Cell 5- / Cell 4+ ── 100Ω ── VC4 ──┤      │      BQ76952PFBR        │
    Cell 4- / Cell 3+ ── 100Ω ── VC3 ──┼──────┤  16S Battery Monitor    │
    Cell 3- / Cell 2+ ── 100Ω ── VC2 ──┤      │  (configured for 8S)    │
    Cell 2- / Cell 1+ ── 100Ω ── VC1 ──┤      │                         │
    Cell 1- (Pack-) ──── 100Ω ── VC0 ──┘      │                         │
                                                │                         │
    Each VC pin: 100nF ── GND (filter cap)     │  Cell Balancing:        │
                                                │  CB1-CB8 ──── 33Ω ──── │
    Cell Balancing (per cell):                  │  each cell (internal    │
    ┌───────────────────────────────────┐       │  FET, 50mA balance)    │
    │  VC(n) ──── BQ76952 CB(n) pin    │       │                         │
    │              │                    │       │  Protection:            │
    │            33Ω (balance resistor) │       │  CHG FET ── DSG FET    │
    │              │                    │       │  (external N-ch MOSFET) │
    │           VC(n-1)                 │       │                         │
    │  Balance current: ~50mA/cell     │       │                         │
    └───────────────────────────────────┘       └─────────┬───────────────┘
                                                          │
    Protection FETs:                                      │
    ┌──────────────────────────────────────────────────┐  │
    │                                                   │  │
    │  Pack+ ──┬── INA226 (Hi-side sense) ──┐          │  │
    │          │                              │          │  │
    │       10mΩ shunt                       │          │  │
    │          │                              │          │  │
    │          ├── DSG (Q1, CSD19536KCS) ────┤          │  │
    │          │   Gate ← BQ76952 DSG pin    │          │  │
    │          │                              │          │  │
    │          ├── CHG (Q2, CSD19536KCS) ────┤          │  │
    │          │   Gate ← BQ76952 CHG pin    │          │  │
    │          │                              │          │  │
    │          └── LOAD ────────────────────────► Main Bus  │
    │                                                   │  │
    └──────────────────────────────────────────────────┘  │
                                                          │
    Communication & Monitoring:                           │
    ┌──────────────────────────────────────────────────┐  │
    │  I2C (400kHz):                                    │  │
    │  SDA ── PB6 (I2C1, shared bus)                   │  │
    │  SCL ── PB7                                       │  │
    │  Address: 0x08 (default)                          │  │
    │                                                   │  │
    │  ALERT ── PD8 (EXTI, active LOW)                 │  │
    │           10kΩ pull-up                             │  │
    │                                                   │  │
    │  Thermistors:                                     │  │
    │  TS1 ── NTC 10kΩ (cells 1-4, on cell tabs)      │  │
    │  TS2 ── NTC 10kΩ (cells 5-8, on cell tabs)      │  │
    │  TS3 ── NTC 10kΩ (FET heatsink)                  │  │
    │  Each: NTC ── TS pin, 10kΩ pull-up to REG18     │  │
    │        + 100nF filter cap                         │  │
    │                                                   │  │
    │  REG18 (1.8V internal LDO) ── 1µF ── GND        │  │
    │  REG1  (3.3V internal LDO) ── 4.7µF ── GND      │  │
    └──────────────────────────────────────────────────┘  │

    Protection Thresholds (configured via I2C):
    ┌────────────────────────────────────────────┐
    │  OVP:  3.65V per cell (LiFePO4)           │
    │  UVP:  2.50V per cell                      │
    │  OCP charge:  5A                           │
    │  OCP discharge: 60A                        │
    │  SCP:  80A (short circuit, <100µs trip)    │
    │  OTP:  60°C (over-temperature)             │
    │  UTP:  -20°C (under-temperature)           │
    │  Cell balance: starts at ΔV > 20mV         │
    └────────────────────────────────────────────┘
```

---

## 14. LT3652 MPPT Solar Charger Circuit

```
                    LT3652 Solar MPPT Charge Controller
═══════════════════════════════════════════════════════════════════════════

    Solar Panel 50W (18V Voc, 2.8A Isc)
    ┌──────────┐
    │  Panel+  ├────┬─── Schottky (SS34) ───┬──────────────────────────┐
    │  18V     │    │    (reverse polarity)  │                          │
    │          │   TVS                       │                          │
    │  Panel-  ├──┬─┤── GND                 │                          │
    └──────────┘  │ │                        │                          │
                  │ │                        │                          │
                 GND│                        │                          │
                    │               ┌────────┴────────────┐            │
                    │               │     LT3652EMSE      │            │
                    │               │     MPPT Controller  │            │
                    │               │                      │            │
    MPPT Voltage    │               │ VIN ─── Input power  │            │
    Set Point:      │               │                      │            │
    ┌──────────┐    │               │ VIN_REG ──┐          │            │
    │ 309kΩ    │    │               │           │ MPPT     │            │
    │ (to VIN) │    │               │           │ voltage  │            │
    │    │     │    │               │           │ set:     │            │
    │  100kΩ   │    │               │       ┌───┘ R1/R2   │            │
    │ (to GND) │    │               │       │   = 14.4V   │            │
    └──────────┘    │               │       │   (0.8×Voc) │            │
                    │               │       │              │            │
                    │               │ BOOST ── 100nF       │            │
                    │               │                      │            │
                    │               │ SW ─── L1 (22µH, 5A)│            │
                    │               │         │            │            │
                    │               │         ├── D1       │            │
                    │               │         │  (SS34     │            │
                    │               │         │  Schottky) │            │
                    │               │         │            │            │
                    │               │         └── VBAT ────┼──► Pack+
                    │               │              │       │   (25.6V)
                    │               │           47µF ×2    │
                    │               │           35V ceramic│
                    │               │              │       │
                    │               │ BAT ─── VBAT │       │
                    │               │              │       │
    Charge Current  │               │ SENSE ──┬────┘       │
    Set (2A max):   │               │         │            │
    ┌──────────┐    │               │      50mΩ           │
    │ R_sense  │    │               │      (current       │
    │ = 50mΩ   │    │               │       sense)        │
    │ I = 100mV│    │               │         │            │
    │   /50mΩ  │    │               │         └── Pack-    │
    │ = 2A     │    │               │                      │
    └──────────┘    │               │ CHRG ── LED (charge  │
                    │               │          indicator)  │
                    │               │          + PC8 GPIO  │
                    │               │                      │
                    │               │ FAULT ── LED (fault) │
                    │               │           + PC9 GPIO │
                    │               │                      │
                    │               │ TIMER ── 1MΩ to GND  │
                    │               │  (3hr safety timer)  │
                    │               │                      │
    Float Voltage   │               │ VFB ──┐              │
    (25.6V):        │               │       │ R_top=953kΩ  │
    ┌──────────┐    │               │       │              │
    │ VFB =    │    │               │       ├── 3.3V ref  │
    │ 3.3V     │    │               │       │              │
    │ R_top/   │    │               │       │ R_bot=140kΩ  │
    │ R_bot    │    │               │       │              │
    │ sets     │    │               │       └── GND        │
    │ 25.6V    │    │               │                      │
    └──────────┘    │               │ NTC ── 10kΩ NTC     │
                    │               │   (battery temp)     │
                    │               │   Shuts down >60°C   │
                    │               │                      │
                    │               │ GND ── GND           │
                    │               └──────────────────────┘
                    │
                   GND
```

---

## 15. ESP32-S3 Co-Processor Interconnect Circuit

```
                    ESP32-S3 ↔ STM32H743 Interconnect
═══════════════════════════════════════════════════════════════════════════

    3.3V (from TPS563200) ──┬──────────────────────────────────────┐
                             │                                      │
                          10µF  100nF ×3                          │
                             │    │                                 │
                             ├────┤                                 │
                             │                                      │
    ┌────────────────────────┴──────────────────────────┐          │
    │            ESP32-S3-WROOM-1-N16R8                  │          │
    │            (WiFi + BLE 5.0 Co-processor)           │          │
    │                                                    │          │
    │  ── UART Link to STM32 (921600 baud) ──           │          │
    │  TXD0 (GPIO43) ─── 100Ω ─── PB10 (UART3_TX)     │          │
    │  RXD0 (GPIO44) ─── 100Ω ─── PB11 (UART3_RX)     │          │
    │                                                    │          │
    │  ── Hardware Flow Control ──                       │          │
    │  RTS (GPIO15) ──── 100Ω ──── PB13 (CTS)          │          │
    │  CTS (GPIO16) ──── 100Ω ──── PB14 (RTS)          │          │
    │                                                    │          │
    │  ── Control Signals ──                             │          │
    │  GPIO0 ─── BOOT (10kΩ pull-up, button to GND)    │          │
    │  EN    ─── RESET (10kΩ pull-up + 1µF RC)         │          │
    │             + PB5 (STM32 can reset ESP32)          │          │
    │  GPIO2 ─── READY signal → PD9 (STM32 EXTI)       │          │
    │             (ESP32 asserts when WiFi connected)    │          │
    │                                                    │          │
    │  ── WiFi AP Mode (Robot hotspot) ──                │          │
    │  SSID: ADR1-{serial_number}                        │          │
    │  Pass: Configured via STM32                        │          │
    │  IP:   192.168.4.1                                 │          │
    │  WebSocket: ws://192.168.4.1:8080/ws              │          │
    │                                                    │          │
    │  ── BLE Service ──                                 │          │
    │  Service UUID:  4ADR0001-1234-5678-ABCD-ADR1ROBOT │          │
    │  CMD Char:      4ADR0002-... (Write, 20 byte MTU) │          │
    │  Telemetry:     4ADR0003-... (Notify, 20 byte)    │          │
    │                                                    │          │
    │  ── USB Programming Port ──                        │          │
    │  GPIO19 (D-) ──── USB-C (secondary)               │          │
    │  GPIO20 (D+) ──── USB-C                           │          │
    │                                                    │          │
    │  ── Integrated Antenna (PCB trace) ──              │          │
    │  Keep-out zone: 15mm × 10mm around antenna        │          │
    │  No copper pour under antenna                      │          │
    │                                                    │          │
    │  GND (14 pads) ── GND plane                       │          │
    └────────────────────────────────────────────────────┘          │
                                                                    │
    UART Protocol (STM32 ↔ ESP32):                                 │
    ┌────────────────────────────────────────────┐                  │
    │  Header: [0xAD][0x01]                      │                  │
    │  Version: [0x01]                            │                  │
    │  Type: [CMD=0x01 | TEL=0x02 | CFG=0x03]   │                  │
    │  Length: [Hi][Lo]                           │                  │
    │  Payload: [N bytes]                         │                  │
    │  CRC-8: [polynomial 0x07]                  │                  │
    │                                             │                  │
    │  Bridge: ESP32 relays packets between       │                  │
    │  WiFi/BLE clients and STM32 UART           │                  │
    └────────────────────────────────────────────┘                 GND
```

---

## 16. ADS1115 ADC & INA226 Current Sense Circuits

```
                    ADS1115 16-bit ADC Circuit (×2)
═══════════════════════════════════════════════════════════════════════════

    3.3V ──┬──────────────────────────────────────┐
           │                                       │
         100nF  10µF                              │
           │     │                                 │
           ├─────┤                                 │
           │                                       │
    ┌──────┴──────────────────────────────┐       │
    │         ADS1115IDGSR (#1)           │       │
    │         I2C Address: 0x48           │       │
    │         (ADDR → GND)               │       │
    │                                     │       │
    │ SDA ── PB10 (I2C2)    VDD ── 3.3V │       │
    │ SCL ── PB11 (I2C2)                 │       │
    │ ALRT/RDY ── PD10 (EXTI)           │       │
    │              10kΩ pull-up           │       │
    │                                     │       │
    │ AIN0 ── Soil Moisture Ch1          │       │
    │          ┌──────────────────┐       │       │
    │          │ 10kΩ ── AIN0    │       │       │
    │          │  │              │       │       │
    │          │ 100nF           │       │       │
    │          │  │              │       │       │
    │          │ GND (LP filter  │       │       │
    │          │  fc ≈ 160Hz)    │       │       │
    │          └──────────────────┘       │       │
    │                                     │       │
    │ AIN1 ── Soil Moisture Ch2          │       │
    │ AIN2 ── Soil Moisture Ch3          │       │
    │ AIN3 ── Battery Voltage (÷10)      │       │
    │          (250kΩ/27.4kΩ divider)    │       │
    │                                     │       │
    │ GND ── GND                          │       │
    └─────────────────────────────────────┘       │
                                                   │
    ┌──────────────────────────────────────┐       │
    │         ADS1115IDGSR (#2)            │       │
    │         I2C Address: 0x49            │       │
    │         (ADDR → VDD)                │       │
    │                                      │       │
    │ AIN0 ── MB1240 Ultrasonic Analog #1 │       │
    │ AIN1 ── MB1240 Ultrasonic Analog #2 │       │
    │ AIN2 ── MB1240 Ultrasonic Analog #3 │       │
    │ AIN3 ── MB1240 Ultrasonic Analog #4 │       │
    │                                      │       │
    │ Each input: 100Ω series + 100nF     │       │
    │ anti-aliasing filter                 │       │
    └──────────────────────────────────────┘       │
                                                   │
    Configuration: 860 SPS, ±4.096V FSR, continuous│
                                                  GND

═══════════════════════════════════════════════════════════════════════════
                    INA226 Current/Power Monitor (×2)
═══════════════════════════════════════════════════════════════════════════

    ┌──── INA226 #1 (Pack Current Monitor, Addr: 0x40, A0=A1=GND) ────┐
    │                                                                    │
    │  Pack+ ───┬── 10mΩ Shunt (Bourns CSS2H-2512R) ──┬── Load+       │
    │           │   (2W, 1%, Kelvin connection)        │               │
    │           │                                       │               │
    │       ┌───┴───┐                              ┌───┴───┐           │
    │       │ VIN+  │                              │ VIN-  │           │
    │       │       │                              │       │           │
    │       │         INA226AIDGSR                 │       │           │
    │       │                                      │       │           │
    │       │ VS ── 3.3V (100nF + 1µF decoupling) │       │           │
    │       │                                      │       │           │
    │       │ SDA ── PB10 (I2C2, shared)           │       │           │
    │       │ SCL ── PB11                           │       │           │
    │       │ ALERT ── PD11 (EXTI, open-drain)     │       │           │
    │       │          10kΩ pull-up                  │       │           │
    │       │                                      │       │           │
    │       │ A0 ── GND    A1 ── GND              │       │           │
    │       │ (Address: 0x40)                      │       │           │
    │       │                                      │       │           │
    │       │ GND ── GND                           │       │           │
    │       └───────┘                              └───────┘           │
    │                                                                    │
    │  Calibration: R_shunt=10mΩ, Max_I=8.192A, LSB=250µA             │
    │  Power LSB: 6.25mW, Bus voltage LSB: 1.25mV                     │
    └──────────────────────────────────────────────────────────────────┘

    ┌──── INA226 #2 (Solar Input Monitor, Addr: 0x41, A0=VDD,A1=GND) ─┐
    │                                                                    │
    │  Solar+ ──┬── 20mΩ Shunt ──┬── Charger VIN                      │
    │           │                  │                                     │
    │        VIN+              VIN-                                      │
    │  Same circuit as above, address 0x41                              │
    │  Max solar current: 4A, LSB=125µA                                │
    └──────────────────────────────────────────────────────────────────┘
```

---

## 17. AS5047P Magnetic Encoder Circuit (×4 Wheels)

```
                    AS5047P Magnetic Rotary Encoder (AEC-Q100)
═══════════════════════════════════════════════════════════════════════════

    3.3V ──┬──────────────────────────────────────────────┐
           │                                               │
         100nF  10µF                                     │
           │     │                                        │
           ├─────┤                                        │
           │                                               │
    ┌──────┴────────────────────────────────────┐         │
    │           AS5047P-ATSM                     │         │
    │           (TSSOP-14, AEC-Q100)             │         │
    │           -40°C to +125°C                  │         │
    │                                            │         │
    │  ── SPI Interface (Front-Left encoder) ──  │         │
    │  CLK  ── PE2 (SPI4_SCK)    Max 10MHz      │         │
    │  MISO ── PE5 (SPI4_MISO)                  │         │
    │  MOSI ── PE6 (SPI4_MOSI)                  │         │
    │  CSn  ── PE4 (SPI4_NSS)   Active LOW      │         │
    │                                            │         │
    │  ── Incremental ABI Output (optional) ──   │         │
    │  A ── NC (or timer input for quadrature)  │         │
    │  B ── NC                                   │         │
    │  I ── NC (index pulse, 1/rev)             │         │
    │                                            │         │
    │  ── PWM Output ──                          │         │
    │  PWM ── NC (not used, SPI preferred)      │         │
    │                                            │         │
    │  VDD3V ── 3.3V                             │         │
    │  VDD5V ── NC (3.3V mode only)             │         │
    │  GND ── GND (thermal pad)                  │         │
    │                                            │         │
    └────────────────────────────────────────────┘         │
                                                           │
    Magnet Mounting:                                       │
    ┌────────────────────────────────────────────┐         │
    │                   ┌─────┐                  │         │
    │  Motor Shaft ─────┤ N S │ Diametrically   │         │
    │  (end)            │ S N │ magnetized       │         │
    │                   └──┬──┘ 6mm dia × 3mm   │         │
    │                      │                     │         │
    │                 Air Gap: 0.5-2.5mm         │         │
    │                      │                     │         │
    │                   ┌──┴──┐                  │         │
    │                   │AS504│ IC face up       │         │
    │                   │7P   │ centered on      │         │
    │                   └─────┘ magnet axis      │         │
    │                                            │         │
    │  Magnet: ams AS5000-MD6H-3               │         │
    │  Material: Sintered NdFeB                 │         │
    │  Tolerance: ±0.1mm centering              │         │
    └────────────────────────────────────────────┘         │
                                                          GND
    Encoder Assignment:
    ┌────────────────────────────────────────────┐
    │  FL (Front-Left):  SPI4 (PE2-6)          │
    │  FR (Front-Right): SPI5 (PF7-9, PF6=CS) │
    │  RL (Rear-Left):   SPI6 (PG12-14,G11=CS)│
    │  RR (Rear-Right):  TIM4 ABI mode (PB6)  │
    │                     (uses ABI output pins)│
    │                                            │
    │  SPI Read: 0x3FFF register = 14-bit angle │
    │  Resolution: 4096 steps/revolution        │
    │  Accuracy: ±0.05° typical                  │
    │  Max RPM: 28,000 (SPI polling)            │
    └────────────────────────────────────────────┘
```

---

## 18. MB1240 Industrial Ultrasonic Sensor Circuit (×4)

```
                    MaxBotix MB1240 XL-MaxSonar-EZ4 (IP67)
═══════════════════════════════════════════════════════════════════════════

    5V (from LM2596S) ──┬────────────────────────────────────────┐
                         │                                        │
                      100µF  100nF                               │
                         │    │                                   │
                         ├────┤                                   │
                         │                                        │
    ┌────────────────────┴────────────────────────┐              │
    │         MB1240 XL-MaxSonar-EZ4              │              │
    │         (Industrial, IP67 rated)             │              │
    │         -40°C to +85°C operating             │              │
    │                                              │              │
    │  Pin 1 (BW) ── NC (or GND for serial out)  │              │
    │                                              │              │
    │  Pin 2 (PW) ── Pulse Width Output           │              │
    │                 (147µs per inch)             │              │
    │                 Not used (analog preferred)  │              │
    │                                              │              │
    │  Pin 3 (AN) ── Analog Voltage Output ───────┼──► ADS1115
    │                 Vcc/1024 per cm              │    (via cable)
    │                 At 5V: ~4.9mV/cm             │
    │                 Max range: 765cm → ~3.75V    │
    │                                              │
    │                 Signal Conditioning:          │
    │                 AN ── 1kΩ ──┬── ADS1115 AIN  │
    │                             │                │
    │                          100nF               │
    │                             │                │
    │                           GND                │
    │                                              │
    │  Pin 4 (RX) ── Range Trigger Input          │
    │                 (Hold HIGH for continuous)   │
    │                 ── 10kΩ ── 5V (always-on)    │
    │                                              │
    │  Pin 5 (TX) ── RS232 Serial Output          │
    │                 9600 baud, "Rnnnn\r"         │
    │                 (backup to analog)           │
    │                                              │
    │  Pin 6 (VCC) ── 5V supply                   │
    │  Pin 7 (GND) ── GND                         │
    │                                              │
    └──────────────────────────────────────────────┘
                                                                 │
    Sensor Placement (4 sensors, 90° apart):                     │
    ┌────────────────────────────────────────────┐               │
    │                                            │               │
    │              ┌─────────┐                   │               │
    │    MB1240 ───┤  FRONT  ├─── MB1240         │               │
    │    (Left)    │  ROBOT  │    (Right)        │               │
    │              │  BODY   │                   │               │
    │    MB1240 ───┤         ├─── MB1240         │               │
    │    (Rear)    └─────────┘    (reserved)     │               │
    │                                            │               │
    │  Beam width: ~45° cone                     │               │
    │  Update rate: 10Hz (100ms cycle)           │               │
    │  Min range: 20cm, Max range: 765cm         │               │
    │                                            │               │
    │  Wiring: 3-wire shielded cable             │               │
    │  IP67 PG7 cable glands at PCB entry        │               │
    └────────────────────────────────────────────┘              GND

    ADS1115 Channel Mapping:
    ┌────────────────────────────────────────────┐
    │  ADS1115 #2 (Addr 0x49):                  │
    │  AIN0 ── MB1240 Front  (PE0 alt)          │
    │  AIN1 ── MB1240 Right  (PE2 alt)          │
    │  AIN2 ── MB1240 Rear   (PE4 alt)          │
    │  AIN3 ── MB1240 Left   (PE6 alt)          │
    │                                            │
    │  Conversion: V_adc × (1024/Vcc) = cm      │
    │  At 3.3V ADC ref: scale by 5V/3.3V ratio  │
    └────────────────────────────────────────────┘
```

---

## 19. AR0234CS Industrial Camera Interface Circuit

```
                    AR0234CS CMOS Image Sensor (NDVI Imaging)
═══════════════════════════════════════════════════════════════════════════

    Power Rails:
    3.3V (I/O) ──┬── 100nF ×4 ── GND
                  │
    2.8V (Analog)┬── LDO (AP2112K-2.8) ── 3.3V input
                  │   100nF + 10µF
                  │
    1.2V (Core) ─┬── LDO (AP2112K-1.2) ── 3.3V input
                  │   100nF + 10µF

    ┌────────────────────────────────────────────────────┐
    │              AR0234CSSC00SUKA0-DRBR                 │
    │              2.3MP Global Shutter                    │
    │              Industrial: -40°C to +105°C             │
    │              1/2.6" optical format                   │
    │                                                      │
    │  ── DCMI Parallel Interface to STM32H743 ──         │
    │                                                      │
    │  D0  (Y0) ──── PE0  (DCMI_D0)  ── 33Ω series      │
    │  D1  (Y1) ──── PE1  (DCMI_D1)  ── 33Ω series      │
    │  D2  (Y2) ──── PE4  (DCMI_D2)  ── 33Ω series      │
    │  D3  (Y3) ──── PE5  (DCMI_D3)  ── 33Ω series      │
    │  D4  (Y4) ──── PE6  (DCMI_D4)  ── 33Ω series      │
    │  D5  (Y5) ──── PB6  (DCMI_D5)  ── 33Ω series      │
    │  D6  (Y6) ──── PE7  (DCMI_D6)  ── 33Ω series      │
    │  D7  (Y7) ──── PE8  (DCMI_D7)  ── 33Ω series      │
    │                                                      │
    │  PCLK  ──────── PA6  (DCMI_PIXCLK)                 │
    │  HSYNC ──────── PA4  (DCMI_HSYNC)                   │
    │  VSYNC ──────── PB7  (DCMI_VSYNC)                   │
    │                                                      │
    │  ── I2C Control (SCCB compatible) ──                │
    │  SDA ──── PB9  (I2C1 alt)  4.7kΩ pull-up          │
    │  SCL ──── PB8  (I2C1 alt)  4.7kΩ pull-up          │
    │  Address: 0x10 (7-bit)                              │
    │                                                      │
    │  ── Clock Input ──                                   │
    │  EXTCLK ── PA8 (MCO1, 24MHz from STM32 PLL)        │
    │            33Ω series resistor                       │
    │                                                      │
    │  ── Control ──                                       │
    │  RESET_B ── PD12 (active LOW, 10kΩ pull-up)        │
    │  STANDBY ── PD13 (active HIGH, 10kΩ pull-down)     │
    │  TRIGGER ── PD14 (external trigger for sync)        │
    │  FLASH   ── NC                                      │
    │                                                      │
    │  VDD_IO  ── 3.3V                                    │
    │  VAA     ── 2.8V (analog supply)                    │
    │  VDD     ── 1.2V (digital core)                     │
    │  GND     ── GND (exposed pad + pin GND)             │
    └────────────────────────────────────────────────────┘

    Dual Camera NDVI Configuration:
    ┌────────────────────────────────────────────┐
    │                                            │
    │  Camera 1: Hoya R-62 Red Filter (620nm+)  │
    │  Camera 2: Hoya IR-72 NIR Filter (720nm+) │
    │                                            │
    │  Both share I2C bus (different addresses)  │
    │  Camera 1: SADDR=LOW  → 0x10              │
    │  Camera 2: SADDR=HIGH → 0x30              │
    │                                            │
    │  Camera 2 DCMI on secondary mux:          │
    │  STM32 DMA switches between cameras       │
    │  Alternate frame capture at 15fps each     │
    │                                            │
    │  NDVI = (NIR - Red) / (NIR + Red)          │
    │  Resolution: 1920×1200 @ 60fps max        │
    │  Used at: 1920×1200 @ 15fps (low power)   │
    └────────────────────────────────────────────┘

    Lens & Optics:
    ┌────────────────────────────────────────────┐
    │  Lens Mount: M12 (S-mount)                 │
    │  Focal Length: 6mm                         │
    │  FOV: ~60° horizontal                      │
    │  Working distance: 30cm - infinity         │
    │  IR-cut filter: REMOVED (for NIR camera)  │
    └────────────────────────────────────────────┘
```

---

## 20. TPS54360B 24V→12V Buck Converter Circuit (Industrial)

```
                    TPS54360BDDAR Step-Down Converter
═══════════════════════════════════════════════════════════════════════════

    24V Bus ──┬──────────────────────────────────────────────────┐
              │                                                   │
           22µF ×2  (ceramic, 50V, X7R)                         │
              │                                                   │
              ├──── 100nF (high-freq bypass)                    │
              │                                                   │
              │     ┌──────────────────────────────────┐         │
              ├─────┤ VIN              TPS54360BDDAR   │         │
              │     │                  (SO-8, Industrial│         │
              │     │                   -40°C to +85°C) │         │
              │     │                                   │         │
              │     │ EN ── 100kΩ/47kΩ divider to VIN  │         │
              │     │       (UVLO: enable above 18V,    │         │
              │     │        disable below 16V)         │         │
              │     │                                   │         │
              │     │ BOOT ── 100nF ── SW              │         │
              │     │         (bootstrap capacitor)     │         │
              │     │                                   │         │
              │     │ SW ─── L1 (33µH, 5A, Würth 744   │         │
              │     │         774133, shielded)         │         │
              │     │         │                         │         │
              │     │         ├── D1 (SS34, catch diode)│         │
              │     │         │   Cathode → SW node     │         │
              │     │         │   Anode → GND            │         │
              │     │         │                         │         │
              │     │         └── VOUT (12V) ──────────┼──► 12V Rail
              │     │              │                    │
              │     │           47µF ×2 (ceramic, 25V) │
              │     │              │                    │
              │     │           100nF (ESR bypass)     │
              │     │              │                    │
              │     │ FB ──┬───────┘                    │
              │     │      │                            │
              │     │   R_top = 100kΩ                  │
              │     │      │                            │
              │     │      ├── FB pin (0.8V reference)  │
              │     │      │                            │
              │     │   R_bot = 7.15kΩ                 │
              │     │      │                            │
              │     │     GND                           │
              │     │                                   │
              │     │ Vout = 0.8V × (1 + 100k/7.15k)  │
              │     │      = 0.8V × 14.98 = 11.99V    │
              │     │      ≈ 12.0V                      │
              │     │                                   │
              │     │ COMP ── Type II compensation:     │
              │     │    ┌── 15kΩ ──┬── COMP pin       │
              │     │    │          │                   │
              │     │    │       2.2nF                  │
              │     │    │          │                   │
              │     │   47pF      GND                  │
              │     │    │                              │
              │     │   GND                             │
              │     │                                   │
              │     │ GND (PowerPAD) ── GND plane      │
              │     │ (thermal vias array: 4×4)         │
              │     └──────────────────────────────────┘
              │
             GND

    Specifications:
    ┌────────────────────────────────────────────┐
    │  Input: 18V-28V (from LiFePO4 pack)       │
    │  Output: 12.0V @ 3.5A max                 │
    │  Switching Freq: 480kHz (internal)         │
    │  Efficiency: ~92% at 2A load               │
    │  Ripple: <30mV pp                          │
    │  Loads: Vacuum pump, servo motors,         │
    │         12V peripherals                    │
    └────────────────────────────────────────────┘
```

---

## 21. Controller - DRV2605L Haptic Driver Circuit

```
                    DRV2605L Haptic Feedback Driver
═══════════════════════════════════════════════════════════════════════════

    3.3V ──┬──────────────────────────────────────────┐
           │                                           │
         100nF  1µF (VBAT decoupling)                │
           │     │                                     │
           ├─────┤                                     │
           │                                           │
    ┌──────┴───────────────────────────────┐          │
    │         DRV2605LDGSR                  │          │
    │         Haptic Driver IC              │          │
    │         (MSOP-10)                     │          │
    │                                       │          │
    │ VDD ── 3.3V                           │          │
    │ VBAT ── 3.3V (or direct LiPo 3.0-5V)│          │
    │                                       │          │
    │ SDA ── GPIO43 (I2C)                   │          │
    │ SCL ── GPIO44 (I2C)                   │          │
    │ 4.7kΩ pull-ups on both lines          │          │
    │ Address: 0x5A (fixed)                 │          │
    │                                       │          │
    │ IN/TRIG ── GPIO45 (PWM/trigger)       │          │
    │             (optional, I2C preferred)  │          │
    │                                       │          │
    │ EN ── 10kΩ ── 3.3V (always enabled)  │          │
    │       + GPIO46 (can disable for sleep)│          │
    │                                       │          │
    │ OUT+ ──┐                              │          │
    │        │  LRA Motor                   │          │
    │        │  ┌──────────┐                │          │
    │        ├──┤ LRA-0832 │                │          │
    │        │  │ 235Hz    │                │          │
    │        │  │ resonant │                │          │
    │        │  └──────────┘                │          │
    │ OUT- ──┘                              │          │
    │                                       │          │
    │ GND ── GND                            │          │
    └───────────────────────────────────────┘          │
                                                      GND

    Haptic Effects Library (pre-programmed):
    ┌────────────────────────────────────────────┐
    │  Mode: LRA (auto-resonance calibration)    │
    │  Library: LRA library (effects 1-123)      │
    │                                            │
    │  Used effects:                             │
    │  - Button press: Effect 1 (Strong Click)   │
    │  - E-STOP confirm: Effect 14 (Sharp Buzz) │
    │  - Obstacle warning: Effect 52 (Pulsing)  │
    │  - Connection lost: Effect 47 (Buzz 100%) │
    │  - Mode change: Effect 7 (Soft Bump)       │
    │  - Low battery: Effect 16 (Alert 750ms)   │
    └────────────────────────────────────────────┘
```

---

## 22. Controller - TPS63020 Buck-Boost Converter Circuit

```
                    TPS63020DSJR 3.3V Buck-Boost Regulator
═══════════════════════════════════════════════════════════════════════════

    VBAT (2.5V-5.5V from LiPo) ──┬───────────────────────────────┐
    (3.0V depleted → 4.2V full)   │                               │
                                   │                               │
                                10µF ×2  100nF                   │
                                (ceramic  (high-freq)             │
                                 6.3V)    │                       │
                                   │      │                       │
                                   ├──────┤                       │
                                   │                               │
    ┌──────────────────────────────┴───────────────┐              │
    │              TPS63020DSJR                     │              │
    │              Buck-Boost Converter              │              │
    │              (QFN-14, Industrial -40/+85°C)   │              │
    │                                               │              │
    │  VIN ── VBAT                                  │              │
    │                                               │              │
    │  VINA ── VBAT (analog supply, 100nF to GND) │              │
    │                                               │              │
    │  EN ── VBAT (always on via 100kΩ pull-up)    │              │
    │         + GPIO48 (ESP32 can disable for       │              │
    │           deep sleep, via MOSFET)             │              │
    │                                               │              │
    │  PS/SYNC ── GND (power-save mode enabled,    │              │
    │              high efficiency at light load)   │              │
    │                                               │              │
    │  L1 ──┐                                       │              │
    │       │  L = 2.2µH (Coilcraft XFL4020)       │              │
    │       │  DCR < 50mΩ, Isat > 5A              │              │
    │  L2 ──┘                                       │              │
    │                                               │              │
    │  VOUT ── 3.3V ────────────────────────────────┼──► 3.3V Rail
    │          │                                    │    (System)
    │       22µF ×2 (ceramic, 6.3V, X5R)          │
    │          │                                    │
    │       100nF                                   │
    │          │                                    │
    │         GND                                   │
    │                                               │
    │  FB ── Voltage divider:                       │
    │     R_top = 1MΩ                               │
    │        │                                      │
    │        ├── FB pin (0.5V internal ref)         │
    │        │                                      │
    │     R_bot = 180kΩ                             │
    │        │                                      │
    │       GND                                     │
    │                                               │
    │  Vout = 0.5V × (1 + 1M/180k) = 3.28V ≈ 3.3V│
    │                                               │
    │  PG (Power Good) ── GPIO38 (ESP32 input)     │
    │                      (open-drain, 100kΩ       │
    │                       pull-up to 3.3V)        │
    │                                               │
    │  GND (exposed pad) ── GND plane              │
    │  (thermal vias: 3×3 array)                   │
    └───────────────────────────────────────────────┘
                                                                  │
    Specifications:                                              GND
    ┌────────────────────────────────────────────┐
    │  Input: 2.5V - 5.5V (LiPo full range)     │
    │  Output: 3.3V ±1%                          │
    │  Max output current: 4A (peak)             │
    │  Typical load: 800mA (ESP32+display)       │
    │  Efficiency: >93% at 500mA                 │
    │  Switching freq: 2.4MHz                    │
    │  Quiescent: 25µA (power-save mode)         │
    │  Ripple: <15mV pp                          │
    └────────────────────────────────────────────┘
```

---

## 23. Controller - MCP73871 Charge & Load Share Circuit

```
                    MCP73871-2CCI LiPo Charger + Load Share
═══════════════════════════════════════════════════════════════════════════

    USB-C (5V) ──────┬────────────────────────────────────────────────┐
                      │                                                │
                    100µF  100nF                                      │
                    10V    │                                           │
                      │    │                                           │
                      ├────┤                                           │
                      │                                                │
    ┌─────────────────┴────────────────────────────────┐              │
    │              MCP73871-2CCI/ML                      │              │
    │              Charge Management + Load Sharing      │              │
    │              (QFN-20, Industrial -40°C to +85°C)  │              │
    │                                                    │              │
    │  IN ── USB 5V (via VBUS from USB-C connector)     │              │
    │                                                    │              │
    │  PROG ── 2kΩ to GND                               │              │
    │           (sets charge current: I = 1000V/R)      │              │
    │           (2kΩ → 500mA charge current)            │              │
    │                                                    │              │
    │  PROG2 ── 10kΩ to GND                             │              │
    │            (sets pre-charge & termination)         │              │
    │                                                    │              │
    │  TE ── 10kΩ to GND                                │              │
    │         (safety timer: ~5hr for 3000mAh)          │              │
    │                                                    │              │
    │  THERM ── NTC 10kΩ (on battery pack)              │              │
    │            10kΩ bias resistor to VDD               │              │
    │            (charge pause: <0°C or >45°C)           │              │
    │                                                    │              │
    │  STAT1 ── LED_CHG (amber) + GPIO (ESP32)          │              │
    │            (LOW = charging)                        │              │
    │  STAT2 ── LED_DONE (green) + GPIO (ESP32)         │              │
    │            (LOW = charge complete)                 │              │
    │  PG ── Power Good (LOW = USB power present)       │              │
    │         10kΩ pull-up                               │              │
    │                                                    │              │
    │  VBAT ── LiPo Battery 3.7V/3000mAh ──────────────┤              │
    │           │                                        │              │
    │        Protection:                                 │              │
    │        ┌──────────────┐                            │              │
    │        │ DW01A +      │                            │              │
    │        │ FS8205A      │                            │              │
    │        │ (OVP: 4.25V) │                            │              │
    │        │ (UVP: 2.50V) │                            │              │
    │        │ (OCP: 3A)    │                            │              │
    │        └──────────────┘                            │              │
    │                                                    │              │
    │  OUT ── Load output ───────────────────────────────┼──► TPS63020
    │          (seamless USB/battery switching)          │    (VBAT in)
    │          When USB present: OUT = USB - 0.2V       │
    │          When USB absent:  OUT = VBAT             │
    │                                                    │              │
    │  VREF ── 100nF to GND (internal reference)        │              │
    │  VSS ── GND (exposed pad, thermal vias)           │              │
    └────────────────────────────────────────────────────┘              │
                                                                       │
    Charge Profile:                                                   GND
    ┌────────────────────────────────────────────┐
    │  Pre-charge: 50mA (VBAT < 3.0V)           │
    │  CC phase: 500mA (constant current)        │
    │  CV phase: 4.20V (constant voltage)        │
    │  Termination: 50mA (end of charge)         │
    │  Charge time: ~6.5hr (0→100%)              │
    │  Trickle recharge: below 4.05V             │
    │  Thermal regulation: junction 120°C        │
    └────────────────────────────────────────────┘
```

---

## 24. Controller - NHD-3.5" Industrial Display Interface

```
                    NHD-3.5-320240MF-ATXI#-1 TFT Display
═══════════════════════════════════════════════════════════════════════════

    3.3V ──┬──────────────────────────────────────────────────┐
           │                                                   │
         100nF  10µF                                         │
           │     │                                            │
           ├─────┤                                            │
           │                                                   │
    ┌──────┴────────────────────────────────────────────┐     │
    │      NHD-3.5-320240MF-ATXI#-1                     │     │
    │      3.5" TFT LCD (480×320, ILI9488 driver)       │     │
    │      Industrial: -30°C to +80°C operating          │     │
    │      Sunlight readable (700 cd/m² typical)         │     │
    │                                                    │     │
    │  ── SPI Interface (4-wire, 40MHz max) ──           │     │
    │                                                    │     │
    │  Pin 1  (VDD)   ── 3.3V                           │     │
    │  Pin 2  (GND)   ── GND                            │     │
    │  Pin 3  (SCK)   ── GPIO7  (SPI3_CLK, 40MHz)     │     │
    │  Pin 4  (SDI)   ── GPIO8  (SPI3_MOSI)           │     │
    │  Pin 5  (SDO)   ── GPIO9  (SPI3_MISO, optional) │     │
    │  Pin 6  (CS)    ── GPIO10 (active LOW)           │     │
    │  Pin 7  (DC)    ── GPIO11 (Data/Command select)  │     │
    │                    (LOW=command, HIGH=data)        │     │
    │  Pin 8  (RST)   ── GPIO12 (active LOW reset)     │     │
    │                    10kΩ pull-up + 100nF RC         │     │
    │                                                    │     │
    │  ── Backlight ──                                   │     │
    │  Pin 9  (LED+)  ── 3.3V                           │     │
    │  Pin 10 (LED-)  ── Q1 (NPN, BC847B)              │     │
    │                    Base ← GPIO13 (PWM, 1kHz)      │     │
    │                    10kΩ base resistor              │     │
    │                    (0-100% brightness control)     │     │
    │                                                    │     │
    │  ── Touch Panel (Resistive, optional) ──           │     │
    │  Pin 11 (XR)    ── GPIO14 (ADC + GPIO)           │     │
    │  Pin 12 (YD)    ── GPIO15 (ADC + GPIO)           │     │
    │  Pin 13 (XL)    ── GPIO16 (GPIO)                 │     │
    │  Pin 14 (YU)    ── GPIO17 (ADC + GPIO)           │     │
    │  (Touch controller: software bit-bang)            │     │
    │                                                    │     │
    └────────────────────────────────────────────────────┘     │
                                                               │
    SPI Bus Sharing (display + SD card):                      GND
    ┌────────────────────────────────────────────┐
    │                                            │
    │  SPI3 Bus ──┬── NHD Display (CS=GPIO10)  │
    │             │    40MHz max                  │
    │             │                               │
    │             └── SD Card (CS=GPIO48)        │
    │                 25MHz max                   │
    │                                            │
    │  Shared: SCK (GPIO7), MOSI (GPIO8),       │
    │          MISO (GPIO9)                      │
    │  Only one CS active at a time              │
    │                                            │
    │  SD Card additional:                       │
    │  ┌──────────────────┐                     │
    │  │ DM3AT-SF-PEJM5  │                     │
    │  │ CS    ── GPIO48  │                     │
    │  │ SCK   ── GPIO7   │                     │
    │  │ MOSI  ── GPIO8   │                     │
    │  │ MISO  ── GPIO9   │                     │
    │  │ CD    ── GPIO39  │ (card detect)       │
    │  │ VDD   ── 3.3V    │                     │
    │  └──────────────────┘                     │
    └────────────────────────────────────────────┘

    Display Specifications:
    ┌────────────────────────────────────────────┐
    │  Resolution: 480 × 320 (HVGA)             │
    │  Color depth: 18-bit (262K colors)         │
    │  Interface: 4-wire SPI (ILI9488 driver)   │
    │  Frame rate: 60Hz max, 30Hz typical       │
    │  Viewing angle: 80° all directions         │
    │  Brightness: 700 cd/m² (sunlight readable)│
    │  Gorilla Glass 3 cover (custom cut)       │
    │  Operating: -30°C to +80°C                 │
    │  Storage: -40°C to +90°C                   │
    └────────────────────────────────────────────┘
```

---

## 25. System-Level Interconnect Diagram

```
                    ADR-1 COMPLETE SYSTEM INTERCONNECT
═══════════════════════════════════════════════════════════════════════════

    ┌═══════════════════════════════════════════════════════════════════┐
    │                        ROBOT MAIN BOARD                          │
    │                                                                   │
    │  ┌─────────────────────────────────────────────────────────────┐ │
    │  │                    STM32H743VIT6                             │ │
    │  │                    (Main MCU, 480MHz)                        │ │
    │  │                                                              │ │
    │  │  UART1 ────────── NEO-M9N GNSS (115200)                    │ │
    │  │  UART2 ────────── MAX3485 ─── RS485 Bus (Soil Sensors)     │ │
    │  │  UART3 ────────── ESP32-S3 Co-processor (921600)           │ │
    │  │  UART5 ────────── TFmini-S-I LiDAR (115200)               │ │
    │  │                                                              │ │
    │  │  SPI1 ─────────── SX1276 LoRa (868MHz)                    │ │
    │  │  SPI2 ─────────── W25Q128 Flash (16MB)                     │ │
    │  │  SPI4 ─────────── AS5047P Encoder FL                       │ │
    │  │  SPI5 ─────────── AS5047P Encoder FR                       │ │
    │  │  SPI6 ─────────── AS5047P Encoder RL                       │ │
    │  │  TIM4 ─────────── AS5047P Encoder RR (ABI)                │ │
    │  │                                                              │ │
    │  │  I2C1 (400kHz) ── BME280 ── BNO055 ── BH1750              │ │
    │  │                   MLX90614 ── VEML6075                      │ │
    │  │  I2C2 (100kHz) ── ADS1115×2 ── INA226×2 ── BQ76952       │ │
    │  │                                                              │ │
    │  │  FDCAN1 ────────── MCP2562FD ─── CAN Bus                  │ │
    │  │  DCMI ──────────── AR0234CS Camera ×2 (NDVI)              │ │
    │  │  1-Wire ────────── DS18B20 Soil Temperature               │ │
    │  │  TIM8 PWM ──────── Servo Motors ×2 (Probe)               │ │
    │  │  GPIO ──────────── Status LEDs, Buzzer, E-STOP            │ │
    │  └─────────────────────────────────────────────────────────────┘ │
    │                           │ UART3                                 │
    │                           │ (921600 baud)                         │
    │  ┌────────────────────────┴────────────────────────────────────┐ │
    │  │                    ESP32-S3-WROOM-1                          │ │
    │  │                    (WiFi/BLE Co-processor)                   │ │
    │  │                                                              │ │
    │  │  WiFi AP ─── 192.168.4.1:8080/ws (WebSocket)              │ │
    │  │  BLE 5.0 ─── ADR1 Service (CMD + Telemetry chars)         │ │
    │  │  Bridge: WiFi/BLE ↔ UART3 ↔ STM32                        │ │
    │  └──────────────────────────────────────────────────────────────┘ │
    │                                                                   │
    │  ┌── POWER ───────────────────────────────────────────────────┐ │
    │  │  LiFePO4 25.6V/20Ah ── BQ76952 BMS                        │ │
    │  │       │                                                     │ │
    │  │       ├── 24V Direct ──── BLDC Drivers (ODrive ×4)        │ │
    │  │       ├── TPS54360B ──── 12V (Vacuum, Servos)             │ │
    │  │       ├── LM2596S ────── 5V (Sensors, MB1240)             │ │
    │  │       ├── TPS563200 ──── 3.3V (MCU, Comms)               │ │
    │  │       └── LT3652 ←────── 50W Solar Panel (18V)           │ │
    │  │       + INA226 ×2 (pack + solar monitoring)                │ │
    │  └────────────────────────────────────────────────────────────┘ │
    │                                                                   │
    │  ┌── CAN BUS ─────────────────────────────────────────────────┐ │
    │  │  STM32─MCP2562FD─┬─ODrive#1(FL)─┬─ODrive#2(FR)           │ │
    │  │                   ├─ODrive#3(RL)─┼─ODrive#4(RR)           │ │
    │  │                   ├─BQ76952 BMS  └─Deseeder Driver        │ │
    │  │                   └─120Ω termination at each end           │ │
    │  └────────────────────────────────────────────────────────────┘ │
    │                                                                   │
    │  ┌── RS485 BUS ───────────────────────────────────────────────┐ │
    │  │  STM32─MAX3485─┬─NPK Sensor (Addr 1)                      │ │
    │  │                 ├─pH Sensor (Addr 2)                        │ │
    │  │                 ├─EC Sensor (Addr 3)                        │ │
    │  │                 ├─Moisture Sensor (Addr 4)                  │ │
    │  │                 └─120Ω termination at each end              │ │
    │  └────────────────────────────────────────────────────────────┘ │
    │                                                                   │
    │  ┌── EXTERNAL CONNECTORS ─────────────────────────────────────┐ │
    │  │  IP67 12-pin (×4): Motor+Encoder per wheel                │ │
    │  │  IP67 4-pin  (×6): Sensors (ultrasonic, soil, weather)    │ │
    │  │  SMA (×2): LoRa antenna, GNSS antenna                     │ │
    │  │  USB-C (×1): Debug/programming (IP67 cap)                 │ │
    │  │  PG7/PG9 cable glands: sensor mast wiring                │ │
    │  └────────────────────────────────────────────────────────────┘ │
    └═══════════════════════════════════════════════════════════════════┘

                    LoRa 868MHz              WiFi / BLE
                    (2km range)              (100m range)
                         │                        │
                         │    ┌───────────┐       │
                         └────┤  WIRELESS ├───────┘
                              │   LINK    │
                         ┌────┤           ├───────┐
                         │    └───────────┘       │
                         │                        │

    ┌═══════════════════════════════════════════════════════════════════┐
    │                    HANDHELD CONTROLLER                            │
    │                                                                   │
    │  ┌─────────────────────────────────────────────────────────────┐ │
    │  │                    ESP32-S3-WROOM-1                          │ │
    │  │                    (Main MCU)                                │ │
    │  │                                                              │ │
    │  │  SPI2 ─────────── SX1276 LoRa (868MHz)                    │ │
    │  │  SPI3 ─────────── NHD-3.5" Display + SD Card              │ │
    │  │  UART1 ────────── MAX-M10S GNSS                            │ │
    │  │  I2C ──────────── DRV2605L Haptic Driver                   │ │
    │  │  ADC ──────────── Joysticks ×2, Battery monitor            │ │
    │  │  GPIO ─────────── Buttons ×8, E-STOP, Buzzer              │ │
    │  │  SPI (GPIO45) ─── APA102-2020 LEDs ×4                     │ │
    │  │  WiFi ─────────── Direct to Robot ESP32 AP                 │ │
    │  │  BLE 5.0 ──────── Direct to Robot ESP32 BLE               │ │
    │  └─────────────────────────────────────────────────────────────┘ │
    │                                                                   │
    │  ┌── POWER ───────────────────────────────────────────────────┐ │
    │  │  USB-C 5V ──── MCP73871 ──── LiPo 3.7V/3000mAh          │ │
    │  │                    │                                        │ │
    │  │               Load Share ──── TPS63020 ──── 3.3V System   │ │
    │  │                               + AP2112K LDO (low noise)    │ │
    │  └────────────────────────────────────────────────────────────┘ │
    └═══════════════════════════════════════════════════════════════════┘

                              │
                         WiFi / BLE
                              │

    ┌═══════════════════════════════════════════════════════════════════┐
    │                    MOBILE APP (Flutter)                           │
    │                                                                   │
    │  ┌─────────────────────────────────────────────────────────────┐ │
    │  │  WiFi WebSocket ── ws://192.168.4.1:8080/ws               │ │
    │  │  BLE ── ADR1 Service UUID (fallback)                       │ │
    │  │                                                              │ │
    │  │  Protocol: [0xAD][0x01][ver][type][len_hi][len_lo]         │ │
    │  │            [payload...][crc8]                                │ │
    │  │                                                              │ │
    │  │  Features: Manual control, Mission planner,                │ │
    │  │            Telemetry dashboard, NDVI/Soil maps,            │ │
    │  │            Sensor monitoring, Settings                     │ │
    │  └─────────────────────────────────────────────────────────────┘ │
    └═══════════════════════════════════════════════════════════════════┘
```

---

## 26. PCB Layer Stack-Up

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
