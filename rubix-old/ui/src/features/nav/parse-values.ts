/**
 * Parse the navigation builder's `key=value` context-override lines into a
 * context-values map (docs/design/page-context-and-nav.md §4,§7). Kept in its
 * own module so the value-binding rule lives in one place: a value stays a
 * string and binds as a SQL parameter downstream, never concatenated.
 */

/** Parse `key=value` lines into a context-values map. Blank lines and lines
 *  without `=` are ignored; a value keeps everything after the first `=` so a
 *  value may itself contain `=`. The value stays a string — it binds as a
 *  parameter downstream, never concatenated. */
export function parseValues(text: string): Record<string, string> {
  const out: Record<string, string> = {}
  for (const line of text.split('\n')) {
    const eq = line.indexOf('=')
    if (eq <= 0) continue
    const key = line.slice(0, eq).trim()
    if (key === '') continue
    out[key] = line.slice(eq + 1).trim()
  }
  return out
}
