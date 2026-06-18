# NHP — Domain Model

Every NHP entity is a **rubix collection record** (`kind: "collection"` defines the
shape; instances are records carrying that `kind`). Nothing here is a new table or
Rust type — it is all data defined through the gate. See
[BACKEND-COLLECTIONS.md](../../rubix/docs/design/BACKEND-COLLECTIONS.md) for the
collection mechanism and [OVERVIEW.md](./OVERVIEW.md) for the build-status caveats
(notably: no native `Select` type yet — closed enums below are modelled as `text`
with a `writeRule` until a `Select` field type is added).

## The hierarchy

```
tenant ──< site ──< gateway ──< network ──< meter ──< register
                                              ▲
                                       meter-type (template)
```

Parent links use the rubix **`relation`** field type (the child stores the
parent's record id). Every record also gets **tag edges** (`record -> tagged ->
tag`) that the dashboard auto-build and chart grouping read (see
[DASHBOARDS.md](./DASHBOARDS.md)).

## Entities

### `tenant`
A customer / organisation. Maps to a rubix **namespace** for isolation (cloud
profile) — onboarding a tenant uses the rubix `/tenants` surface
([ADMIN-API.md](../../rubix/docs/design/ADMIN-API.md) §3).

| field | type | notes |
| --- | --- | --- |
| `key` | text, required, unique | stable slug, e.g. `acme` |
| `name` | text, required | display name |
| `namespace` | text | the rubix namespace this tenant owns |

### `site`
A physical location belonging to a tenant.

| field | type | notes |
| --- | --- | --- |
| `key` | text, required, unique | |
| `name` | text, required | |
| `tenant` | relation → tenant | parent |
| `address` | text | |
| `timezone` | text | IANA tz; dashboards show site-local time |
| `geo` | text | optional lat,lng |

### `gateway`
A field device that hosts network ports and bridges meters to the polling service.
A gateway has **unlimited network ports** but each network enforces a **device
limit** (below).

| field | type | notes |
| --- | --- | --- |
| `key` | text, required, unique | |
| `name` | text, required | |
| `site` | relation → site | parent |
| `model` | text | gateway hardware model |
| `host` | text | address the *polling service* will use (NHP only stores it) |
| `status` | text | `online` / `offline` / `unknown` — written by the poller, read by NHP |
| `last_seen` | date | written by the poller |

### `network`
A communications bus on a gateway. This is where the **network type** and
**protocol** live, and where the **device limit** is enforced.

| field | type | notes |
| --- | --- | --- |
| `key` | text, required, unique | e.g. `gw-01-net-1` |
| `name` | text | |
| `gateway` | relation → gateway | parent |
| `net_type` | text (enum: `485`, `ethernet`, `lora`) | the physical/link layer |
| `protocol` | text (enum: `modbus`, `lora`) | Modbus on a 485/ethernet bus; LoRaWAN on a `lora` radio link |
| `max_devices` | number, required | **device cap** — the wizard/writeRule rejects an N+1th meter |
| `params` | json | net-type-specific: baud/parity/stop for 485; ip/port for ethernet; region/spreading_factor for lora |

> **LoRa networks (extra devices).** A `lora` network is the gateway's LoRaWAN
> radio link; a battery sensor is "matched to a gateway" by sitting on it. LoRa
> devices are modelled as meter-types too (seed/device-types.mjs) and stamped
> through the same meter/register pipeline — their registers are decoded from the
> uplink payload (`fn_code: lora_uplink`, `address` = a logical channel) rather
> than Modbus-addressed, and each carries a `battery` register with a low-battery
> alarm. **Modbus IO** devices (pulse input, on/off coil, read/write holding
> register) are ordinary Modbus meter-types on a 485/ethernet bus.

> **Device limit.** `max_devices` is the per-network cap (point #1 from the
> kickoff). A gateway's *total* device count is the sum across its networks and is
> derived, not stored. Enforcement: the add-meter wizard checks the count, and a
> collection `writeRule` on `meter` rejects a create that would exceed the parent
> network's `max_devices` (defence in depth). The number of networks per gateway is
> **unlimited** by design.

### `meter`
A metering device on a network. A meter is **stamped from a `meter-type`** — it
inherits that type's register set at creation (see versioning below).

| field | type | notes |
| --- | --- | --- |
| `key` | text, required, unique | |
| `name` | text, required | |
| `network` | relation → network | parent |
| `meter_type` | relation → meter-type | the template it was stamped from |
| `meter_type_version` | number | the type version stamped at creation (versioning, below) |
| `address` | number, required | unit/slave address on the bus (e.g. Modbus unit id) |
| `status` | text | `online` / `offline` / `unknown` — written by the poller |
| `last_seen` | date | written by the poller |

### `meter-type`
The **admin-defined template**: a named set of register definitions a meter is
stamped from (e.g. "Schneider PM5560"). This is the heart of the back-of-house —
see [ADMIN.md](./ADMIN.md).

| field | type | notes |
| --- | --- | --- |
| `key` | text, required, unique | |
| `name` | text, required | |
| `manufacturer` | text | |
| `version` | number, required | bumped on every edit (versioning, below) |
| `registers` | json (array of register-def) | the template register set (shape below) |

### `register`
A single readable/writable point on a meter — **the Modbus metadata contract that
the polling service consumes** (point #2). NHP never reads these values over the
wire; it stores the definition so the poller knows *what* to read and *how* to
interpret it, and so dashboards know *how* to present it.

A register exists in two forms:
- **register-def** — the entry inside a `meter-type.registers[]` template.
- **register** — the concrete record under a `meter`, stamped from the def, that
  history and charts attach to.

Both share these fields:

| field | type | group | notes |
| --- | --- | --- | --- |
| `key` | text, required | id | e.g. `voltage_l1` |
| `name` | text, required | display | "Voltage L1" |
| **— protocol metadata (consumed by the poller) —** | | | |
| `address` | number, required | modbus | register/coil address, e.g. `3027` |
| `fn_code` | text (enum: `read_holding`, `read_input`, `read_coil`, `read_discrete`, `write_holding`, `write_coil`) | modbus | Modbus function |
| `datatype` | text (enum: `int16`, `uint16`, `int32`, `uint32`, `float32`, `float64`, `bool`, …) | modbus | how to decode the raw register(s) |
| `word_count` | number | modbus | registers spanned (derivable from datatype; stored explicit) |
| `byte_order` | text (enum: `big`, `little`, `big_swap`, `little_swap`) | modbus | word/byte endianness |
| `scale` | number | modbus | multiplier applied to raw value (e.g. `0.1`) |
| `offset` | number | modbus | additive offset after scale |
| `signed` | bool | modbus | (when datatype is ambiguous) |
| **— semantics & presentation (consumed by NHP/dashboards) —** | | | |
| `unit` | text | display | unit id (V, A, kW, kWh, Hz, …) — feeds `rubix-prefs` registry |
| `quantity` | text | display | physical quantity for unit conversion (Voltage, Power, Energy) |
| `history` | bool, required | history | **whether the poller persists history for this register** (point: admin sets this) |
| `chart_type` | text (enum: `line`, `bar`, `area`, `stat`, `gauge`, `table`) | display | default render |
| `chart_group` | text | display | grouping key (e.g. `voltage`) — also mirrored as a tag |
| `precision` | number | display | decimals |
| **— alarms (point #3) —** | | | |
| `alarm` | json (thresholds) | alarm | optional threshold ramp; see ALARMS below |

> **Why store protocol metadata if we don't poll?** (point #2) Because the
> separate polling service consumes it. NHP is the system of record for "register
> 3027 on this meter is a `float32`, big-endian, scale 0.1, a Voltage in V, keep
> history, alarm if >253". The poller reads that, talks Modbus, and writes values +
> status back. Keeping it as data (not protocol code) is the whole point of the
> rubix substrate.

## Tagging (drives dashboards & grouping)

Every record carries tag edges. Conventions NHP relies on:

| tag pattern | purpose |
| --- | --- |
| `tenant:<key>`, `site:<key>`, `gateway:<key>`, `network:<key>`, `meter:<key>` | hierarchy membership — the dashboard auto-build walks these |
| `group:<chart_group>` (e.g. `group:voltage`) | chart grouping — all voltages in one chart |
| `quantity:<q>` (e.g. `quantity:power`) | cross-cut a quantity across meters |
| `meter-type:<key>` | every meter of a type |

Tags are how a page "all voltages on this meter in one chart" is built without a
fixed schema (see [DASHBOARDS.md](./DASHBOARDS.md)).

## Alarms / thresholds (point #3)

Alarms reuse the dashboard **FieldConfig threshold ramp** from
[DASHBOARDS-SCOPE.md](../../rubix/docs/design/DASHBOARDS-SCOPE.md) §7 rather than
inventing a parallel model — the same `{ value → colour/severity }` steps both
paint the chart and define the alarm. A register's `alarm` field:

```jsonc
{
  "thresholds": [
    { "value": null, "severity": "ok" },        // baseline
    { "value": 250,  "severity": "warning" },    // ≥250 V → warn
    { "value": 253,  "severity": "critical" }    // ≥253 V → critical
  ],
  "for": "5m"     // optional dwell before firing (hysteresis)
}
```

A ramp may set `"direction": "below"` to fire as the value **falls** into a step
(`value <= step`) instead of the default `"above"` (`value >= step`). This is how a
LoRa device's **low-battery** alarm is expressed (warn ≤30%, critical ≤15%). The
direction is honoured by the single `severityFor` both the chart colouring and the
alarm panel use, so "what you see is what alarms" holds for either direction.

- **Evaluation** is a rubix **rule** (Rhai) over the register's history/live value;
  it writes an `insight`/alarm record on cross, published as a data-change event.
  NHP defines the *thresholds as data*; rubix's rule engine fires them.
- The same thresholds drive the chart's colour ramp, so what you see is what
  alarms.
- Alarm state surfaces on dashboards alongside online/offline status
  (see [DASHBOARDS.md](./DASHBOARDS.md)).

## Meter-type versioning (point #4)

Editing a meter-type must not silently mutate every deployed meter.

- **Stamp-on-create.** A meter records `meter_type` (the relation) **and**
  `meter_type_version` (the integer version at stamp time). The meter's own
  `register` records are copied from the type's `registers[]` at creation — the
  meter owns its registers thereafter.
- **Editing a type bumps `version`** and does **not** touch existing meters. New
  meters stamped after the edit get the new version.
- **Re-sync is explicit.** A "re-apply meter-type" action (admin or wizard) diffs
  the meter's registers against the current type version and applies adds/changes
  on confirmation — never automatically. The diff and the apply both cross the gate
  (audited, undoable).
- This makes a meter-type a **template**, not a live binding: drift is allowed and
  visible (`meter_type_version` < type's `version` ⇒ "out of date" badge).

## Status fields are poller-owned

`gateway.status`, `meter.status`, `*.last_seen`, and live register values are
**written by the polling service**, read by NHP. NHP's collections define and
validate them; NHP's UI displays them; NHP never produces them. This keeps the
"no protocol I/O in NHP" boundary clean while still rendering online/offline
(see [DASHBOARDS.md](./DASHBOARDS.md)).
