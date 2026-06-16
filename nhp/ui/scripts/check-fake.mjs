/**
 * Fail the build if the UI re-introduces the deleted demo layer. Mirrors the
 * grep gate the loop driver runs verbatim (see rubix/docs/sessions/ui/STATUS.md
 * DONE GATE): any `api/demo`, `VITE_DEMO`, or `sample-board` reference in shipped
 * `src` code (test files excepted) is fake data and must not land.
 */
import { readdirSync, readFileSync, statSync } from 'node:fs'
import { join } from 'node:path'

const ROOT = new URL('../src', import.meta.url).pathname
const FORBIDDEN = [/api\/demo/, /VITE_DEMO/, /sample-board/]
const isTest = (name) => /\.test\.|\.unit\.test\./.test(name)
const isSource = (name) => /\.tsx?$/.test(name)

/** @type {{ file: string; line: number; text: string }[]} */
const hits = []

function walk(dir) {
  for (const entry of readdirSync(dir)) {
    const path = join(dir, entry)
    if (statSync(path).isDirectory()) {
      walk(path)
      continue
    }
    if (!isSource(entry) || isTest(entry)) continue
    const lines = readFileSync(path, 'utf8').split('\n')
    lines.forEach((text, i) => {
      if (FORBIDDEN.some((re) => re.test(text))) {
        hits.push({ file: path, line: i + 1, text: text.trim() })
      }
    })
  }
}

walk(ROOT)

if (hits.length > 0) {
  console.error('check:fake found forbidden demo references:')
  for (const h of hits) console.error(`  ${h.file}:${h.line}  ${h.text}`)
  process.exit(1)
}

console.log('check:fake clean — no demo references in shipped src')
