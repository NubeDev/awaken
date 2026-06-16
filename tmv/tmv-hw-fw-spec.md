# 1. Scope & Purpose

This document is the **Nube iO Hardware & Firmware Technical Specification** for the TMV
Monitoring Device — the Nube iO **ACX** platform (drawing ACX-001) — that Nube iO is developing
for Galvin Engineering's **CliniMix-TMV / CliniMix Generation 2** water-safety platform.

It is a *reverse specification*: it restates, from the hardware and firmware team's point of
view, only the parts of Galvin's design intent that Nube iO is responsible for building, and
expands each one with the engineering detail required to take the product from first concept to
a manufacturable design. Galvin's document is a system-level design scope; this document is the
device-level response to it.

**Primary input.** Galvin Engineering, *Temperature Monitoring Gen 2 — Design Scope*,
Document No. 004.00.00.30, Revision 0, NPD/PDDR 1442N (hereafter "the Design Scope").

**What Nube iO owns (this document):** the monitoring transmitter — its electronics, enclosure
interfaces, power architecture, on-device measurement and event logic, connectivity, security
and OTA, and the local control of the optional solenoid output.

**What Nube iO does not own (interface only):** the cloud platform, the CliniMix Management
Portal, the dashboard, email/alert generation, group/threshold administration and the
maintenance-date ledger. Galvin's Design Scope marks these explicitly as **cloud logic**; the
device is deliberately constrained to a *telemetry and execution node* (Design Scope §6.1.2).
This specification treats every cloud-owned parameter as a value the device stores and applies,
not one it decides.

This is Nube iO's **first design pass** against the Gen 2 scope. Where the Design Scope records
an item as *TBD*, this document carries it into **§7 Open Items & Engineering Recommendations**
with a proposed resolution rather than leaving it open.

## 1.1 Reference documents

| Tag | Document | Identifier |
|-----|----------|------------|
| **[DS]** | Galvin Engineering — *Temperature Monitoring Gen 2 Design Scope* | Doc 004.00.00.30 Rev 0 / NPD/PDDR 1442N |
| **[TS]** | EXSENSE Sensor Technology — *TS Series NTC Temperature Sensor, Specifications Approval Sheet* (customer: Galvin Engineering) | P/N TS103F25C3950FA-ML1300A · SAS21022503W · 2021-02-25 |
| **[WG]** | Wallgate — *Installation Drawing for WVPB400-2 (Solenoid)* | WVPB400-2 |
| **[PO]** | Nube iO — *CliniMix-TMV Product Scope (Product Overview)*, prepared for Galvin Engineering | Nube iO drawing ACX-001 Rev V01 |
| **[FL]** | Nube iO ↔ Galvin — *Hardware Development Feedback Loop* (TMV-ACX) | live issues log, 9 items |

The Product Overview **[PO]** supersedes the earlier iO8G Executive Summary as the Nube iO product
reference. Cross-references below are written as, e.g., *(DS §4.1)* for the Design Scope, *(TS §3)*
for the thermistor datasheet, *(WG)* for the solenoid drawing, *(PO)* for the Product Overview, and
*(FL 1.0)* for a Feedback Loop item.


# 2. System Context

The device is a **direct device-to-cloud** transmitter mounted inside the Galvin TMV cabinet. It
takes its sole network path from a single Ethernet/PoE drop to the building network, measures
outlet temperature and flow at the valve, computes per-event summaries on-device, and pushes
them to the Galvin/Rubix cloud. There are no on-site controllers or gateways in the data path
(DS §3.1, §4.7).

```
  TMV outlet                 Galvin TMV cabinet
 +--------------+
 | Flow switch  |--reed--+    +---------------------------+
 | (reed/piston)|        +--> | ACX Monitoring            |  PoE/RJ45
 +--------------+        |    | Transmitter (CliniMix-TMV)|--------> LAN -> Cloud
 | NTC probe    |--10k---+    |   - MCU + RTC              |          (Rubix)
 | (EXSENSE TS) |             |   - Ethernet PHY + PoE PD  |  BLE
 +--------------+             |   - Measurement front-end  | .....> CliniMix
                             |   - Solenoid driver        |        Setup app
 +--------------+    drive    |   - USB-C: test/commission |
 | Solenoid     |<--mini-XLR--|     (valves powered by PoE)|
 | Wallgate     |             +---------------------------+
 | WVPB400-2    |
 +--------------+
```

