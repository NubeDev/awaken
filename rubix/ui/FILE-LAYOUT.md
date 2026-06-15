# FILE LAYOUT — React UI organization

The UI follows the same **one-responsibility-per-file** principle as the backend.
Small, well-named files are how an AI (or a human reading cold) finds the right code
without burning context on irrelevant lines.

**Adapted from the backend [FILE-LAYOUT.md](../docs/FILE-LAYOUT.md).**

---

## 1. Hard limits

| Limit | Value | Hard? |
|---|---|---|
| Lines per file | **400** | Hard. PR blocked above this. |
| Lines per file (warning) | 300 | Soft. Plan the split. |
| Lines per component | 150 | Soft. Extract a sub-component. |
| Props per component | ~10 | Soft. Use an object, consider splitting. |
| Nesting depth | 4 | Soft. Extract early. |

400 lines is the **ceiling**, not the target. Most files in this repo should sit
between 20 and 150 lines.

---

## 2. Folder structure

```
ui/src/
  components/           ← reusable UI components
  pages/                ← full-page views (route targets)
  hooks/                ← custom React hooks
  utils/                ← pure functions (format, validate, transform)
  types/                ← shared TypeScript types + interfaces
  api/                  ← API client functions
  styles/               ← global CSS + theme
  App.tsx               ← top-level app + router
  main.tsx              ← entry point
  index.css             ← global styles
```

---

## 3. Components — verb-per-file

Group components by **what they do**, not by what data they render.

### Example — record list

**Wrong** — one file per noun:

```
components/
  RecordList.tsx        ← 500 lines: display + filter + sort + search + export
```

**Correct** — one file per verb/responsibility:

```
components/records/
  index.ts              ← exports only
  List.tsx              ← display list rows
  ListFilters.tsx       ← filter UI + state
  ListSort.tsx          ← sort controls
  ListSearch.tsx        ← search input
  ListExport.tsx        ← export button + handler
  useRecordList.ts      ← fetch + filter logic (custom hook)
```

Each component: **one job, ≤150 lines, ≤10 props**.

**Bad component signature:**
```tsx
<RecordList records={r} onFilter={f} onSort={s} onSearch={q} onExport={e} ... />
```

**Good component signatures:**
```tsx
<RecordList records={records} onRowClick={handleSelect} />
<ListFilters filters={filters} onChange={setFilters} />
<ListSort sort={sort} onChange={setSort} />
```

### Index files are barrels only

```typescript
// components/records/index.ts
export { List as RecordList } from './List'
export { ListFilters } from './ListFilters'
export { useRecordList } from './useRecordList'
```

Never put component logic inside `index.ts`.

---

## 4. Pages — one route per file

One page component per route/URL.

```
pages/
  Records.tsx           ← /records
  RecordDetail.tsx      ← /records/:id
  Settings.tsx          ← /settings
  NotFound.tsx          ← 404 fallback
```

A page may **compose** multiple components, but stays within 200 lines by
delegating to sub-components:

```tsx
// pages/Records.tsx (80 lines)
export default function RecordsPage() {
  const [filters, setFilters] = useState(...)
  const { records } = useRecords(filters)
  
  return (
    <div>
      <h1>Records</h1>
      <RecordListFilters {...} />
      <RecordList records={records} />
    </div>
  )
}
```

---

## 5. Hooks — custom React logic

One hook per distinct piece of state/effect logic.

```
hooks/
  useRecords.ts         ← fetch records, manage cache
  useDebounce.ts        ← debounce any value
  useLocalStorage.ts    ← sync state to localStorage
  usePagination.ts      ← offset/limit pagination
  useAsync.ts           ← generic async data fetching
```

Hook files are **small and focused**. If a hook approaches 100 lines, split it
by phase (setup, fetch, transform, cache).

```typescript
// hooks/useRecords.ts (60 lines)
export function useRecords(filters: Filters) {
  const [records, setRecords] = useState([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<Error | null>(null)

  useEffect(() => {
    fetchRecords(filters).then(...).catch(...)
  }, [filters])

  return { records, loading, error }
}
```

---

## 6. Utilities — pure functions

Functions that have no state, no side effects, no React dependency.

```
utils/
  format.ts             ← date, number, string formatting
  validate.ts           ← form validation functions
  transform.ts          ← data shape conversions
  retry.ts              ← retry logic (generic, reusable)
  http.ts               ← request/response helpers
```

