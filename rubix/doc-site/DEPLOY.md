# Deploying the Rubix Docs Site

The site is a **static export** (`pnpm --filter @rubix/doc-site build` →
`doc-site/out/`). No server runs at runtime — any static host works.

Two env vars shape a deploy:

- `DOCS_BASE_PATH` — the sub-path the site is served from (empty if served at
  the host root). Used by `next.config.mjs` for asset/link URLs.
- `DOCS_REPO_URL` — source repo for the navbar / "Edit this page" links.

## Option A — GitHub Pages (own repo)

If Rubix has its own repository, enable Pages and deploy `out/`.

- Project site → served at `https://<org>.github.io/<repo>`, so set
  `DOCS_BASE_PATH=/<repo>`.
- User/org site or custom domain at root → leave `DOCS_BASE_PATH` empty.

A workflow that builds and publishes:

```yaml
name: Docs
on:
  push: { branches: [main] }
  workflow_dispatch:
permissions: { contents: read, pages: write, id-token: write }
concurrency: { group: pages, cancel-in-progress: true }
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v4
      - uses: actions/setup-node@v4
        with: { node-version: 22, cache: pnpm }
      - run: pnpm install --frozen-lockfile
      - env: { DOCS_BASE_PATH: /<repo> }
        run: pnpm --filter @rubix/doc-site build
      - uses: actions/upload-pages-artifact@v3
        with: { path: doc-site/out }
  deploy:
    needs: build
    runs-on: ubuntu-latest
    environment: { name: github-pages, url: '${{ steps.d.outputs.page_url }}' }
    steps:
      - id: d
        uses: actions/deploy-pages@v4
```

> **If Rubix currently lives inside a larger monorepo that already publishes
> GitHub Pages**, note that GitHub serves only **one** Pages site per repo — a
> second independent Pages deploy will conflict. Either move Rubix to its own
> repo (cleanest), nest this build under a sub-path of the existing Pages
> artifact, or use Option B.

## Option B — Non-GitHub static host

Deploy `out/` to Cloudflare Pages / Netlify / S3 + CloudFront. No sub-path
needed (`DOCS_BASE_PATH` empty), custom-domain friendly, independent of any
other site.

## Local production check

```sh
pnpm --filter @rubix/doc-site build
npx serve doc-site/out
```

## Notes

- `next.config.mjs` sets `output: 'export'`, `images.unoptimized`,
  `trailingSlash: true`, and reads `DOCS_BASE_PATH` / `DOCS_REPO_URL`.
- `postbuild` writes `out/.nojekyll` so GitHub Pages doesn't strip
  `_next/`-style folders.