**Roles relevant to the device (DS §6.1.4):** the plumber commissions it over BLE with the
CliniMix Setup app; the facility manager and Galvin manage thresholds and alerts entirely in the
cloud. The device exposes no general-user control surface beyond the momentary service button.


# 3. Hardware Specification

## 3.1 General electrical & environmental

| Parameter | Specification | Source |
|-----------|---------------|--------|
| Operating ambient temperature | 0 °C to 65 °C | DS §4.1 |
| Ingress protection | IPx4 required (DS); ACX platform rated **IPx5** | DS §4.1, §4.3; PO |
| Primary power | Power over Ethernet (PoE), 100 Mb single-cable; PoE class TBD per device budget — **must also power the valve(s)** — see §7 | DS §4.1; PO; FL 7.0 |
| Secondary power | USB-C — **test and commissioning only**; 12 VDC jack is the alternative field supply on the ACX platform | DS §4.1, §5.5.3; PO; FL 8.0 |
| Concurrent-supply protection | Circuit protection must permit USB and PoE to be connected simultaneously without damage | DS §4.1 |
| Real-time clock | On-board RTC, disciplined to cloud time; all timestamps synchronised to cloud | DS §4.1 |
| Time zone | Configurable (cloud-pushed) | DS §4.1 |

## 3.2 Core components

Building on the established Nube iO **ACX** platform (PO, drawing ACX-001 Rev V01) — which
provides 8 × I/O (2 RO, 2 AO 0–10 VDC, 4 UI), PoE/12 VDC power, an onboard RTC and optional
RS-485/LoRa — the transmitter is built around:

- **MCU:** application microcontroller providing **BLE** for the CliniMix Setup app and
  **optional Wi-Fi** (DS §4.4). Primary connectivity remains wired LAN.
- **Ethernet PHY + PoE PD front-end:** 100 Mb Ethernet (PO) with an IEEE 802.3 PoE
  powered-device controller. Final PoE class is set by the power budget, which must include the
  valve load (§7; FL 7.0).
- **Real-time clock** with battery/supercap hold-over for timestamp continuity across power loss
  (DS §4.1; PO).
- **Measurement front-end** for the NTC probe and reed flow switch (§3.6, §3.7).
- **Solenoid driver** stage, valve powered from PoE with USB-C reserved for test/commissioning
  (§3.8; FL 7.0/8.0).

## 3.3 Power architecture

The device supports two independent supply paths with seamless, automatic hand-over (DS §4.1,
§5.5.3):

1. **PoE (primary)** — powers the full device, **and the solenoid valve(s)**, over the single
   100 Mb RJ45 drop. The feedback loop confirms PoE must supply the valves (FL 7.0); this is the
   sizing-critical case and refines the Design Scope's original USB-C-fed-solenoid assumption
   (DS §5.5.2).
2. **USB-C (secondary)** — used for **testing and commissioning only** (e.g. powering the device
   from a USB power bank before the network drop is live, DS §3.2; FL 8.0). It must still carry
   enough power to actuate the valve during commissioning checks.

Where USB-C is used it supports **USB Power Delivery (USB-PD)** negotiation to request a source
voltage matching the configured solenoid requirement, e.g. 9 V / 12 V or higher (DS §5.5.2).
Local energy buffering decouples solenoid in-rush from the supply so activation does not impose
abrupt load transients (DS §5.5.2). Power-path management ensures no user intervention is
required when switching between PoE and USB-C (DS §5.5.3). Confirming the PoE budget covers
device + valve is the highest-priority open item (§7; FL 7.0/8.0).

