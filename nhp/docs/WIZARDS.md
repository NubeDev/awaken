# NHP — Onboarding Wizards

Wizards make adding infrastructure fast and consistent: they author many
gate-audited records (and their tags) in one guided flow, applying NHP conventions
(tagging, device-limit checks, meter-type stamping) so the dashboards auto-build
correctly. Every wizard step is an ordinary collection write — the wizard is UI +
orchestration, not new backend.

## Principles

- **Batched, atomic-ish, resumable.** A wizard collects input, previews the records
  it will create, then writes them. Each write crosses the gate (audited); a failed
  step reports per-record and the wizard can resume rather than restart.
- **Conventions applied for you.** Wizards attach the standard tags
  (`tenant:…`, `site:…`, `gateway:…`, `group:…`) so dashboards build without manual
  tagging (see [DASHBOARDS.md](./DASHBOARDS.md)).
- **Validation up front.** Device-limit (`max_devices`), unique keys, and enum
  fields are checked before write, not after.
- **Stamp from templates.** Meter steps stamp registers from the chosen meter-type
  (and record `meter_type_version`).

## Wizards

### 1. New tenant (root/system)
Onboard a customer end-to-end:
1. Tenant (creates/owns a **namespace** via rubix `/tenants`).
2. First **admin** user (principal + grants).
3. Optional first site.
4. Optional: register the **polling service** service-account for this tenant.

### 2. New site
Add a site under a tenant: name, address, **timezone** (drives dashboard local
time), geo. Offers to chain into the gateway wizard.

### 3. New gateway (with N networks) — the "30 networks" flow
The headline bulk wizard:
1. Gateway: name, model, `host`, parent site.
2. **Bulk networks**: "add **30** networks" in one step — choose `net_type`
   (`485`/`ethernet`), `protocol`, a `max_devices` cap, a naming pattern
   (`gw-01-net-{n}`), and per-type `params` defaults (baud/parity, or ip/port
   range). The wizard generates all N network records + tags in one batch.
3. Preview the N records, confirm, write.

### 4. New meters (bulk, onto a network)
1. Pick the parent network (shows remaining capacity = `max_devices` − current).
2. Pick a **meter-type** (the register template).
3. Add meters: a list or a range of bus `address`es (e.g. units 1–20), name
   pattern. The wizard **blocks** anything that would exceed `max_devices`.
4. Each meter is stamped from the type: its `register` records are created, tagged
   (`meter:…`, `group:…`, `quantity:…`), `history` flags carried from the type.

### 5. New user / team
Add a user (principal) or team to a tenant, assign a **role** (viewer/operator/
admin, see [ADMIN.md](./ADMIN.md)) and team membership. Uses rubix
`/principals` + grants.

### 6. (Admin) New meter-type
Not strictly onboarding, but wizard-shaped: guided register-map authoring —
manufacturer, then add/clone/bulk-paste registers with their protocol metadata,
units, history flags, chart types, groups, and alarms. See [ADMIN.md](./ADMIN.md).

## "Add everything" combined flow

For a greenfield customer, a single combined wizard chains 1→2→3→4→5:
tenant → site → gateway(+networks) → meters → users. Each sub-step is the wizard
above; the combined flow just threads the parent ids and shows one final preview of
the full tree before writing. On completion the customer's dashboards are already
auto-built from the tags the wizard applied.

## After a wizard runs

- Records exist, tagged; **dashboards auto-build** immediately (DASHBOARDS.md).
- `status`/`last_seen`/values stay `unknown`/empty until the **polling service**
  picks up the new definitions and writes back — NHP shows "awaiting first poll".