Never `utils.ts` / `helpers.ts` / `common.ts`. Name the concept.

Each file: **one family of related functions, ≤150 lines**.

```typescript
// utils/format.ts — only date/number formatting, nothing else
export function formatDate(d: Date): string { ... }
export function formatCurrency(n: number): string { ... }
export function formatPercent(n: number): string { ... }
```

---

## 7. Types — shared interfaces

One file per domain / entity type.

```
types/
  index.ts              ← re-exports all types
  Record.ts             ← Record, RecordDTO, RecordInput
  User.ts               ← User, UserDTO
  API.ts                ← API response/error shapes
  Filters.ts            ← filter/query types shared across pages
```

**No generic `types.ts` or `index.ts` with 50 type definitions.** Split by domain.

```typescript
// types/Record.ts
export interface Record {
  id: string
  name: string
  tags: string[]
  createdAt: Date
}

export interface RecordInput {
  name: string
  tags?: string[]
}
```

---

## 8. API — server communication

One file per API resource / domain.

```
api/
  client.ts             ← HTTP client setup (axios, fetch wrapper)
  records.ts            ← GET/POST/PATCH /records
  tags.ts               ← GET/POST /tags
  users.ts              ← GET /user (current user)
  errors.ts             ← error handling + retry logic
```

Each file: **HTTP calls only, no state, no React.**

```typescript
// api/records.ts
import { apiClient } from './client'
import type { Record, RecordInput } from '../types'

export async function getRecords(filters?: Filters): Promise<Record[]> {
  const { data } = await apiClient.get('/records', { params: filters })
  return data
}

export async function createRecord(input: RecordInput): Promise<Record> {
  const { data } = await apiClient.post('/records', input)
  return data
}
```

---

## 9. Styles — CSS organization

```
styles/
  index.css             ← global reset + theme variables
  layout.css            ← grid, flexbox utilities
  typography.css        ← font families + sizes
  colors.css            ← color palette (via CSS variables)
```

**Component-scoped styles**: keep them in the component file as `<style scoped>` or
a `.module.css` file if reuse is needed.

```
components/records/
  List.tsx
  List.module.css       ← styles for List.tsx only
```

Never `global-styles.css` or `all.css`. Name the domain.

---

## 10. File naming

| Never | Always |
|---|---|
| `utils.ts` / `helpers.ts` / `common.ts` | Name the concept: `retry.ts`, `format.ts`, `validate.ts` |
| `index.tsx` with logic in it | `index.ts` is a barrel (exports only). Logic lives elsewhere. |
| `RecordComponent.tsx` | Name the responsibility: `RecordList.tsx`, `RecordForm.tsx` |
| `useCustomHook.ts` for every hook | Name the hook's job: `useRecords.ts`, `useDebounce.ts` |
| Generic types in `types/types.ts` | Domain-specific: `types/Record.ts`, `types/Filters.ts` |

If you cannot describe the file's job in one sentence without "and" — it's two files.

---

## 11. The split heuristic

When you sit down to write a file or open one to edit, ask in order:

1. **One-sentence test.** Can I describe this file's job in one short sentence
   with no "and"? If no → it's two or more files.
2. **Blast-radius test.** If this file changes, what else might break? If the
   answer mentions unrelated features → it's mixed, split it.
3. **Filename test.** Would someone searching by filename find what they expect?
   `RecordList.tsx` → yes. `RecordUI.tsx` → no.
4. **Edit-locality test.** If two PRs both touch this file, do they touch the
   same lines or different concerns? If different concerns → split.

If you're about to write more than **~100 lines** in a new component, pause and
extract sub-components first.

---

## 12. When NOT to split

- A `<label>` wrapper component with inline styles is fine in the same file as
  the form it's used in.
- A small component + its styling module belong together.
- A page and the single custom hook it uses, where no other page calls it, may
  live as `Page.tsx` + `usePage.ts` in the same folder.
- Type definitions for a single component live in the same file.

**Rule of thumb:** split when there are **two distinct caller-visible
responsibilities**. Two private helper functions that always run together in
one component's render are not two responsibilities.

---

## 13. One-line summary

**One responsibility per file. Component-per-verb, not component-per-noun.
≤400 lines hard, ~80 lines typical. Names are concepts, never shapes
(`utils`, `helpers`, `common`).**