## 3.4 Connectors & interfaces

| Interface | Connector | Qty | Source |
|-----------|-----------|-----|--------|
| Networking | RJ45 — waterproofing approach under selection (external port, IP67 connector, sealed cable, or internal-connect) | 1 | DS §4.2; FL 3.0 |
| Power / programming | USB-C | 1 | DS §4.2 |
| Temperature probe | 2-pin IP65 waterproof plug (candidate; pending dev) | 1 | DS §4.2; FL 2.0 |
| Solenoid output | **Male 3-pin mini-XLR**, mating the valve's female mini-XLR | 1 | DS §4.2; FL 1.0 |
| GPIO | TBD | 2 | DS §4.2 |
| Expansion port | RJ11 — remote I/O cards over BACnet / Modbus | 1 | DS §4.2, §5.5.4; FL 5.0 |

## 3.5 Enclosure, indicators & labelling

| Item | Specification | Source |
|------|---------------|--------|
| Dimensions (W × L × H) | 60 × 120 × 40 mm | DS §4.3 |
| Material | ABS | DS §4.3 |
| Indicators | 1 × Power (red), 1 × Status (green) LED | DS §4.3 |
| Button | Momentary status/service button, IPx4 | DS §4.3 |
| Colour | Galvin Blue (Pantone PMS-2935), second colour TBD | DS §4.3 |
| Compliance markings | RCM; ABS / e-waste / IP-rating / lithium-battery (if fitted) marks | DS §4.3 |
| Labelling | Galvin Engineering logo, product code/name, I/O designations, QR code | DS §4.3 |

The QR code carries the immutable manufacturer GUID used to bind the device to the platform at
commissioning (DS §4.3, §6.1.3).

The Design Scope enclosure (60 × 120 × 40 mm ABS, IPx4) and the ACX platform reference enclosure
(max 150 × 40 × 35 mm, IPx5, per drawing ACX-001 Rev V01, PO) differ; the feedback loop flags
Galvin's **detailed box dimensions and internal component layout** as a High-priority input still
needed to finalise the casing and connector placement (FL 4.0; §7).


## 3.6 Temperature-measurement sub-system

Outlet temperature is measured with the **EXSENSE TS-series NTC probe already specified to
Galvin (TS)**, read only while the flow switch indicates flow (DS §5.5.5). The measurement
front-end and the probe together set the accuracy budget.

**Probe — EXSENSE P/N TS103F25C3950FA-ML1300A (TS §1–§3):**

| Characteristic | Value | Source |
|----------------|-------|--------|
| Element | 10 kΩ NTC thermistor | TS §3-1 |
| Resistance @ 25 °C (R25) | 10 kΩ ± 1 % | TS §3-1 |
| B-value B(25/50) | 3950 K ± 1 % | TS §3-2 |
| Thermal time constant τ | ≤ 12 s | TS §3-3 |
| Dissipation factor δ | ≥ 0.8 mW/°C | TS §3-4 |
| Rated power | 5 mW max | TS §3-7 |
| Housing / thread | Stainless steel 304, NPT 1/8 | TS §1 |
| Seal | Silicone O-ring | TS §1 |
| Lead | UL4478, #24 AWG, 1300 mm | TS §1 |
| Insulation / withstand | ≥ 100 MΩ @ DC 500 V; AC 1200 V, 2 mA, 1 s | TS §3-5/3-6 |

**Design implications for the Nube iO front-end:**

- The Design Scope mandates **internal measurement-circuitry tolerance < 1 %** (DS §5.5.5).
  Combined with the probe's ±1 % R25 / ±1 % B-value (TS §3-1/3-2), the bias network and ADC
  reference must be selected so the *electronics* contribution stays inside that 1 % budget.
- Over the clinically relevant 40–60 °C mixed-water band the probe presents roughly
  **5.34 kΩ → 2.48 kΩ** (TS R-T table, §10 below); the bias divider and ADC range should be
  centred on this band for best resolution while still resolving the full 0–65 °C ambit.
