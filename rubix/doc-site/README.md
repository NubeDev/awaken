# Rubix Docs (`@rubix/doc-site`)

Production documentation site for **Rubix**, built with
[Nextra 4](https://nextra.site).

> Internal architecture specs live in `rubix/docs/design` and are **not**
> published. This site is the user-facing, curated documentation.

## Develop

```sh
pnpm install
pnpm --filter @rubix/doc-site dev
# http://localhost:3010
```

## Build (static export)

```sh
pnpm --filter @rubix/doc-site build
# output: doc-site/out
```

The build statically exports to `out/` (no server at runtime). Configure these
env vars as needed:

- `DOCS_REPO_URL` — source repo for the navbar / "Edit this page" links
  (default: a neutral placeholder).
- `DOCS_BASE_PATH` — sub-path the site is served from, when not at the host
  root (e.g. `/rubix`). Empty locally.

## Authoring

- Pages are MDX under `content/`.
- Sidebar order / labels come from `_meta.js` files in each folder.
- The **Changelog** page is generated from `../CHANGELOG.md` by
  `scripts/sync-changelog.mjs` (runs on `predev` / `prebuild`). Edit the
  changelog there, never `content/changelog.mdx`.

## Versioning

See [VERSIONING.md](./VERSIONING.md). Today the site is **latest-only**;
per-release snapshots come later via git tags.

## Pinned dependencies (why)

- `next` is pinned to `15.3.5` — Next 15.5's RSC bundler breaks nextra's static
  export ("Could not find module ... in the React Client Manifest").
- `nextra`'s `zod` is pinned to `4.1.12` (via root `pnpm.overrides`) — zod ≥4.4
  makes `z.custom()` reject `undefined`, crashing every page render.
