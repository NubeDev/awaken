# Feature — Points & the 16-Level Priority Array

> Verified: **verified** — run live on `rubix-gaps` @ `d48dd8b2`, 2026-06-13
> (`RUBIX_ZENOH=0`, HTTP-only). Every ✅ gate confirmed against the backend; the
> backend was correct, several runbook commands were wrong and are fixed below
> (see "Known issues / fixes"). Source: `rubix-core/src/{model,priority,value}.rs`,
> `rubix-server/src/api/{sites,equips,points,command,his}`.

Covers: sites/equips/points CRUD with Haystack tag filters; the priority-array
command path (`write` / `relinquish` / `cur`); history accumulation. This is the
foundation every other feature stands on.

Prereq: stack up per [../00_setup/QUICKSTART.md](../00_setup/QUICKSTART.md). `$BASE`,
`post()`, `del()` from [../reference/API_CHEATSHEET.md](../reference/API_CHEATSHEET.md).

---

## What to prove

The priority array is BACnet semantics: **16 slots, lower level number wins**, with
a relinquish-default fallback. Every effective-value change lands in history. The
agent ceiling does **not** gate direct HTTP writes (operators are unrestricted).

---

## Runbook

### 1. Build the topology

```bash
SITE=$(post /api/v1/sites '{"org":"nube","slug":"hq","display_name":"HQ","tags":{"site":true}}' | jq -r .id)
EQUIP=$(post /api/v1/equips "$(jq -nc --arg s "$SITE" '{site_id:$s,path:"ahu-3",display_name:"AHU 3",tags:{"ahu":true}}')" | jq -r .id)
# `display_name` is REQUIRED on CreatePoint (omitting it → 422 "missing field display_name").
# NB: point create wraps its response as {keyexpr, point:{…}} — the id is `.point.id`,
# not `.id` (sites/equips return a bare object, so those use `.id`).
PT=$(post /api/v1/points "$(jq -nc --arg e "$EQUIP" '{equip_id:$e,slug:"sp",display_name:"Setpoint",kind:"sp",unit:"°C",tags:{"temp":true,"sp":true}}')" | jq -r .point.id)
```

> **Response shape:** point **create**, **GET**, **write**, and **relinquish** all
> return `{keyexpr, point:{…}}` — point fields (`id`, `kind`, `cur_value`,
> `priority_array`) are under `.point`, only `keyexpr` is top-level. The **list**
> endpoint (`GET /api/v1/points`) and **`/cur`** (sensor ingest) instead return the
> **bare** point object(s). Site/equip create return bare too. The jq paths below
> reflect that (`.point.id`, `.point.cur_value`, …).

✅ `GET /api/v1/points/$PT | jq .keyexpr` → `nube/hq/ahu-3/sp`.
✅ A `kind:"sp"` (or `"cmd"`) point exposes a 16-slot `priority_array`:
`jq '.point.priority_array.slots | length'` → `16`. A `kind:"sensor"` point *also*
serializes a 16-slot array (it is part of the struct), but is **read-only** — its
read-only-ness is enforced at the write endpoint (Step 6), not by array absence.

### 2. Tag filtering

```bash
# The filter param is a single comma-separated `tags=`, NOT repeated `tag=`.
# (A repeated/unknown `tag=` param is silently ignored → every point returns.)
curl -s "$BASE/api/v1/points?tags=temp,sp"       | jq '[.[].slug]'  # → ["sp"] only
curl -s "$BASE/api/v1/points?tags=temp"          | jq 'length'      # → 2 (both carry temp)
curl -s "$BASE/api/v1/points?tags=does-not-exist" | jq 'length'     # → 0
```

✅ The `temp,sp` filter returns only the `sp` point (has both); `temp` alone returns
both; a missing tag returns none. Markers match on **presence**, not value —
`has_all` semantics.

### 3. Priority-array writes — lower level wins

```bash
# write/cur/relinquish responses return the full point wrapped as {keyexpr, point:{…}}.
post /api/v1/points/$PT/write '{"priority":13,"value":22.0}' | jq '.point.cur_value'   # → 22.0
post /api/v1/points/$PT/write '{"priority":8,"value":18.0}'  | jq '.point.cur_value'   # → 18.0
curl -s $BASE/api/v1/points/$PT | jq '{cur:.point.cur_value, slot8:.point.priority_array.slots[7], slot13:.point.priority_array.slots[12]}'
```

✅ After the slot-8 write the effective `cur_value` is **18.0** — level 8 beats
level 13 (lower number wins). Slots 8 and 13 are both populated (`18.0` / `22.0`).

### 4. Relinquish restores the fallback

```bash
del /api/v1/points/$PT/write/8 | jq '.point.cur_value'   # → 22.0
curl -s $BASE/api/v1/points/$PT | jq '{cur:.point.cur_value, slot8:.point.priority_array.slots[7]}'  # cur 22.0, slot8 null
```

✅ Relinquishing slot 8 drops back to slot 13 → `cur_value` 22.0.

### 5. Range checking