- Self-heating must be limited: at 5 mW rating and δ ≥ 0.8 mW/°C (TS §3-4/3-7), sense current is
  chosen to keep self-heating error well under the 1 % circuitry budget.
- τ ≤ 12 s (TS §3-3) bounds how quickly the probe tracks a step change; this informs the on-device
  sampling/averaging window during a flow event (§4.2) and supports the Design Scope's 5 s
  post-stabilisation compliance reading (DS §4.6).

## 3.7 Flow-switch input

Flow presence is sensed by the TMV outlet's mechanical **reed flow switch** (DS §5.5.5). With no
flow the reed contact is open; flowing water drives the magnetic piston, compresses the spring,
closes the reed contact and so closes the thermistor measurement circuit (DS §5.5.5). The input
must:

- Detect "Flow" when the switch is active for ≥ 1 s (DS §5.1.3).
- Apply **debounce in hardware or firmware** to reject reed contact bounce (DS §5.5.5, §3.7).
- Gate thermistor sampling on flow state so temperature is only logged during genuine flow
  (DS §5.5.5).

## 3.8 Solenoid driver output

The device provides a dedicated output to drive an **external DC solenoid valve** — the
reference valve being the **Wallgate WVPB400-2 (WG)**, an inline brass compression-fitting
solenoid supplied with a flying lead. The valve terminates in a **female mini-XLR**, so the
device side uses a mating **male 3-pin mini-XLR** connector (FL 1.0; cf. Wallgate WDC-100-NX
cable). Driver requirements (DS §5.5.2):

| Requirement | Specification | Source |
|-------------|---------------|--------|
| Supported solenoid supply | 6 V to 24 V DC | DS §5.5.2 |
| Voltage selection | Configurable from the cloud (one hardware platform, many valves) | DS §5.5.2, §4.10 |
| Source of solenoid power | **PoE** in the field; USB-C for test/commissioning (refines DS §5.5.2) | FL 7.0/8.0; DS §5.5.2 |
| Connector | Male 3-pin mini-XLR (valve presents female mini-XLR) | FL 1.0; WG |
| Protection | Current limiting, flyback/free-wheeling, predictable behaviour under open-load and short-circuit | DS §5.5.2 |
| Buffering | Local conditioning so activation does not disturb the supply | DS §5.5.2 |

The configured solenoid voltage (`SOL_VDC`) is stored on the device and selectable from a
provided option list (DS §4.8, §4.10). The PoE power budget for the valve load is the key open
item (§7; FL 7.0).

## 3.9 GPIO & expansion

- **2 × GPIO (DS §5.5.1):** reserved for future use — IR activation, device lockout,
  piezo/pushbutton activation, or simulating a push-button for an external controller (Galvin
  PB2). Where **PWM** is available it is preferred, to allow flow-meter or pressure-sensor inputs.
- **RJ11 expansion port (DS §5.5.4):** headroom for future development, including moving from a
  1:1 to a many-TMV-to-one-device topology. The intended bus to **remote I/O cards** is
  **BACnet or Modbus** — Modbus preferred where the link should be hidden from third-party
  access, BACnet where it should be open (FL 5.0). Two architecture questions remain open
  (§7): whether remote I/O cards carry their own power supply (PoE may not cover them), and how
  many remote cards attach to one master device.

<div class="page-break"></div>

# 4. Firmware Specification

The Design Scope draws an explicit line between **device logic** and **cloud logic** in its flow
and service-button diagrams (DS §5.1.5–§5.1.8). This section specifies the device side only.
Cloud-owned computation — temperature classification (`TEMP_CLASS`), maintenance-date
arithmetic, stagnation counting across groups, alert/email dispatch — is named here purely as the
interface the firmware feeds (DS §4.9–§4.13, §5.3–§5.4).

## 4.1 Firmware role

The transmitter is a **telemetry and execution node** (DS §6.1.2). Safety-critical thresholds
(scald limit, stagnation, alert rules) live in the cloud; the firmware measures, computes
per-event summaries, applies cloud-pushed configuration, and executes commanded actions. It does
not make independent safety decisions.

