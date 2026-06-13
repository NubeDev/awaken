import { useEffect, useRef } from 'react'
import {
  autocompletion,
  closeBrackets,
  closeBracketsKeymap,
  completionKeymap,
  type CompletionContext,
  type CompletionResult,
} from '@codemirror/autocomplete'
import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands'
import {
  HighlightStyle,
  StreamLanguage,
  syntaxHighlighting,
} from '@codemirror/language'
import { sql } from '@codemirror/lang-sql'
import { Compartment, EditorState, type Extension } from '@codemirror/state'
import {
  EditorView,
  highlightActiveLine,
  highlightActiveLineGutter,
  keymap,
  lineNumbers,
  placeholder as placeholderExt,
} from '@codemirror/view'
import { tags as t } from '@lezer/highlight'
import { rhaiLanguage, RUBIX_PRIMITIVES } from './rhai-language'

/**
 * Headless CodeMirror 6 editor themed to the app's dark tokens. Two languages:
 * `rhai` for rule scripts (with rubix-primitive highlighting + autocomplete) and
 * `sql` for the query workbench. Mirrors the existing surfaces' typography (a
 * mono 12.5px) so it reads native, not bolted on.
 */
export type CodeLanguage = 'rhai' | 'sql'

type CodeEditorProps = {
  value: string
  onChange: (value: string) => void
  language: CodeLanguage
  /** Compile/runtime error message to surface as an inline banner under the editor. */
  error?: string
  placeholder?: string
  /** Extra params (e.g. `params.*`) to offer in Rhai autocomplete. */
  paramNames?: string[]
  ariaLabel?: string
  minHeight?: number
}

// App-token highlight style: maps lezer tags to the same CSS vars the rest of
// the UI uses, so the editor inherits the theme rather than hardcoding colors.
const highlight = HighlightStyle.define([
  { tag: t.keyword, color: 'var(--chart-1)' },
  { tag: [t.string, t.special(t.string)], color: 'var(--sev-warning)' },
  { tag: [t.number, t.bool], color: 'var(--sev-info)' },
  { tag: t.comment, color: 'var(--muted-foreground)', fontStyle: 'italic' },
  { tag: [t.function(t.variableName), t.function(t.propertyName)], color: 'var(--positive)' },
  { tag: t.operator, color: 'var(--muted-foreground)' },
  { tag: t.propertyName, color: 'var(--foreground)' },
])

const theme = EditorView.theme(
  {
    '&': {
      backgroundColor: 'transparent',
      color: 'var(--foreground)',
      fontSize: '12.5px',
    },
    '.cm-content': {
      fontFamily: 'var(--font-mono, ui-monospace, monospace)',
      caretColor: 'var(--foreground)',
      padding: '8px 0',
    },
    '.cm-gutters': {
      backgroundColor: 'transparent',
      color: 'var(--muted-foreground)',
      border: 'none',
      fontSize: '11px',
    },
    '.cm-activeLine': { backgroundColor: 'color-mix(in oklab, var(--muted) 35%, transparent)' },
    '.cm-activeLineGutter': { backgroundColor: 'transparent' },
    '.cm-cursor': { borderLeftColor: 'var(--foreground)' },
    '.cm-selectionBackground, &.cm-focused .cm-selectionBackground, ::selection': {
      backgroundColor: 'color-mix(in oklab, var(--chart-1) 28%, transparent)',
    },
    '.cm-tooltip': {
      backgroundColor: 'var(--popover)',
      border: '1px solid var(--border)',
      borderRadius: '8px',
      color: 'var(--popover-foreground)',
      fontSize: '12px',
    },
    '.cm-tooltip-autocomplete ul li[aria-selected]': {
      backgroundColor: 'color-mix(in oklab, var(--chart-1) 22%, transparent)',
      color: 'var(--foreground)',
    },
    '&.cm-focused': { outline: 'none' },
    '.cm-placeholder': { color: 'var(--muted-foreground)' },
  },
  { dark: true }
)

