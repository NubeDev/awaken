// Makes Pagefind search work in `pnpm dev`.
//
// Pagefind indexes built `.html`, which only exists after `next build` (in
// `out/`). `next dev` renders pages on the fly and has nothing to index, so the
// search box otherwise shows "Failed to load search index". Nextra always
// fetches `/_pagefind/pagefind.js`, and `next dev` serves files from `public/`,
// so we generate the index once from the last static build and drop it into
// `public/_pagefind`. Dev then serves a real (possibly slightly stale) index.
//
// Runs via the `predev` npm script. It does NOT trigger a build itself — if
// there's no `out/` yet it just prints a hint and exits 0, so `pnpm dev` never
// fails because of search. Re-run `pnpm build` (or `pnpm dev`) after editing
// content to refresh the dev index.
import { existsSync, rmSync, cpSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, resolve } from 'node:path'
import { execFileSync } from 'node:child_process'

const here = dirname(fileURLToPath(import.meta.url))
const root = resolve(here, '..')
const outDir = resolve(root, 'out')
const builtIndex = resolve(outDir, '_pagefind')
const publicIndex = resolve(root, 'public', '_pagefind')

if (!existsSync(outDir)) {
  console.log(
    '[search] no static build found (out/) — dev search index skipped.\n' +
      '[search] run `pnpm build` once to enable search in `pnpm dev`.',
  )
  process.exit(0)
}

// Index the built HTML. Mirrors the `postbuild` step's explicit output path so
// the index lands where Nextra fetches it (/_pagefind, not /pagefind). Resolve
// the local binary rather than relying on PATH, so the script works whether or
// not it's invoked through an npm lifecycle script.
const pagefindBin = resolve(root, 'node_modules', '.bin', 'pagefind')
const pagefind = existsSync(pagefindBin) ? pagefindBin : 'pagefind'
execFileSync(
  pagefind,
  ['--site', 'out', '--output-path', 'out/_pagefind'],
  { cwd: root, stdio: 'inherit' },
)

// Publish the index where `next dev` can serve it.
rmSync(publicIndex, { recursive: true, force: true })
cpSync(builtIndex, publicIndex, { recursive: true })
console.log(`[search] dev search index ready -> public/_pagefind`)
