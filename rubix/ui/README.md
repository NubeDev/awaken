# Rubix UI

Frontend for the Rubix edge-to-cloud data platform.

**Stack:** React 19 + TypeScript + Vite

---

## Quick start

From the `rubix/` directory:

```bash
# Install dependencies
cd ui && pnpm install && cd ..

# Run backend + UI together
make dev

# Or run them separately
make dev-be      # backend only
make dev-ui      # UI only (requires backend on :8088)
```

**Ports:**
- Backend: `http://127.0.0.1:8088`
- UI dev server: `http://127.0.0.1:5180` (proxies `/api` to backend)

---

## Commands

```bash
make build        # build backend + UI for production
make build-ui     # build UI only (creates dist/)
make test         # run backend + UI tests
make test-ui      # run UI tests only
make lint         # run clippy (backend) + eslint (UI)
make fmt          # cargo fmt + prettier
make clean        # remove build artifacts
make kill         # free dev ports if stuck
```

---

## File layout

**Read [FILE-LAYOUT.md](FILE-LAYOUT.md) first.** It's the design guide for the repo.

One responsibility per file. Component-per-verb, not component-per-noun. ≤400 lines hard,
~80 lines typical.

Quick reference:

```
src/
  components/           ← reusable UI components (one verb per file)
  pages/                ← full-page views (route targets)
  hooks/                ← custom React hooks
  utils/                ← pure functions (format, validate, transform)
  types/                ← shared TypeScript types
  api/                  ← API client functions
  styles/               ← global CSS + theme
  App.tsx               ← app root + router
  main.tsx              ← entry point
```

---

## Development guidelines

### Components

- One responsibility per file (e.g., `List.tsx`, `ListFilters.tsx`, not
  `ListWithEverything.tsx`).
- Props object: keep ≤10 props per component.
- Extract sub-components early — if a component approaches 150 lines, split it.

### Hooks

- One hook per distinct piece of state/effect logic.
- Custom hooks go in `hooks/` with a `use` prefix: `useRecords.ts`,
  `useDebounce.ts`.

### Utilities

- Pure functions only — no state, no side effects, no React imports.
- Group by concept: `format.ts` (date/number/string formatting), `validate.ts`
  (form validation), `transform.ts` (data shape conversions).
- Never `utils.ts` or `helpers.ts`. Name the concept.

### Types

- One file per domain entity: `types/Record.ts`, `types/User.ts`.
- Share types via `types/index.ts` barrel: `export * from './Record'`.

### API client

- One file per API resource: `api/records.ts`, `api/tags.ts`.
- HTTP calls only — no state, no hooks.
- Use a shared `api/client.ts` for the HTTP client setup.

### Styles

- Global CSS in `styles/` (reset, theme variables, typography).
- Component-scoped styles: `.module.css` files or inline `<style>` blocks.
- Never `global.css` or `all.css`. Name the domain.

---

## Testing

Tests mirror the source structure:

```
src/components/records/List.tsx
test/components/records/List.test.tsx
```

Run tests with `make test-ui` or `pnpm test` from the `ui/` directory.

---

## Linting and formatting

```bash
# Lint only
pnpm lint

# Format code
pnpm format

# Or from the repo root
make lint
make fmt
```

---

## Build for production

```bash
pnpm build
# or
make build-ui
```

Output goes to `dist/`. Serve with any static HTTP server.

---

## Troubleshooting

**Port 5180 already in use?**
```bash
make kill         # free the ports
make dev-ui       # try again
```

**pnpm not installed?**
```bash
npm install -g pnpm
```

**Types/imports not resolving?**
```bash
cd ui && pnpm install && pnpm type-check
```

---

## Next steps

- **WS-16** (backend transport) wires the HTTP routes and API surface.
- UI will consume the `/api/*` endpoints and live-query WebSocket bridge.
- Start with `pages/` for route targets, then `components/` for reusable UI.

See [../docs/SCOPE.md](../docs/SCOPE.md) for the backend context and API contracts.