/** Autocomplete the curated rubix primitives + declared `params.*` for Rhai. */
function rhaiCompletions(paramNames: string[]) {
  return (ctx: CompletionContext): CompletionResult | null => {
    const word = ctx.matchBefore(/[\w.]*/)
    if (!word || (word.from === word.to && !ctx.explicit)) return null
    const primitives = RUBIX_PRIMITIVES.map((p) => ({
      label: p.label,
      type: p.type,
      info: p.info,
      apply: p.apply ?? p.label,
    }))
    const params = paramNames.map((name) => ({
      label: `params.${name}`,
      type: 'variable',
      info: 'Declared rule parameter',
    }))
    return { from: word.from, options: [...primitives, ...params], validFor: /[\w.]*/ }
  }
}

function languageExtension(language: CodeLanguage, paramNames: string[]): Extension {
  if (language === 'sql') return sql()
  return [
    StreamLanguage.define(rhaiLanguage),
    autocompletion({ override: [rhaiCompletions(paramNames)] }),
  ]
}

export function CodeEditor({
  value,
  onChange,
  language,
  error,
  placeholder,
  paramNames = [],
  ariaLabel,
  minHeight = 200,
}: CodeEditorProps) {
  const host = useRef<HTMLDivElement>(null)
  const view = useRef<EditorView | null>(null)
  const onChangeRef = useRef(onChange)
  const langCompartment = useRef(new Compartment())

  // Keep the latest onChange reachable from the (stable) updateListener without
  // rebuilding the editor; updating the ref in an effect avoids a render-time write.
  useEffect(() => {
    onChangeRef.current = onChange
  }, [onChange])

  // Build the view once; reconfigure language and sync value via effects so the
  // editor is not torn down on every keystroke (which would lose cursor state).
  useEffect(() => {
    if (!host.current) return
    const state = EditorState.create({
      doc: value,
      extensions: [
        lineNumbers(),
        highlightActiveLine(),
        highlightActiveLineGutter(),
        history(),
        closeBrackets(),
        syntaxHighlighting(highlight),
        keymap.of([
          ...closeBracketsKeymap,
          ...defaultKeymap,
          ...historyKeymap,
          ...completionKeymap,
          indentWithTab,
        ]),
        EditorView.lineWrapping,
        theme,
        placeholder ? placeholderExt(placeholder) : [],
        langCompartment.current.of(languageExtension(language, paramNames)),
        EditorView.updateListener.of((u) => {
          if (u.docChanged) onChangeRef.current(u.state.doc.toString())
        }),
        EditorView.contentAttributes.of(
          ariaLabel ? { 'aria-label': ariaLabel } : {}
        ),
      ],
    })
    const v = new EditorView({ state, parent: host.current })
    view.current = v
    return () => {
      v.destroy()
      view.current = null
    }
    // The view is created once; language/value/params are pushed via the effects
    // below. Recreating on every prop change would reset cursor + scroll.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  // Reconfigure the language (and its completions) when it or params change.
  useEffect(() => {
    view.current?.dispatch({
      effects: langCompartment.current.reconfigure(
        languageExtension(language, paramNames)
      ),
    })
    // paramNames is a fresh array each render; compare by content to avoid churn.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [language, paramNames.join(',')])

  // Sync external value changes (e.g. selecting a different rule) into the doc.
  useEffect(() => {
    const v = view.current
    if (!v) return
    const current = v.state.doc.toString()
    if (current !== value) {
      v.dispatch({ changes: { from: 0, to: current.length, insert: value } })
    }
  }, [value])

  return (
    <div className='flex flex-col'>
      <div
        ref={host}
        style={{ minHeight }}
        className={`scroll overflow-auto rounded-md border bg-card/40 ${
          error ? 'border-destructive' : 'border-border'
        }`}
      />
      {error && (
        <p role='alert' className='text-destructive mt-1.5 font-mono text-[11px]'>
          {error}
        </p>
      )}
    </div>
  )
}
