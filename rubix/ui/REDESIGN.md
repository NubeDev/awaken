# UI Redesign — decision & plan

Goal: make the app look and feel like [satnaing/shadcn-admin](https://github.com/satnaing/shadcn-admin)
— floating inset sidebar, macOS rounded edges, collapsible/hideable nav, light + dark
— without the duplicated per-screen chrome we have today.

## Decision: port the shell, do NOT rewrite

We considered starting a fresh UI (`frontend/`). **Rejected.** The existing app is
well-structured, not a hack — the "hack" feeling comes from ~300 LOC of layout chrome,
not the application.

Measured scope of what a rewrite would throw away (non-test code, ~8,000 LOC):

| Area          | LOC   | What it is                                                              |
|---------------|-------|------------------------------------------------------------------------|
| `api/`        | 618   | 12 files of backend integration (admin, agents, auth, boards, charts, collections, query, records, savedQueries, connection gate) |
| `components/` | 3,680 | SQL editor (CodeMirror + catalog + params + store), dashboards (grid, panels, presets, time ranges), chart-builder, viz lib (Bars/Donut/Line/Spark) |
| `pages/`      | 2,784 | 12 operator + admin screens                                            |
| `utils/`      | 334   | domain logic (`derive`, `schema` profiling) — **unit-tested**          |
| `hooks/`      | 187   | TanStack Query data layer                                              |

Clean `api → hooks → utils` separation, tested pure-function core, documented in
`FILE-LAYOUT.md`. The stack is **already identical** to shadcn-admin (React 19, Vite,
Tailwind v4, TanStack Router + Query, shadcn primitives), so a rewrite would reproduce
api/hooks/utils/viz/sql/dashboards near-verbatim — paying 8,000 lines to fix 300.

**The actual hack (what the redesign removes):**
- `components/admin/AdminLayout.tsx` threads `active="schema"` as a prop instead of
  deriving the active item from the route
- `components/ui/TopBar.tsx` is re-mounted per screen instead of one shared shell
- flat full-bleed nav, hardcoded body gradient, `@theme` color literals (theme.css)

## Status

- [x] **React 18 → 19** — `react`/`react-dom`/`@types` bumped to `^19.2.0` (installed
  19.2.7). `pnpm build` clean (0 type errors), `pnpm test` 21/21, no peer warnings.
  README stack line updated. `@vitejs/plugin-react` left at `^4` (plugin-react 6 needs
  Vite 6+; `^4` resolves to 4.3.x which supports React 19).
- [ ] Shell port (below)

## Approach: lift, don't fork

shadcn-admin is a *template with no backend*; our app is the product. We lift its layout
files onto our existing tokens/router, and skip its auth (Clerk), demo pages, and
`sidebar-data.ts`. Reference clone lives at `/tmp/shadcn-admin` (re-clone with
`git clone --depth 1 https://github.com/satnaing/shadcn-admin`).

Files to lift (adapt to our palette + `NAV`):

| From the reference repo                         | Into our app                                  |
|-------------------------------------------------|-----------------------------------------------|
| `components/ui/sidebar.tsx` (+ `dropdown-menu`, `sheet`, `tooltip`, `separator`) | `src/components/ui/` — the primitives (copy `sidebar.tsx` wholesale; it owns inset/icon/mobile-sheet logic) |
| `components/layout/authenticated-layout.tsx`    | `src/components/shell/AppShell.tsx` — `SidebarProvider + AppSidebar + SidebarInset` |
| `components/layout/app-sidebar.tsx`             | `src/components/shell/AppSidebar.tsx`         |
| `components/layout/nav-group.tsx`               | nav groups                                    |
| `components/layout/nav-user.tsx`                | footer avatar dropdown (replaces TopBar "AK") |
| `components/layout/team-switcher.tsx`           | tenant·site switcher (moves out of TopBar)    |
| `components/layout/header.tsx` + `main.tsx`     | `src/components/shell/AppHeader.tsx` — sticky header (trigger + breadcrumbs + ⌘K + theme) |
| `context/theme-provider.tsx`                    | light/dark provider                           |
| `context/layout-provider.tsx`                   | sidebar collapse pref persistence             |
| `layout/types.ts`                               | nav-item shape (replace `sidebar-data.ts` with our `NAV`) |

**New deps:** `@radix-ui/react-dropdown-menu`, `@radix-ui/react-tooltip`. (`sheet` reuses
`@radix-ui/react-dialog` we already have; `zustand` already present.)

## Work breakdown (small reviewable PRs)

1. **Primitives + theme** — vendor sidebar/dropdown-menu/sheet/tooltip/separator;
   add `ThemeProvider`; in `styles/theme.css` move the dark triples into `.dark {}`, add
   a `:root` light set, and switch the `@theme` block from `hsl(...)` literals to
   `hsl(var(--token))` so colors flip with the class. Add `--sidebar-*` tokens. No
   visual change yet. *Risk: the literal→var switch — operator pages use raw
   `bg-panel2`/`text-r1`; those vars stay, but verify in light mode.*
2. **Shell** — `AppShell`/`AppSidebar`/`AppHeader`; rewire the `t/$tenant` route to render
   `<AppShell><Outlet/></AppShell>` (router.tsx). One shell wraps operator + admin.
   Delete `AdminLayout.tsx` and the per-page `TopBar` mounts; active nav derives from the
   matched route.
3. **Sidebar visibility pref** — 3-state (expanded / icon rail / hidden) persisted via the
   ported `layout-provider`; control in the user menu. Hidden → full-bleed content, nav via
   ⌘K + trigger.
4. **Per-screen relayout** (one commit each) — header block → stacked full-width cards.
   `AdminSchema` first: drop its inner 260px kind-list column (it fights the sidebar =
   3 columns); kinds become top tabs, field table full-width, tags below. Then Records /
   Principals / Agents / Query / Dashboards.
5. **Light-mode polish + e2e** — CodeMirror light theme (`@uiw/codemirror-themes`),
   dashboard grid; update Playwright selectors (they assert on TopBar/nav), add specs for
   sidebar collapse + theme persistence.

## Open question: rename `ui/` → `frontend/`?

Optional and independent of the shell port. `git mv rubix/ui rubix/frontend` keeps 100%
of the code and history; cost is updating build/workspace paths and wherever the Rust side
serves the built `dist/`. Do it first (clean name + clean shell) or skip it — no effect on
the plan above.
