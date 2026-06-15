// CodeMirror SQL editor — the §1a "lift wholesale" editor, trimmed to what works
// on today's surface: SQL syntax highlighting, a Rubix-themed dark palette, and a
// ⌘/Ctrl+Enter run keybinding. Schema-aware autocomplete is deferred — it tracks
// the query-catalog endpoint (§1 prereq), which is not on the committed spine.

import { sql } from '@codemirror/lang-sql'
import { keymap } from '@codemirror/view'
import { Prec } from '@codemirror/state'
import CodeMirror from '@uiw/react-codemirror'
import { createTheme } from '@uiw/codemirror-themes'
import { tags as t } from '@lezer/highlight'
import { useMemo } from 'react'

// Mirror the app's dark theme (styles/theme.css) so the editor sits in the
// surrounding panels rather than shipping CodeMirror's default light look.
const rubixTheme = createTheme({
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

interface SqlEditorProps {
  value: string
  onChange: (value: string) => void
  onRun?: () => void
  minHeight?: string
}

export function SqlEditor({ value, onChange, onRun, minHeight = '160px' }: SqlEditorProps) {
  // High-precedence keymap so ⌘/Ctrl+Enter runs the query rather than inserting a
  // newline. `Prec.highest` beats CodeMirror's default Enter handling.
  const extensions = useMemo(
    () => [
      sql(),
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
    [onRun],
  )

  return (
    <div className="overflow-hidden rounded-md border border-border bg-card/40">
      <CodeMirror
        value={value}
        onChange={onChange}
        theme={rubixTheme}
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