## 4.2 Event measurement & packet computation

A packet is emitted to the cloud on each qualifying flow event or service-button press; the cloud
extracts its fields into the asset table (DS §4.8). Temperature values are **computed on the
device**, with sampling/averaging during flow chosen for accuracy (DS §4.8). Per event the
firmware computes (DS §4.6, §4.8):

| Field | Definition | Source |
|-------|------------|--------|
| `MIN_TEMP` / `MAX_TEMP` / `AVG_TEMP` | Min / max / average temperature, evaluated from 20 s into the flow event | DS §4.6, §4.8 |
| `COMP_TEMP` | Average of readings at 5 s after the stabilisation period — the compliance reading | DS §4.6, §4.8 |
| `PEAK_TIME` | Time (s) within the event at which peak temperature occurred | DS §4.8 |
| `FLOW_DUR` | Total flow-event duration (s) | DS §4.8 |
| `SCALD_YN` / `SCALD_TIME` | Scald flag and accumulated seconds above the scald limit (§4.4) | DS §4.8 |
| `FS_ERROR` | Flow switch open > 2 h (§4.3) | DS §4.8 |
| `EVENT_TIME` | Event timestamp | DS §4.8 |

## 4.3 Flow-event state machine

Gating rules the firmware enforces locally (DS §5.1.1, §3.3, Appendix flow scenarios):

- **Stabilisation (`STABLE`, default 20 s):** readings in the first `STABLE` seconds are ignored
  before data recording. If an event is shorter than `STABLE`, only flow-duration is sent (DS
  §5.1.1, §4.11).
- **Flow reject (`FLOW_REJECT`, default 5 s):** if event duration < `FLOW_REJECT`, no packet is
  sent (DS §5.1.1, §4.11).
- **Flow-switch error:** a flow event longer than 2 h raises `FS_ERROR` (DS §5.1.1, §4.8).
- **Compliance window:** flow longer than 25 s (`STABLE` + 5 s) yields a captured `COMP_TEMP` for
  cloud classification (DS §5.1.5).

`STABLE` and `FLOW_REJECT` are cloud-pushed group parameters the device must mirror and update
when they change (DS §4.8, §4.11).

## 4.4 On-device safety & event logic

- **Scald detection (DS §5.1.2):** when measured temperature exceeds `TEMP_SCALD`, the firmware
  starts a timer; if it stays above the limit for longer than `SCALD_TIME` it sets the scald flag
  in the packet and records the scald duration. The threshold and duration are cloud-defined
  (DS §4.12); the device only measures and flags.
- **Hot-flush measurement (DS §5.1.4):** the firmware supplies the temperature and duration from
  which a hot-flush (e.g. > 60 °C for 10 min, or > 70 °C for 5 min) is recognised; the flush-date
  ledger itself is cloud logic.
- **Service button (DS §5.1.7, §4.6):** a **short (1 s)** press logs general/annual maintenance —
  Status LED flashes once; a **long (3 s)** press logs 5-year service — Status LED flashes three
  times. The press event and timestamp are sent; the maintenance *dates* are set in the cloud.
  The Design Scope flags the 1 s / 3 s scheme as error-prone and asks for a more robust,
  reversible scheme (DS §5.1.7) — carried to §7.

## 4.5 Commissioning & calibration

Commissioning is performed on-site over BLE with the CliniMix Setup app, fully offline-capable,
storing values on the transmitter until the cloud is reachable (DS §5.2, §7-app). The firmware:

- Exposes the device GUID via the QR linkage for asset binding (DS §6.1.3).
- Stores the **Commissioning Offset (`OFFSET`)** produced by the calibration wizard: the plumber
  runs the outlet ≥ 1 min, reads a calibrated reference thermometer, and the app computes the
  offset that aligns the digital probe to the physical reading (DS §3.2, §7-app, §4.9). `OFFSET`
  is applied to subsequent readings on-device.
