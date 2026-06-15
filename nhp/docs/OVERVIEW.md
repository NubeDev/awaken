# NHP — Overview

NHP is a **power-metering management platform**: a thin domain layer on top of the
existing **rubix** backend, with a UI ported from `rubix-old/ui`. It manages the
*configuration and presentation* of metering infrastructure — tenants, sites,
gateways, networks, meters, registers, units, dashboards, and users — but it does
**not** talk to hardware. There is no polling, no protocol I/O, no Modbus master.
Those live in a separate service that *consumes* the configuration NHP stores.

> **The split that defines the project.** NHP *describes* the world (a gateway on
> a 485 network has a meter at unit 5, whose register 30013 is a voltage in volts,
> kept as history, charted as a line). A separate polling service *reads* that
> description and does the talking. NHP owns the model and the screens; it never
> owns the wire.

## Why rubix is the backend

The rubix [SCOPE.md](../../rubix/docs/SCOPE.md) is deliberately domain-free:
*"the domain is not baked in (no equipment / site / point schema); structure comes
from tagging on a graph."* That is exactly what NHP needs — the entire NHP domain
is **data**, not backend code:

- Every entity type (tenant, site, gateway, network, meter, meter-type, register)
  is a **collection record** (`kind: "collection"`), defined at runtime with typed
  fields and validation. No Rust type, no table, no route per entity.
- Parent/child structure is **relations + tag edges** on the graph.
- Dashboards are **auto-built** from those tags.
- Units, history-on/off, chart type, chart grouping, alarms are **fields and
  tags** on register/meter records.

What NHP adds is: the collection definitions, the onboarding wizards, the
dashboard auto-build rules, a seed of mock data, and the UI. The hard substrate
(auth, multi-tenancy, gate, audit, query, realtime) already exists.

## The documents

| Doc | Covers |
| --- | --- |
| [DOMAIN-MODEL.md](./DOMAIN-MODEL.md) | The entities, their fields, the relation/tag graph, the Modbus register metadata contract |
| [ADMIN.md](./ADMIN.md) | Back-of-house: meter types, gateway/network types, registers, units, chart grouping, roles, meter-type versioning |
| [WIZARDS.md](./WIZARDS.md) | Onboarding wizards (add tenant→site→gateway→meters→users; "30 networks" bulk add) |
| [DASHBOARDS.md](./DASHBOARDS.md) | Auto-built dashboard pages, chart grouping, online/offline stats, alarms/thresholds |
| [SEED.md](./SEED.md) | Mock data seed + Makefile (copied from rubix) |

The authoritative backend references are
[rubix/docs/SCOPE.md](../../rubix/docs/SCOPE.md),
[BACKEND-COLLECTIONS.md](../../rubix/docs/design/BACKEND-COLLECTIONS.md),
[ADMIN-API.md](../../rubix/docs/design/ADMIN-API.md), and
[DASHBOARDS-SCOPE.md](../../rubix/docs/design/DASHBOARDS-SCOPE.md). Where NHP and
rubix disagree, rubix's SCOPE wins.

## What's a meter, in one diagram

```
tenant ──< site ──< gateway ──< network ──< meter ──< register
                    (485 |        (modbus)   (device,   (modbus addr,
                     ethernet)                limited     unit, history y/n,
                                              per net)    chart type, alarms)

meter-type  : an admin-defined template — a named set of register definitions
              a meter instance is stamped from (e.g. "Acme PM5560").
tags        : voltage-group, current-group, site:acme, gateway:gw-01, …
              drive dashboard auto-build and chart grouping.
```

## Backend build-status (point #6 — verified 2026-06-16)

The rubix substrate is **further along than its design docs imply**. Verified
against the current code, not aspiration:

| Capability NHP needs | Status in rubix today | Source |
| --- | --- | --- |
| Collections as records (`kind:"collection"`) | ✅ built — `FieldType {Text, Number, Bool, Date, File, Relation}` | [collection/field.rs](../../rubix/crates/rubix-core/src/collection/field.rs) |
| Per-kind validation on write | ✅ wired into the gate write path; **fail-open** when no collection, strict-namespace option | [command/validate.rs](../../rubix/crates/rubix-gate/src/command/validate.rs) |
| `required` / `unique` field rules | ✅ built | collection/field.rs |
| Relation field (parent/child links) | ✅ built (`Relation` = target record id) | collection/field.rs |
| Principals (users + service accts) CRUD | ✅ `/principals` + `/principals/:subject/grants` | [http/admin/](../../rubix/crates/rubix-server/src/http/admin/) |
| Tenants (namespaces) onboarding | ✅ `/tenants` | http/admin/ |
| Device registry | ✅ `/devices` CRUD, `DeviceManage` capability, gate-audited | [http/admin/devices.rs](../../rubix/crates/rubix-server/src/http/admin/devices.rs) |
| Batch query (run a board at once) | ✅ `POST /query` + `POST /query/batch` (≤50) | [http/query/](../../rubix/crates/rubix-server/src/http/query/) |
| Audit / undo / correlation id | ✅ automatic at the gate | SCOPE.md |
| Live/realtime | ✅ `/ws/records`, row-filtered per principal | SCOPE.md |
| Unit conversion engine | ✅ `rubix-prefs` crate (metric↔imperial, unit registry) | rubix-prefs |

**Gaps to be aware of (fixable — you said you can patch rubix):**

1. **No `Select`/enum field type.** Closed enums NHP wants — network *type*
   (`485` / `ethernet`), *protocol* (`modbus`), register *datatype*, *chart type* —
   have no native field type. **Workaround now:** model them as `Text` with the
   allowed set enforced by a collection `writeRule` (or validated in the wizard).
   **Proper fix:** add a `Select { options }` variant to `FieldType` (a deliberate
   enum change in [field.rs](../../rubix/crates/rubix-core/src/collection/field.rs)
   — see BACKEND-COLLECTIONS open question 3). Recommended before launch; the enums
   are core to the admin UX.
2. **File field bytes are deferred.** The `File` field type and its reference shape
   exist, but the blob storage subsystem is not built
   (BACKEND-COLLECTIONS build-order step 6). NHP needs this only for things like a
   site floor-plan / gateway photo — not on the critical path.
3. **`rubix-prefs` is not wired to HTTP.** The converter exists; there is **no
   `GET/PATCH /prefs` endpoint** and the frontend consumes none of it yet
   (DASHBOARDS-SCOPE §2). Units display as raw labels until this is wired. NHP's
   per-register unit metadata (DOMAIN-MODEL) is unaffected — that's stored data —
   but live unit *conversion/formatting* waits on this endpoint.

None of these block the domain model or the docs below; they are noted so the
build plan budgets for them.
