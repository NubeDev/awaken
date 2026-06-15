// CodeMirror SQL editor — the §1a "lift wholesale" editor. Now schema-aware:
// autocomplete over the Rubix catalog (canonical tables + structural columns +
// json_get/date_bin/aggregate functions), search, and selection-match
// highlighting, all from `editor-extensions.ts` (ported from Laminar's
// `components/sql/utils.ts`). A ⌘/Ctrl+Enter keybinding runs the query.
//
// Schema-aware completion is no longer deferred: it ships against a *static*
// catalog today. Per-`content.kind` typed columns will appear once the
// `/query/schema` endpoint lands (§1 prerequisite) — at which point the only
// change here is passing a fetched catalog instead of the default.

import { keymap } from '@codemirror/view'
import { Prec } from '@codemirror/state'
import CodeMirror from '@uiw/react-codemirror'
import { useMemo } from 'react'

import { createExtensions, theme } from './editor-extensions'
import type { RubixCatalog } from './catalog'

interface SqlEditorProps {
  value: string
  onChange: (value: string) => void
  onRun?: () => void
  minHeight?: string
  /** Override the autocomplete catalog (e.g. a fetched, row-perm-scoped schema). */
  catalog?: RubixCatalog
}

export function SqlEditor({ value, onChange, onRun, minHeight = '160px', catalog }: SqlEditorProps) {
  // High-precedence keymap so ⌘/Ctrl+Enter runs the query rather than inserting a
  // newline. `Prec.highest` beats CodeMirror's default Enter handling.
  const extensions = useMemo(
    () => [
      ...createExtensions(catalog),
      Prec.highest(
        keymap.of([
          {
            key: 'Mod-Enter',
            run: () => {
              onRun?.()
              return true
            },
          },
        ]),
      ),
    ],
    [onRun, catalog],
  )

  return (
    <div className="overflow-hidden rounded-md border border-border bg-card/40">
      <CodeMirror
        value={value}
        onChange={onChange}
        theme={theme}
        extensions={extensions}
        minHeight={minHeight}
        basicSetup={{
          lineNumbers: true,
          foldGutter: false,
          highlightActiveLine: true,
          autocompletion: true,
        }}
      />
    </div>
  )
}
