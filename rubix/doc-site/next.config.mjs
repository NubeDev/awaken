import nextra from 'nextra'

const withNextra = nextra({
  // Nextra 4 reads MDX from ./content and builds the page map automatically.
  defaultShowCopyCode: true,
})

// When the site is served from a sub-path (e.g. a GitHub Pages project site),
// set DOCS_BASE_PATH in CI (e.g. "/rubix"). Locally it stays empty so
// `pnpm dev` serves from "/".
const basePath = process.env.DOCS_BASE_PATH || ''

export default withNextra({
  // Static HTML export — no Node server at runtime, suitable for GitHub Pages.
  output: 'export',
  images: { unoptimized: true },
  basePath,
  // Pages hosts trailing-slash directories more reliably for static export.
  trailingSlash: true,
})
