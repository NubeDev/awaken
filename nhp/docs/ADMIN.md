# NHP — Admin / Back-of-House

The admin section is where an operator defines the *types* and *rules* the rest of
the system is built from: meter types and their Modbus register maps, gateway
network types and protocols, per-register settings (units, history on/off, chart
type, grouping, alarms), roles, and meter-type versioning. Everything here is a
**gate-audited write to a collection record** — no per-feature backend code (see
[DOMAIN-MODEL.md](./DOMAIN-MODEL.md), [OVERVIEW.md](./OVERVIEW.md)).

## 1. Meter types (the register map)

The central admin task. A **meter-type** is a named template of register
definitions a meter is stamped from.

- **Create / edit a meter-type**: name, manufacturer, and the `registers[]` set.
- **Register editor** — per register, the admin sets every field from
  [DOMAIN-MODEL.md](./DOMAIN-MODEL.md) `register`:
  - **Protocol metadata** (consumed by the poller): `address`, `fn_code`,
    `datatype`, `word_count`, `byte_order`, `scale`, `offset`, `signed`.
  - **Semantics & presentation**: `unit`, `quantity`, `history` (on/off),
    `chart_type`, `chart_group`, `precision`.
  - **Alarm** thresholds (optional).
- **Import / clone**: clone an existing type as a starting point; bulk-paste a
  register table (CSV/JSON) for meters with dozens of registers.
- **Versioning**: every save **bumps `version`**; existing meters are untouched
  (see [DOMAIN-MODEL.md](./DOMAIN-MODEL.md) → versioning, point #4). The UI shows
  how many deployed meters are on older versions and offers a per-meter
  "re-apply type" diff.

## 2. History on/off (per register)

`register.history` is a boolean the admin sets per register (in the meter-type, or
overridden on an individual meter). It is the instruction to the **polling
service** to persist a time-series for that register. Registers with
`history = false` still poll for live value/alarm but keep no history — so charts
that need a trend only offer history-enabled registers.

## 3. Units

- Each register declares a `unit` id (V, A, kW, kWh, Hz, …) and a `quantity`
  (Voltage, Power, Energy) for conversion.
- Units back onto the **`rubix-prefs`** registry + converter (metric↔imperial).
  The conversion/formatting endpoint is **not yet wired** (OVERVIEW gap #3) — until
  it is, units render as fixed labels. Storing `unit`/`quantity` now means no
  rework when it lands.

## 4. Chart type & chart grouping

- **Chart type** per register: `chart_type` (`line`/`bar`/`area`/`stat`/`gauge`/
  `table`). This is the default render on auto-built dashboards.
- **Chart grouping** uses **tags** (the easy path, as suggested in the kickoff).
  Setting `chart_group: voltage` on registers tags them `group:voltage`; the
  dashboard auto-build renders all `group:voltage` series of a meter in **one
  chart** (e.g. V L1 / L2 / L3 overlaid). Grouping works across meters too
  (`quantity:power` across a site). See [DASHBOARDS.md](./DASHBOARDS.md).
- Group definitions can be authored at the meter-type level (every voltage register
  tagged `group:voltage`) so grouping is inherited by every stamped meter.

## 5. Gateway & network types

- **Network types** are a closed set: `net_type ∈ {485, ethernet}`,
  `protocol ∈ {modbus}` (protocol is single-valued today; the field exists so
  other protocols can be added without a schema change).
- **485** networks carry serial `params` (baud, parity, stop bits, data bits);
  **ethernet** networks carry `ip`/`port`. The admin/wizard form switches fields on
  `net_type`.
- **Device limit** per network: `max_devices` (required). Unlimited networks per
  gateway; capped meters per network (see [DOMAIN-MODEL.md](./DOMAIN-MODEL.md)).

> **Enum caveat.** `net_type`, `protocol`, register `datatype`/`fn_code`/
> `byte_order`, and `chart_type` are conceptually closed enums but there is no
> native `Select` field type in rubix yet (OVERVIEW gap #1). Until a `Select`
> variant is added, enforce the allowed set in the admin form **and** a collection
> `writeRule`. Adding `Select { options }` to `FieldType` is the clean fix and is
> recommended before launch.

## 6. Roles & access (point #5)

Built on rubix principals + role-in-namespace
([ADMIN-API.md](../../rubix/docs/design/ADMIN-API.md)). NHP defines this role set:

| Role | Can |
| --- | --- |
| **viewer** | read dashboards, meters, history, alarms — no writes |
| **operator** | viewer + acknowledge alarms, run wizards to add sites/gateways/networks/meters, edit instance-level settings (e.g. a meter's `history` override) |
| **admin** | operator + manage meter-types/register maps, network types, units/groups, users & teams within the tenant |
| **root/system** | cross-tenant: onboard new tenants (fresh namespace), register the polling service as a service-account principal |

- **Users & teams** are principals; team membership groups users for grant
  assignment. Management is the rubix `/principals` + `/principals/:subject/grants`
  surface — "user management" and "service-account management" are the same
  endpoints (ADMIN-API §"one identity model").
- **The polling service is a principal too** — a service account scoped to the
  tenant with grants to write `status`/`last_seen`/history/values and read the
  register definitions. It authenticates and is audited like any user.
- Every admin mutation crosses the gate: **audited, correlation-id'd, undoable**.

## 7. What admin does *not* do

- No live device communication, no test-poll, no register scan against real
  hardware — that is the polling service. Admin can define and validate a register
  map; it cannot read a live value except via the values the poller has already
  written back.