- Accepts the **Operational Performance Setpoint (`TEMP_SP`)** and **IP configuration** captured
  during setup (DS §4.9, §4.10).

## 4.6 Timekeeping

The device maintains an RTC but treats **cloud time as authoritative**; all timestamps are
synchronised to cloud date/time, with a configurable time zone (DS §4.1). The RTC preserves
ordering and event timing through network or power interruptions.

## 4.7 Networking & connectivity

- **Addressing:** DHCP by default, static IP configurable via the app (DS §4.7).
- **Architecture:** device direct to cloud over Ethernet; functions without on-site servers or
  controllers (DS §4.7).
- **App link:** BLE for commissioning; optional Wi-Fi (DS §4.4).

## 4.8 Offline buffering

When the cloud is unreachable, all event packets and service-button records are **retained
locally** and uploaded when connectivity returns. On reaching the storage limit the firmware
applies a **first-in, first-out (FIFO)** policy, discarding the oldest record to admit the newest
(DS §5.2). Local buffer depth is an open item pending the packet-memory budget (DS §4.1 "Packet
Memory: TBD"; §7). The feedback loop also raises **optional MCU SD-card storage** to retain
longer sensor-history data on-device (FL 9.0, pending dev); if adopted, it provides storage for
both the offline buffer and extended history.

## 4.9 Security & OTA

- **Encrypted device-to-cloud pathways** within a direct device-to-cloud architecture that
  removes intermediary servers and reduces attack surface (DS §4.7, §6).
- **Secure over-the-air (OTA) firmware updates**, so fixes and improvements deploy without
  physical access (DS §4.7, §6).
- **Immutable identity:** the firmware binds permanently to the manufacturer GUID surfaced by the
  QR code, preserving history across name/location changes and guarding against asset
  substitution (DS §6.1.3).
- **Constrained local configuration:** because safety parameters are cloud-managed, the device
  inherently limits unauthorised local change (DS §6.1.2).

## 4.10 Solenoid control firmware (optional)

Active only when **Flow** or **Both** mode is selected at commissioning (DS §5.1.8). The firmware
supports (DS §5.1.8):

- **Automatic activation:** periodic flush (no activation for a set duration), scheduled purge
  (e.g. 19:00 daily), and stagnation flush (when the group stagnation counter reaches threshold).
- **Input lockout:** physical inputs (piezo/sensor) can be remotely locked from the portal;
  locked inputs are ignored until cleared.
- **Runtime configuration:** per-activation open duration (`SOL_DUR`) configurable from the Setup
  Tool or portal (DS §4.9).
- **Usage quotas / anti-vandal:** a maximum number of activations per minute / hour / day; once
  reached the input is disabled until the window resets.
- **Activation sources:** cloud push, automated logic, or the Setup Tool, even with no physical
  trigger fitted. Each activation records `SOL_EXEC_TIME`, `SOL_DUR` and `SOL_TRIGG` (DS §4.6,
  §4.9).


# 5. Compliance & Standards

| Area | Requirement | Source |
|------|-------------|--------|
| Emissions | EN IEC 61000-6-3:2021 | DS §4.5 |
| Product safety | AS/NZS 62368.1 | DS §4.5 |
| EMC (radiated/conducted) | AS/NZS CISPR 32 | DS §4.5 |
| Regulatory mark | RCM (Australia) | DS §4.3 |
| Radio (BLE / optional Wi-Fi) | AS/NZS 4268 | platform target |
| Additional targets (platform) | CE, FCC, EMC per CISPR 32 / IEC 61000-4-x | platform target |
| TMV maintenance intervals referenced by logic | AS 4032.3 (annual `MAINT_1` = 365 d; 5-yearly `MAINT_5` = 1825 d) | DS §4.9, §5.1.6 |

Exact test plans are to be confirmed with the supplier (DS §4.5). The radio and broader
CE/FCC/EMC marks reflect the Nube iO ACX platform certification set.

# 6. Key Reference Components

| Function | Reference part / platform | Notes | Source |
|----------|---------------------------|-------|--------|
| Platform | Nube iO ACX (drawing ACX-001 Rev V01) | 8 × I/O, PoE/12 VDC, RTC, opt. RS-485/LoRa, IPx5 | PO |
| Application MCU + BLE/Wi-Fi | Application microcontroller | BLE for Setup app; optional Wi-Fi | DS §4.4; PO |
| Network + power | 100 Mb Ethernet PHY + IEEE 802.3 PoE PD | PoE class per budget incl. valve (§7) | DS §4.1; PO; FL 7.0 |
| Temperature probe | EXSENSE TS103F25C3950FA-ML1300A | 10 kΩ NTC, B3950, SS304 NPT1/8, 1300 mm; 2-pin IP65 plug | TS; FL 2.0 |
| Solenoid valve (driven load) | Wallgate WVPB400-2 | Inline brass DC solenoid; female mini-XLR lead | WG; FL 1.0 |
| Solenoid power | PoE (field) / USB-C (test) | 6–24 V DC configurable | FL 7.0/8.0; DS §5.5.2 |
| Timekeeping | On-board RTC | Cloud-disciplined | DS §4.1 |


# 7. Open Items & Engineering Recommendations

Items the Design Scope records as *TBD* or that the live **Hardware Development Feedback Loop
(FL)** is tracking, with Nube iO's position and the current status. Resolving these is the
purpose of this design cycle; once Galvin's inputs (FL items marked *Pending Client*) are
received, Nube iO expects to complete layout and schematic within ~2 weeks (FL 6.0).

| # | Open item | Nube iO position / recommendation | Status |
|---|-----------|-----------------------------------|--------|
| 1 | **PoE power budget — must power the valve(s)** (DS §4.1; FL 7.0/8.0) — High | Complete the power budget (MCU + PHY + valve drive); confirm a single PoE class covers device + valve actuation, with USB-C carrying enough for commissioning checks. Sizing-critical. | Pending Client |
| 2 | **Valve connector** (DS §4.2; FL 1.0) | Valve presents a female mini-XLR → use a **male 3-pin mini-XLR** on the device; document pinout/polarity for the 6–24 V range. | Pending Client |
| 3 | **Temperature-probe connector** (DS §4.2; FL 2.0) | Adopt a **2-pin IP65 waterproof plug** with strain relief, mating the EXSENSE 1300 mm UL4478 lead (TS §1). | Pending Dev |
| 4 | **RJ45 connection / waterproofing** (DS §4.2; FL 3.0) | Choose among external RJ45 port, IP67 connector, sealed cable, or internal-connect-then-close per the final box design. | Pending Client |
| 5 | **Product box dimensions & internal layout** (DS §4.3; FL 4.0) — High | Need Galvin's detailed cabinet dims and component layout to finalise connector placement and the ACX casing (reconcile DS 60×120×40/IPx4 vs ACX-001 150×40×35/IPx5). | Pending Client |
| 6 | **Expansion / remote I/O cards** (DS §5.5.4; FL 5.0) | Bus is **BACnet or Modbus** (Modbus to hide from third parties). Confirm whether remote cards self-power and how many attach per master. | Pending Client |
| 7 | **MCU SD-card / history storage** (DS §4.1, §5.2; FL 9.0) | Optional SD card to hold extended sensor history plus the FIFO offline buffer; size for ≥ 30 days of events per asset on non-volatile storage. | Pending Dev |
| 8 | **Service-button robustness** (DS §5.1.7) | Add confirmation feedback (LED + cloud echo) and make service entries reversible by authorised users only; consider an app-confirmed action over bare 1 s / 3 s hold-times. | Nube iO rec. |
| 9 | **Measurement accuracy budget** (DS §5.5.5 vs TS §3) | Lock bias/ADC reference and sense-current so electronics stay < 1 %; state one end-to-end figure covering probe + electronics + self-heating. | Nube iO rec. |
| 10 | **Enclosure colour / markings & data retention** (DS §4.3, §6) | Confirm Galvin Blue PMS-2935 + mark set; device-side retention bounded by the FIFO/SD buffer, long-term retention is a cloud responsibility. | Pending Client |

# Appendix A — Requirements Traceability

Maps Galvin Design Scope clauses to the responsible domain. **HW** / **FW** = Nube iO scope (this
document); **Cloud** = Galvin/Rubix, interface only.

| Design Scope clause | Item | Owner | This doc |
|---------------------|------|-------|----------|
| §4.1 | Power, RTC, environmental, IP | HW | §3.1, §3.3, §4.6 |
| §4.2 | Connectors | HW | §3.4 |
| §4.3 | Enclosure, LEDs, button, labelling | HW | §3.5 |
| §4.4 | Communication (LAN / BLE / Wi-Fi) | HW/FW | §3.2, §4.7 |
| §4.5 | Compliance testing | HW | §5 |
| §4.6, §4.8 | Data acquisition & packet fields | FW | §4.2 |
| §4.9–§4.13 | Cloud parameters / thresholds | Cloud | interface — §4.1 |
| §5.1.1 | Flow gating (STABLE, FLOW_REJECT, 2 h) | FW | §4.3 |
| §5.1.2 | Scald detection | FW | §4.4 |
| §5.1.3 | Flow / low-flow detection | FW | §3.7, §4.3 |
| §5.1.4 | Hot-flush measurement | FW (Cloud ledger) | §4.4 |
| §5.1.5 | Over-temp / COMP_TEMP capture | FW (Cloud classifies) | §4.3 |
| §5.1.6 | Maintenance interval arithmetic | Cloud | interface — §4.4 |
| §5.1.7 | Service button | FW | §4.4 |
| §5.1.8 | Solenoid & input control | FW | §4.10 |
| §5.2 | Offline FIFO buffering | FW | §4.8 |
| §5.3–§5.4 | Alerts / reports / email | Cloud | interface — §4 intro |
| §5.5.1 | GPIO / PWM | HW | §3.9 |
| §5.5.2 | Solenoid output driver | HW/FW | §3.8, §4.10 |
| §5.5.3 | USB-C input / power path | HW | §3.3 |
| §5.5.4 | Expansion port (RJ11) | HW | §3.9 |
| §5.5.5 | Flow switch & thermistor | HW | §3.6, §3.7 |
| §6 | Security, OTA, data sovereignty | FW (Cloud hosting) | §4.9 |

# Appendix B — Thermistor R–T Reference (operating band)

Extract from the EXSENSE datasheet R-T table (TS §6), nominal resistance, covering the device's
0–65 °C operating ambit (DS §4.1) and highlighting the 40–60 °C mixed-water band.

| Temp (°C) | R nominal (kΩ) | Temp (°C) | R nominal (kΩ) |
|-----------|----------------|-----------|----------------|
| 0 | 31.898 | 40 | 5.338 |
| 5 | 25.074 | 45 | 4.373 |
| 10 | 19.739 | 50 | 3.591 |
| 15 | 15.665 | 55 | 2.977 |
| 20 | 12.480 | 60 | 2.477 |
| 25 | 10.000 | 65 | 2.078 |
| 30 | 8.075 | — | — |
| 35 | 6.538 | — | — |

# Appendix C — Definitions (device-relevant subset)

**TMV** Thermostatic Mixing Valve · **PoE** Power over Ethernet · **USB-PD** USB Power Delivery ·
**RTC** Real-Time Clock · **NTC** Negative Temperature Coefficient · **GPIO** General-Purpose
Input/Output · **PWM** Pulse-Width Modulation · **OTA** Over-the-Air · **DHCP** Dynamic Host
Configuration Protocol · **FIFO** First-In, First-Out · **GUID** Globally Unique Identifier ·
**BLE** Bluetooth Low Energy · **LAN** Local Area Network · **RCM** Regulatory Compliance Mark
(Australia). (Source: DS §8.)
