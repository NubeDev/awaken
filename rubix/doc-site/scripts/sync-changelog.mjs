// Copies the canonical rubix/CHANGELOG.md into the docs content tree as an MDX
// page so the published Changelog never drifts from the source of truth.
// Runs automatically via the `predev` / `prebuild` npm scripts.
import { readFileSync, writeFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, resolve } from 'node:path'

const here = dirname(fileURLToPath(import.meta.url))
const source = resolve(here, '..', '..', 'CHANGELOG.md')
const target = resolve(here, '..', 'content', 'changelog.mdx')

const body = readFileSync(source, 'utf8')

// Keep a Changelog files start with "# Changelog"; strip the H1 since the page
// title is supplied by frontmatter, and prepend frontmatter for Nextra.
const withoutH1 = body.replace(/^#\s+Changelog\s*\n/, '')
const page = `---\ntitle: Changelog\n---\n\n{/* Generated from rubix/CHANGELOG.md by scripts/sync-changelog.mjs — do not edit by hand. */}\n\n# Changelog\n\n${withoutH1}`

writeFileSync(target, page)
console.log(`synced changelog -> ${target}`)
