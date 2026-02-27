# ADR-1 Robot Mechanical Design Document

## 1. Robot Form Factor & Shape

### 1.1 Overall Configuration: Differential-Drive Skid-Steer Platform

```
                    FRONT (Direction of Travel)
                          ▲
                          │
         ┌────────────────┼────────────────┐
         │    ┌───────────┴───────────┐    │
         │    │    SENSOR MAST        │    │
         │    │  ┌─────┐  ┌─────┐    │    │
         │    │  │NavIC │  │Anem │    │    │
         │    │  │Ant.  │  │meter│    │    │
         │    │  └─────┘  └─────┘    │    │
         │    └───────────────────────┘    │
         │         SOLAR PANEL             │
         │    ┌───────────────────────┐    │
         │    │ ░░░░░░░░░░░░░░░░░░░░ │    │
         │    │ ░░░ 50W SOLAR ░░░░░░ │    │
         │    │ ░░░░░░░░░░░░░░░░░░░░ │    │
         │    └───────────────────────┘    │
    ┌────┤                                 ├────┐
    │    │    ┌──── TOP VIEW ────────┐     │    │
    │ L  │    │                      │     │  R │
    │ E  │    │  ┌────┐    ┌────┐   │     │  I │
    │ F  │    │  │BLDC│    │BLDC│   │     │  G │
    │ T  │    │  │ FL │    │ FR │   │     │  H │
    │    │    │  └────┘    └────┘   │     │  T │
    │ T  │    │                      │     │    │
    │ R  │    │      ┌──────┐       │     │  T │
    │ A  │    │      │ MAIN │       │     │  R │
    │ C  │    │      │ PCB  │       │     │  A │
    │ K  │    │      │      │       │     │  C │
    │    │    │      └──────┘       │     │  K │
    │    │    │                      │     │    │
    │    │    │  ┌──────────────┐   │     │    │
    │    │    │  │  DESEEDING   │   │     │    │
    │    │    │  │  MECHANISM   │   │     │    │
    │    │    │  │  ┌────────┐  │   │     │    │
    │    │    │  │  │Rot.Hoe │  │   │     │    │
    │    │    │  │  └────────┘  │   │     │    │
    │    │    │  │  ┌────────┐  │   │     │    │
    │    │    │  │  │Seed Vac│  │   │     │    │
    │    │    │  │  └────────┘  │   │     │    │
    │    │    │  └──────────────┘   │     │    │
    │    │    │                      │     │    │
    │    │    │  ┌────┐    ┌────┐   │     │    │
    │    │    │  │BLDC│    │BLDC│   │     │    │
    │    │    │  │ RL │    │ RR │   │     │    │
    │    │    │  └────┘    └────┘   │     │    │
    │    │    │                      │     │    │
    │    │    └──────────────────────┘     │    │
    └────┤                                 ├────┘
         │    ┌───────────────────────┐    │
         │    │   SOIL SENSOR PROBE   │    │
         │    │   DEPLOYMENT ARM      │    │
         │    └───────────────────────┘    │
         └─────────────────────────────────┘

              ◄─────── 600mm ────────►
```

### 1.2 Side Profile

```
         Sensor Mast (350mm above chassis)
              │
              │   Solar Panel (tilted 15°)
              │  ╱────────────────╲
              │╱                    ╲    Clearance: 200mm
    ┌─────────┴──────────────────────┴───────────┐
    │             MAIN CHASSIS                     │ Height: 250mm
    │  ┌──────────────────────────────────────┐   │
    │  │ Electronics Bay (sealed IP65)        │   │
    │  │ Battery Pack underneath              │   │
    │  └──────────────────────────────────────┘   │
    └───┬──────────────────────────────────┬──────┘
        │                                  │
    ┌───┴───┐  Deseeding Arm (retractable) ┌───┴───┐
    │       │        │                     │       │
    │  ○    │     ┌──┴──┐                  │    ○  │
    │ Wheel │     │Probe│                  │ Wheel │
    │ 200mm │     └─────┘                  │ 200mm │
    │       │                              │       │
    └───────┘                              └───────┘
        ◄──────────── 800mm ────────────────►

    Ground Clearance: 150mm (adjustable via suspension)
```

### 1.3 Dimensions Summary
| Parameter | Value | Justification |
|-----------|-------|---------------|
| Length | 800mm | Fits between standard crop rows (750-900mm spacing) |
| Width | 600mm | Allows passage in narrow furrows while maintaining stability |
| Height | 600mm (with mast) | Low CG for slope stability, mast for clear satellite view |
| Ground clearance | 150mm | Clears crop stubble and uneven terrain |
| Wheel diameter | 200mm | Sufficient for soft soil, ~5cm obstacle traversal |
| Wheel width | 80mm | Distributes weight to reduce soil compaction |
| Weight (empty) | 22 kg | Manageable by single person for transport |
| Weight (loaded) | 32 kg | Within single-person field deployment capability |
| Track width | 500mm | Wide enough for stability on 15° slopes |

---

## 2. Materials Selection & Justification

