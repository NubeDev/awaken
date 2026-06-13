# Rubix Console

Operator UI for the rubix BMS/EMS backend. Every surface reads and writes the
real `rubix-server` API — there is no demo/fixture data layer (the `check:fake`
gate enforces this).

## Dev loop

```sh
# 1. backend (port 8088, serves /api/v1/* and /api-docs/openapi.json)
cd rubix && cargo run -p rubix-server

# 2. UI (port 5180, proxies /api → 8088)
pnpm -C rubix/ui dev
```

If file watching misses changes under a low inotify limit, prefix the dev server
with `CHOKIDAR_USEPOLLING=true`.

The server's `/api-docs/openapi.json` is the wire source of truth; the TS client
types in `src/api/` are verified against it.

## Authentication

`rubix-server` accepts a bearer token on `/api/v1/*`. Until a deployment issuer
(OIDC) is configured, paste a raw API token on the sign-in screen:

- The token is stored client-side and attached as `Authorization: Bearer <token>`
  on every request.
- A `401` clears the token, toasts, and redirects back to sign-in.
- The signed-in principal is shown as a neutral "Operator" — no `whoami` endpoint
  is exposed on the wire, so no identity is invented.

## Dev seed

The demo building (sites, equips, points, sparks, live sim) is seeded into the
real store by the backend dev seed (UI-02), not by UI fixtures. Run the server
with the seed enabled so populated pages render with real rows.

## Scripts

| Script | Purpose |
| --- | --- |
| `pnpm dev` | Vite dev server (port 5180). |
| `pnpm build` | `tsc -b` typecheck + production `vite build`. |
| `pnpm lint` | ESLint (a permanent UI gate — keep it clean). |
| `pnpm test:unit` | Fast jsdom unit suite (`*.unit.test.*`), the canonical gate. |
| `pnpm test` | Browser component suite (needs `pnpm test:browser:install`). |
| `pnpm check:fake` | Fails if any `api/demo` / `VITE_DEMO` / `sample-board` reference returns. |
| `pnpm knip` | Reports unused files/exports/deps. |
| `node scripts/screenshot.mjs` | Capture reference screenshots for the look-freeze. |

## File discipline

See `/home/user/code/rust/starter/rubix/FILE-LAYOUT.md`: ≤400 lines per source
file (outside vendored `components/ui/` and the generated `routeTree.gen.ts`), one
verb/concept per file, no `utils.ts`-style grab bags.
