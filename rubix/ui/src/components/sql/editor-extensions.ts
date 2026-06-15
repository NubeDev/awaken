// Schema-aware CodeMirror extensions for the SQL console — ported from Laminar's
// `components/sql/utils.ts` (§1a, LAMINAR-BORROW.md) and adapted to Rubix.
//
// What carried over: the autocomplete mechanism (`schemaCompletionSource` +
// `keywordCompletionSource` fed by a table/column schema), `search`,
// selection-match highlighting, line wrapping, and the themed completion
// dropdown. What did NOT carry over: Laminar's ClickHouse dialect, signature
// help, and enum-value-in-string completion all live in their `lang-clickhouse`
// module and assume real, typed columns — Rubix's surface exposes canonical
// tables with a `content` JSON *string* column (schema.rs), so those stay out
// until a `/query/schema` catalog endpoint lands (§1 prerequisite). This is the
// "swap the schema" move: same engine, Rubix catalog instead of ClickHouse's.

import {
  autocompletion,
  completionKeymap,
  type Completion,
} from '@codemirror/autocomplete'
import {
  StandardSQL,
  keywordCompletionSource,
  schemaCompletionSource,
  sql,
  type SQLNamespace,
} from '@codemirror/lang-sql'
import { highlightSelectionMatches, search } from '@codemirror/search'
import { Prec } from '@codemirror/state'
import { EditorView, keymap } from '@codemirror/view'
import { tags as t } from '@lezer/highlight'
import { createTheme } from '@uiw/codemirror-themes'

import {
  type RubixCatalog,
  defaultCatalog,
  functionCompletions,
} from './catalog'

// Syntax palette — Rubix's existing dark identity (mirrors styles/theme.css),
// kept rather than Laminar's VSCode hex so the editor stays visually native.
export const theme = createTheme({
  theme: 'dark',
  settings: {
    background: 'transparent',
    foreground: 'hsl(220, 16%, 93%)',
    caret: 'hsl(258, 84%, 74%)',
    selection: 'hsl(258, 84%, 74%, 0.25)',
    selectionMatch: 'hsl(258, 84%, 74%, 0.20)',
    lineHighlight: 'hsl(230, 14%, 12%, 0.4)',
    gutterBackground: 'transparent',
    gutterForeground: 'hsl(220, 8%, 56%)',
    fontFamily: 'var(--font-mono)',
  },
  styles: [
    { tag: t.keyword, color: 'hsl(258, 84%, 74%)' },
    { tag: t.string, color: 'hsl(150, 64%, 52%)' },
    { tag: [t.number, t.bool, t.null], color: 'hsl(32, 92%, 60%)' },
    { tag: t.function(t.variableName), color: 'hsl(200, 86%, 64%)' },
    { tag: t.comment, color: 'hsl(220, 8%, 56%)', fontStyle: 'italic' },
    { tag: t.operator, color: 'hsl(174, 70%, 56%)' },
  ],
})

// Themed autocomplete dropdown + search-match styling — ported from Laminar's
// `autocompleteStyles`/`editorBaseStyles`, using the shadcn CSS vars Rubix's
// Tailwind theme already defines (--background/--border/--accent/--primary).
const editorTheme = EditorView.theme({
  '&.cm-focused': { outline: 'none !important' },
  '&': { fontSize: '0.8125rem !important' },
  '.cm-searchMatch': {
    backgroundColor: 'hsl(var(--primary) / 0.3)',
    border: '1px solid hsl(var(--primary))',
    borderRadius: '3px',
  },
  '.cm-searchMatch-selected': {
    backgroundColor: 'hsl(var(--primary))',
    color: 'hsl(var(--primary-foreground))',
  },
  '.cm-tooltip.cm-tooltip-autocomplete': {
    background: 'hsl(var(--background))',
    border: '1px solid hsl(var(--border))',
    borderRadius: '6px',
    boxShadow: '0 4px 12px rgba(0, 0, 0, 0.25)',
  },
  '.cm-tooltip-autocomplete ul': {
    fontFamily: 'var(--font-mono)',
    fontSize: '13px',
  },
  '.cm-tooltip-autocomplete ul li': {
    padding: '2px 6px !important',
    display: 'flex',
    alignItems: 'center',
    gap: '6px',
  },
  '.cm-tooltip-autocomplete ul li[aria-selected]': {
    background: 'hsl(var(--accent))',
    color: 'hsl(var(--accent-foreground))',
  },
  '.cm-completionIcon': {
    width: '14px',
    padding: '0 !important',
    marginRight: '2px',
    opacity: '1',
  },
  '.cm-completionIcon-property::after': { content: "'◇'", color: '#9CDCFE' },
  '.cm-completionIcon-keyword::after': { content: "'⊞'", color: '#C586C0' },
  '.cm-completionIcon-function::after': { content: "'ƒ'", color: '#DCDCAA' },
  '.cm-completionIcon-type::after': { content: "'T'", color: '#4EC9B0' },
  '.cm-completionDetail': {
    color: 'hsl(var(--muted-foreground))',
    fontStyle: 'normal',
    marginLeft: 'auto',
    fontSize: '11px',
  },
  '.cm-completionMatchedText': {
    color: 'hsl(var(--primary))',
    fontWeight: '600',
    textDecoration: 'none',
  },
  '.cm-tooltip.cm-completionInfo': {
    background: 'hsl(var(--background))',
    border: '1px solid hsl(var(--border))',
    borderRadius: '6px',
    padding: '6px 10px',
    maxWidth: '360px',
    fontSize: '12px',
    color: 'hsl(var(--muted-foreground))',
    whiteSpace: 'pre-wrap',
  },
})

// Turn the Rubix catalog into the `SQLNamespace` shape the lang-sql schema
// completion source consumes — table → its columns (ported from Laminar's
// `createScopedCompletionSource`, minus the per-enum breakdown).
function toSqlNamespace(catalog: RubixCatalog): SQLNamespace {
  return Object.fromEntries(
    catalog.tables.map((table) => [
      table.name,
      table.columns.map(
        (col): Completion => ({
          label: col.name,
          type: 'property',
          detail: col.type,
          info: col.description,
        }),
      ),
    ]),
  )
}

// CodeMirror extension bundle for the SQL editor, scoped by a catalog
// (defaults to the canonical-table catalog). Ported from Laminar's
// `createExtensions`, with the ClickHouse dialect/signature-help dropped.
export function createExtensions(catalog: RubixCatalog = defaultCatalog) {
  const schemaSource = schemaCompletionSource({
    schema: toSqlNamespace(catalog),
    upperCaseKeywords: true,
  })

  return [
    editorTheme,
    search(),
    highlightSelectionMatches(),
    EditorView.lineWrapping,
    sql({ dialect: StandardSQL, upperCaseKeywords: true }),
    autocompletion({
      override: [
        schemaSource,
        keywordCompletionSource(StandardSQL, true),
        // Rubix UDFs + DataFusion functions the canonical surface understands —
        // json_get, date_trunc/date_bin, aggregates, approx_percentile_cont.
        functionCompletions(catalog),
      ],
    }),
    Prec.highest(keymap.of(completionKeymap)),
  ]
}
