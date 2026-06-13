import type { StreamParser } from '@codemirror/language'

/**
 * A lightweight Rhai stream parser for CodeMirror. Rhai has no official CM6
 * grammar; a stream tokenizer is enough for the rule editor — it colours
 * keywords, strings, numbers, comments, and the curated rubix primitives as
 * function calls. It is intentionally shallow (no full AST): rule scripts are
 * short, and compile errors come from the server's real engine via dry-run.
 */

const KEYWORDS = new Set([
  'let',
  'const',
  'if',
  'else',
  'switch',
  'while',
  'loop',
  'for',
  'in',
  'do',
  'until',
  'continue',
  'break',
  'return',
  'fn',
  'private',
  'throw',
  'try',
  'catch',
  'import',
  'export',
  'as',
  'global',
])

const ATOMS = new Set(['true', 'false', '()'])

// The curated rubix primitives the editor highlights as known functions and
// offers in autocomplete. Kept in sync with the rules-engine design doc.
const PRIMITIVES = new Set([
  'select',
  'rename',
  'filter_gt',
  'filter_lt',
  'filter_eq',
  'rolling_mean',
  'rolling_min',
  'rolling_max',
  'rolling_sum',
  'zscore',
  'resample',
  'lag',
  'diff',
  'pct_change',
  'fill_null',
  'head',
  'tail',
  'sort',
  'anomalies',
  'describe',
  'any_true',
  'rule',
  'finding',
  'with_value',
  'clear',
])

export const rhaiLanguage: StreamParser<{ inBlockComment: boolean }> = {
  name: 'rhai',
  startState: () => ({ inBlockComment: false }),
  token(stream, state) {
    if (state.inBlockComment) {
      if (stream.match(/.*?\*\//)) state.inBlockComment = false
      else stream.skipToEnd()
      return 'comment'
    }
    if (stream.eatSpace()) return null

    // Comments.
    if (stream.match('//')) {
      stream.skipToEnd()
      return 'comment'
    }
    if (stream.match('/*')) {
      state.inBlockComment = true
      return 'comment'
    }

    // Strings (double and single quoted), with escapes.
    if (stream.match(/"(?:[^"\\]|\\.)*"/) || stream.match(/'(?:[^'\\]|\\.)*'/)) {
      return 'string'
    }

    // Numbers (int, float, hex).
    if (stream.match(/0x[\da-fA-F_]+/) || stream.match(/\d[\d_]*(?:\.\d[\d_]*)?(?:[eE][+-]?\d+)?/)) {
      return 'number'
    }

    // Identifiers / keywords / primitive calls.
    if (stream.match(/[A-Za-z_$][\w$]*/)) {
      const word = stream.current()
      if (KEYWORDS.has(word)) return 'keyword'
      if (ATOMS.has(word)) return 'bool'
      // A name directly followed by `(` is a call; primitives get the function
      // tag, everything else is a generic property/variable.
      const isCall = stream.peek() === '('
      if (PRIMITIVES.has(word)) return 'function'
      if (isCall) return 'function'
      return 'variableName'
    }

    // Operators / punctuation.
    if (stream.match(/[+\-*/%=!<>&|^~?:.]+/)) return 'operator'

    stream.next()
    return null
  },
  languageData: {
    commentTokens: { line: '//', block: { open: '/*', close: '*/' } },
    closeBrackets: { brackets: ['(', '[', '{', '"', "'"] },
  },
}

/** A curated primitive for the autocomplete list: label, type, and signature info. */
export type Primitive = {
  label: string
  type: string
  info: string
  /** Text inserted on accept (defaults to `label`). */
  apply?: string
}

// Signatures shown in the completion popup. The `apply` text inserts a call
// skeleton so an operator gets the argument shape, not just the bare name.
export const RUBIX_PRIMITIVES: Primitive[] = [
  { label: 'select', type: 'method', info: 'select(cols) — keep only these columns', apply: 'select(' },
  { label: 'rename', type: 'method', info: 'rename(from, to) — rename a column', apply: 'rename(' },
  { label: 'filter_gt', type: 'method', info: 'filter_gt(col, n) — rows where col > n', apply: 'filter_gt(' },
  { label: 'filter_lt', type: 'method', info: 'filter_lt(col, n) — rows where col < n', apply: 'filter_lt(' },
  { label: 'filter_eq', type: 'method', info: 'filter_eq(col, v) — rows where col == v', apply: 'filter_eq(' },
  { label: 'rolling_mean', type: 'method', info: 'rolling_mean(time_col, window) — moving average', apply: 'rolling_mean(' },
  { label: 'rolling_min', type: 'method', info: 'rolling_min(time_col, window)', apply: 'rolling_min(' },
  { label: 'rolling_max', type: 'method', info: 'rolling_max(time_col, window)', apply: 'rolling_max(' },
  { label: 'rolling_sum', type: 'method', info: 'rolling_sum(time_col, window)', apply: 'rolling_sum(' },
  { label: 'zscore', type: 'method', info: 'zscore(col) — standard score of a column', apply: 'zscore(' },
  { label: 'resample', type: 'method', info: 'resample(time_col, every, aggs) — downsample', apply: 'resample(' },
  { label: 'lag', type: 'method', info: 'lag(col) — previous-row value', apply: 'lag(' },
  { label: 'diff', type: 'method', info: 'diff(col) — first difference', apply: 'diff(' },
  { label: 'pct_change', type: 'method', info: 'pct_change(col) — percent change', apply: 'pct_change(' },
  { label: 'fill_null', type: 'method', info: "fill_null(strategy) — e.g. 'forward'", apply: 'fill_null(' },
  { label: 'head', type: 'method', info: 'head(n) — first n rows', apply: 'head(' },
  { label: 'tail', type: 'method', info: 'tail(n) — last n rows', apply: 'tail(' },
  { label: 'sort', type: 'method', info: 'sort(col) — sort rows by a column', apply: 'sort(' },
  { label: 'anomalies', type: 'method', info: 'anomalies(col, z) — rows beyond z std devs', apply: 'anomalies(' },
  { label: 'describe', type: 'method', info: 'describe() — summary stats frame', apply: 'describe()' },
  { label: 'any_true', type: 'method', info: 'any_true(col) — true if any row in col is true', apply: 'any_true(' },
  { label: 'rule', type: 'function', info: 'rule(name, df, #{...}) — compose another rule', apply: 'rule(' },
  { label: 'finding', type: 'function', info: 'finding("info"|"warning"|"fault", msg) — a verdict', apply: 'finding(' },
  { label: 'with_value', type: 'method', info: 'with_value(n) — attach a score to a finding', apply: 'with_value(' },
  { label: 'clear', type: 'function', info: 'clear() — the no-finding verdict', apply: 'clear()' },
  { label: 'params', type: 'variable', info: 'The rule parameter map' },
  { label: 'df', type: 'variable', info: 'The input frame (ts + value columns)' },
]
