/**
 * Extract the variable names a piece of SQL references. Mirrors the server-side
 * interpolation engine's token grammar (docs/design/variables-and-templating.md
 * §2): `$name`, `${name}`, `${name:format}`, and `$__sqlIn(name)`. Used to build
 * the cascade dependency graph (a `query` variable's SQL may reference another
 * variable) and to decide which widgets re-fetch on a selection change.
 *
 * This never *binds* anything — binding is the server's job; this only reads the
 * names out of the text so the frontend knows the dependencies.
 */

/** A bare/positional `$1` placeholder is not a variable; a name starts with a
 *  letter or underscore. Built-in tokens (`$__org`, `$__sqlIn`) start with `$__`
 *  and are handled by the dedicated patterns below. */
const SQL_IN = /\$__sqlIn\(\s*([A-Za-z_][A-Za-z0-9_]*)\s*\)/g
const BRACE = /\$\{\s*([A-Za-z_][A-Za-z0-9_]*)\s*(?::[^}]*)?\}/g
const BARE = /\$([A-Za-z_][A-Za-z0-9_]*)/g

/**
 * The set of variable names referenced in `sql`, in first-appearance order.
 * `$__sqlIn(name)` and `${name}` forms are detected first so their inner names
 * are captured; the bare `$name` scan then picks up any remaining references
 * (the `$__sqlIn`/`${...}` text is masked so its `$` is not double-counted).
 */
export function referencedVariables(sql: string): string[] {
  const names = new Set<string>()

  // Mask the structured tokens as we read them, so the bare-`$name` pass does
  // not re-match the `$` that opens `${...}` or `$__sqlIn(...)`.
  let masked = sql
  for (const re of [SQL_IN, BRACE]) {
    re.lastIndex = 0
    masked = masked.replace(re, (whole, name: string) => {
      names.add(name)
      return ' '.repeat(whole.length)
    })
  }

  BARE.lastIndex = 0
  let m: RegExpExecArray | null
  while ((m = BARE.exec(masked)) !== null) {
    // A `$__`-prefixed token that survived masking is a built-in (e.g. `$__org`,
    // `$__from`); those are resolved from context, not authored variables, so
    // they are still returned — a caller filters built-ins if it wants only the
    // dashboard's own variables.
    names.add(m[1])
  }

  return [...names]
}
