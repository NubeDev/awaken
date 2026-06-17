// Generate the static OpenAPI spec the API explorer page renders.
//
// The server builds its OpenAPI 3.1 document with utoipa and serves it at
// `GET /api-docs/openapi.json` at runtime. The doc-site is a static export with
// no server, so we materialise the same document at build time by running the
// `dump_openapi` example (which prints `rubix_server::openapi_document()` as
// JSON) and writing it into `public/openapi.json`. The API explorer page then
// fetches it client-side and renders it with Scalar.
//
// Runs via the `predev`/`prebuild` npm scripts. If cargo isn't available (e.g. a
// docs-only CI runner) it prints a hint and exits 0 so the docs still build —
// any previously generated `public/openapi.json` is left in place.
import { existsSync, writeFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, resolve } from 'node:path'
import { execFileSync } from 'node:child_process'

const here = dirname(fileURLToPath(import.meta.url))
const root = resolve(here, '..')
const rubixRoot = resolve(root, '..')
const outFile = resolve(root, 'public', 'openapi.json')

// Resolve cargo: prefer the user-local toolchain (not always on PATH), then PATH.
const cargoHome = resolve(process.env.HOME || '', '.cargo', 'bin', 'cargo')
const cargo = existsSync(cargoHome) ? cargoHome : 'cargo'

try {
  const json = execFileSync(
    cargo,
    ['run', '-q', '-p', 'rubix-server', '--example', 'dump_openapi'],
    { cwd: rubixRoot, encoding: 'utf8', maxBuffer: 32 * 1024 * 1024 },
  )
  // Validate it parses before writing, so a partial/garbled build can't poison
  // the page with invalid JSON.
  JSON.parse(json)
  writeFileSync(outFile, json)
  console.log(`[openapi] wrote ${outFile}`)
} catch (err) {
  if (existsSync(outFile)) {
    console.log(
      `[openapi] generation failed (${err.message?.split('\n')[0]}); ` +
        `keeping existing public/openapi.json`,
    )
  } else {
    console.log(
      `[openapi] generation failed (${err.message?.split('\n')[0]}); ` +
        `API explorer will have no spec to render.\n` +
        `[openapi] run with a working cargo toolchain to generate public/openapi.json.`,
    )
  }
  process.exit(0)
}