### 2.1 Chassis Frame

| Component | Material | Justification |
|-----------|----------|---------------|
| Main frame | 6061-T6 Aluminum | Excellent strength-to-weight (276 MPa yield), corrosion resistant, weldable, cost-effective. Density 2.7 g/cm³ vs steel 7.8 g/cm³ saves ~65% weight |
| Cross braces | 6061-T6 Al tube (25x25x2mm) | Standard extrusion, easy sourcing, good torsional rigidity |
| Motor mounts | 7075-T6 Aluminum | Higher strength (503 MPa) for vibration-prone mounting points |
| Suspension brackets | 304 Stainless Steel | Fatigue resistance for cyclic loading, corrosion proof |

**Justification for Aluminum over alternatives:**
- **vs. Steel**: 65% lighter, adequate strength for 32kg robot, no rust concerns
- **vs. Carbon Fiber**: 5-8x cheaper, easier field repair, similar stiffness at this scale
- **vs. HDPE/Plastic**: Insufficient rigidity, poor thermal conductivity for heat dissipation
- **vs. Titanium**: 10x cost with marginal benefit at this load level

### 2.2 Body Panels & Enclosure

| Component | Material | Justification |
|-----------|----------|---------------|
| Top cover | ASA (Acrylonitrile Styrene Acrylate) | UV-resistant (unlike ABS), IP65 achievable, injection moldable |
| Side panels | ASA, 3mm thick | Impact resistant (15 kJ/m²), weather stable for 10+ years outdoor |
| Sensor windows | Polycarbonate (Lexan) | 250x stronger than glass, optical clarity for cameras, UV stable |
| Gaskets | EPDM rubber | Temperature stable (-40 to 120°C), ozone/UV resistant, excellent sealing |
| Fasteners | A4-80 Stainless Steel | Marine-grade corrosion resistance, prevents galvanic corrosion with aluminum |

**IP65 Sealing Strategy:**
- All panel joints: EPDM gasket compression seal
- Cable entries: IP68 cable glands (PG7/PG9/PG11)
- Connector interfaces: Amphenol circular connectors (IP67 rated)
- PCB enclosure: Sealed aluminum sub-enclosure with desiccant pack

### 2.3 Drive System

| Component | Material/Spec | Justification |
|-----------|--------------|---------------|
| Wheels | Pneumatic rubber tire on aluminum hub | Shock absorption on rough terrain, replaceable tires |
| Tire compound | Natural rubber + SBR blend | Self-cleaning tread, good wet traction, field-repairable |
| Tread pattern | Agricultural lug (30mm depth) | Prevents clogging in wet soil, maintains traction |
| Wheel axles | 12mm hardened steel (4140) | Handles shear loads from 4-wheel skid steering |
| Bearings | 6001-2RS sealed ball bearing | Dust/water sealed, 12mm bore, 10kN dynamic rating |
| Drive belt/chain | HTD-5M polyurethane timing belt | No lubrication needed, quiet, precise, field-replaceable |

### 2.4 Deseeding Mechanism

| Component | Material/Spec | Justification |
|-----------|--------------|---------------|
| Rotary hoe blades | Boron steel (30MnB5) | Extremely hard (50 HRC), self-sharpening, used in commercial tillers |
| Hoe shaft | 4140 Chrome-moly steel | Fatigue resistant for rotary operation at 300-500 RPM |
| Seed vacuum nozzle | 316L Stainless Steel | Corrosion proof, smooth bore for seed flow, food-grade |
| Vacuum tubing | Silicone rubber (medical grade) | Flexible, UV resistant, won't degrade with seed/soil contact |
| Seed collection bin | HDPE (food grade) | Lightweight, chemical resistant, easy to clean |
| Probe deployment arm | 6061-T6 Al + linear rail | Precise Z-axis control for soil probe insertion depth |

### 2.5 Sensor Mast

| Component | Material | Justification |
|-----------|----------|---------------|
| Mast tube | Carbon fiber tube (20mm OD) | Lightweight, vibration-damping for antenna stability |
| Antenna ground plane | Copper-clad FR4 | Standard for GNSS antennas, easily fabricated |
| Camera mount | 3D printed PA12 Nylon (SLS) | Complex geometry, UV stable, vibration dampening |
| Weather station bracket | 316L Stainless | Permanent outdoor exposure, zero maintenance |

---

## 3. Suspension System

### 3.1 Design: Independent Trailing Arm with Coil Spring

```
    Chassis Mount Point
         │
    ┌────┴────┐
    │ Pivot   │
    │ Bearing │
    └────┬────┘
         │
    ┌────┴──────────────────────┐
    │    Trailing Arm (Al 6061) │
    │    ┌──────┐               │
    │    │Spring│               │
    │    │Damper│          ┌────┴────┐
    │    │      │          │  Wheel  │
    │    └──────┘          │  Hub    │
    │                      │  Motor  │
    └──────────────────────┴────────┘

    Travel: ±50mm
    Spring Rate: 15 N/mm
    Damping: 0.3 critical
```