```bash
curl -s -o /dev/null -w "%{http_code}\n" -X POST "$BASE/api/v1/points/$PT/write" -H content-type:application/json -d '{"priority":0,"value":1}'   # → 400
curl -s -o /dev/null -w "%{http_code}\n" -X POST "$BASE/api/v1/points/$PT/write" -H content-type:application/json -d '{"priority":17,"value":1}'  # → 400
```

✅ Both return `400` — body `{"error":"priority N out of range 1..=16"}`. Valid
range is `1..=16`.

### 6. Sensor ingest + history; command/sensor endpoint split

`/cur` is **sensor ingest only**; a writable (`sp`/`cmd`) point rejects it
(`400 "point is writable; use the write endpoint"`). Conversely a `sensor` rejects
`/write` (`400 "is a sensor and cannot be commanded"`). Ingest on the sensor point:

```bash
SEN=$(post /api/v1/points "$(jq -nc --arg e "$EQUIP" '{equip_id:$e,slug:"temp",display_name:"Temp",kind:"sensor",unit:"°C",tags:{"temp":true}}')" | jq -r .point.id)
post /api/v1/points/$SEN/cur '{"value":21.5}' | jq '.cur_value'        # → 21.5 (bare point, not wrapped)
curl -s "$BASE/api/v1/points/$SEN/his?limit=20" | jq 'length'          # → 1, grows per ingest

# the writable point accrues a his row on each write/relinquish (effective-value change):
curl -s "$BASE/api/v1/points/$PT/his?limit=20" | jq 'length'           # → 3 after steps 3–4

# endpoint split (both 400):
curl -s -o /dev/null -w "cur-on-sp %{http_code}\n"   -X POST "$BASE/api/v1/points/$PT/cur"  -H content-type:application/json -d '{"value":1}'
curl -s -o /dev/null -w "write-on-sensor %{http_code}\n" -X POST "$BASE/api/v1/points/$SEN/write" -H content-type:application/json -d '{"priority":8,"value":1}'
```

✅ Sensor `cur` ingest sets `cur_value` and appends a `his` row. Each effective-value
change on the writable point (write/relinquish) also appends a `his` row. `/cur` on a
writable point and `/write` on a sensor both `400`.

> **Response-shape note:** `/cur` on a sensor returns the **bare** point object,
> while `/write`, `/relinquish`, and `GET` return it wrapped as `{keyexpr, point}`.
> Hence `.cur_value` (bare) here vs `.point.cur_value` elsewhere.

---

## Acceptance criteria ("done")

- [x] Topology CRUD round-trips; keyexpr resolves `{org}/{site}/{equip}/{point}`.
- [x] Tag filter matches on marker presence; absent tag → empty.
- [x] Lower priority level number wins; relinquish restores the next level.
- [x] Priority outside `1..=16` is rejected.
- [x] Every effective-value change (write/relinquish/cur) lands in history.
- [x] A `sensor` point rejects priority-array writes; only `cmd`/`sp` accept them
      (and `/cur` is sensor-only — writable points reject it).

---

## Gotchas

- **The agent ceiling (`RUBIX_AI_MIN_PRIORITY`, default 13) does not apply here.**
  It gates the *agent's* `write_point` tool, not direct HTTP operators. A human can
  write any slot 1–16. See [AI_TOOLS_AND_AGENT.md](AI_TOOLS_AND_AGENT.md).
- Existing unit coverage: `rubix-core/src/priority.rs:78-126` already proves the
  array semantics in isolation — this runbook proves the HTTP wiring around it.

## Known issues / fixes

**2026-06-13 — verified live @ `d48dd8b2` (`RUBIX_ZENOH=0`).** Backend behaviour was
correct on every gate; the failures were all in the runbook's own commands (doc
bugs), now fixed above. No backend change was needed. The drift found:

1. **`CreatePoint` requires `display_name`.** The Step-1 point body omitted it →
   `422 missing field display_name`. Added it (sites/equips already had it).
2. **Tag filter param is `?tags=a,b` (single, comma-separated), not `?tag=a&tag=b`.**
   The repeated form is an unknown param, silently ignored, so *every* point came
   back — masquerading as "filter broken." With the correct param, `has_all`
   semantics hold exactly.
3. **Point create / GET / write / relinquish wrap the point as `{keyexpr, point:{…}}`;**
   only `keyexpr` is top-level. So the point **id** is `.point.id` (the original
   `jq -r .id` returned `null`, silently breaking every later step), and field jq
   paths must be `.point.cur_value` / `.point.priority_array`. The **list** endpoint
   and **`/cur`** (sensor) instead return the **bare** point object; site/equip
   create are bare too. An API shape inconsistency worth noting, not a bug here.
4. **A `sensor` point still serializes a 16-slot `priority_array`** (it's a struct
   field). Read-only-ness is enforced at the `/write` endpoint
   (`400 "is a sensor and cannot be commanded"`), not by the array being absent —
   the original gate's "a sensor does not have a priority_array" was wrong.
5. **`/cur` and `/write` are mutually exclusive by kind:** `/cur` on a writable
   point → `400 "use the write endpoint"`; `/write` on a sensor → `400`. Step 6 was
   doing `/cur` on the `sp` point; it now targets a sensor point.