**Justification**: Independent suspension on all 4 wheels ensures:
- All wheels maintain ground contact on uneven terrain
- Reduced vibration to sensitive electronics (cameras, IMU)
- Better traction distribution in soft soil conditions
- 100mm total travel absorbs furrow transitions

---

## 4. Thermal Management

| Zone | Solution | Justification |
|------|----------|---------------|
| MCU/Power electronics | Aluminum chassis as heatsink + thermal pads | Passive cooling, no moving parts to fail in dusty environment |
| Motor drivers | Dedicated aluminum heat spreader (50x50x10mm) | MOSFETs can dissipate 5-10W each at full load |
| Battery pack | Insulated compartment + phase-change material | Maintains 15-45°C optimal range, prevents thermal runaway |
| Camera modules | Passive cooling fins on housing | Prevents thermal noise in NDVI measurements |

**Operating Envelope:**
- Ambient: -10°C to 55°C (Indian agricultural range)
- Electronics derate: Above 50°C, reduce motor duty cycle to 70%
- Battery: Heating pad activates below 5°C for lithium cell protection

---

## 5. Handheld Controller Mechanical Design

### 5.1 Form Factor

```
         ┌─────────────────────────────────┐
         │        3.5" TFT DISPLAY         │
         │    ┌───────────────────────┐    │
         │    │                       │    │
         │    │    Status Dashboard   │    │
         │    │    Map View           │    │
         │    │    Sensor Readings    │    │
         │    │                       │    │
         │    └───────────────────────┘    │
         │                                 │
         │  [MODE] [HOME] [ESTOP] [MENU]   │
         │                                 │
    ┌────┤    ┌───┐           ┌───┐       ├────┐
    │    │    │ L │           │ R │       │    │
    │Grip│    │Joy│           │Joy│       │Grip│
    │    │    │   │           │   │       │    │
    │    │    └───┘           └───┘       │    │
    │    │                                 │    │
    │    │  [L1] [L2]         [R1] [R2]   │    │
    │    │                                 │    │
    └────┤    Antenna (internal)           ├────┘
         │    ┌─────────────────────┐      │
         │    │ LoRa + NavIC        │      │
         │    └─────────────────────┘      │
         │                    [USB-C]      │
         └─────────────────────────────────┘

         ◄──────── 180mm ─────────►
         Height: 120mm
         Depth: 45mm (grip: 35mm)
         Weight: 280g (with battery)
```

### 5.2 Controller Materials

| Component | Material | Justification |
|-----------|----------|---------------|
| Housing upper | PC/ABS blend | Impact resistant, UV stable, textured for grip |
| Housing lower | Glass-filled PA6 nylon | Structural rigidity, chemical resistant, RF transparent |
| Grips | TPE (Shore A 60) overmold | Comfortable grip, vibration isolation, non-slip when wet |
| Buttons | Silicone rubber dome + PC keycap | Tactile feedback, water-sealed, 1M+ cycle life |
| Joystick | Alps RKJXV series | Industrial grade, IP40, 2M+ cycle, self-centering |
| Display lens | Gorilla Glass 3 (0.7mm) | Scratch resistant, sunlight readable with AR coating |
| E-STOP button | Mushroom head, latching, red | IEC 60947-5-5 compliant, impossible to miss |
| Antenna | PCB patch antenna (internal) | No external protrusion, sufficient gain for LoRa + NavIC |
| Battery door | PA6 + EPDM gasket | Tool-free access for battery replacement, IP54 sealed |

### 5.3 Controller Ergonomics
- Weight distribution: Balanced center of gravity when held in two hands
- Button reach: All controls accessible without releasing grip
- Display angle: 10° tilt toward user for outdoor readability
- Sunlight readability: 1000 nit TFT + anti-reflective coating
- Operating with gloves: Button spacing >12mm, raised tactile markers

---

## 6. Manufacturing Considerations

| Process | Components | Volume Justification |
|---------|------------|---------------------|
| CNC machining | Chassis frame, motor mounts | Prototype & low volume (<100 units) |
| Sheet metal bending | Side panels (Al option) | Medium volume (100-1000 units) |
| Injection molding | Controller housing, robot covers | High volume (>1000 units) |
| 3D printing (SLS) | Sensor mounts, camera brackets | Complex geometry, low volume |
| PCB assembly | Main board, controller board | Standard SMT process |
| Die casting | Wheel hubs | High volume alternative to CNC |

---

## 7. Maintenance & Serviceability

| Service Item | Interval | Access Method |
|-------------|----------|---------------|
| Tire replacement | 500 hours | Quick-release hub, no tools |
| Belt replacement | 1000 hours | Side panel removal (4 screws) |
| Blade sharpening | 200 hours | Quick-release hoe cartridge |
| Air filter (vacuum) | 100 hours | Tool-free snap lid |
| Battery replacement | 2000 cycles | Slide-out battery tray |
| Firmware update | As needed | OTA WiFi or USB-C port (IP67 cap) |
| Sensor calibration | 500 hours | BLE connection from controller |
