import { Footer, Layout, Navbar } from 'nextra-theme-docs'
import { Head } from 'nextra/components'
import { getPageMap } from 'nextra/page-map'
import 'nextra-theme-docs/style.css'

// Source repo link. Set DOCS_REPO_URL in the environment to point the navbar /
// "Edit this page" links at the repository. Defaults to a neutral placeholder.
const repoUrl = process.env.DOCS_REPO_URL || 'https://github.com/your-org/rubix'

export const metadata = {
  title: {
    default: 'Rubix Docs',
    template: '%s – Rubix Docs',
  },
  description:
    'Rubix — a generic, AI-ready, edge-to-cloud data processing platform on SurrealDB.',
}

const navbar = (
  <Navbar logo={<b>Rubix</b>} projectLink={repoUrl} />
)

const footer = <Footer>{new Date().getFullYear()} © Rubix</Footer>

export default async function RootLayout({ children }) {
  const pageMap = await getPageMap()
  return (
    <html lang="en" dir="ltr" suppressHydrationWarning>
      <Head />
      <body>
        <Layout
          navbar={navbar}
          footer={footer}
          pageMap={pageMap}
          docsRepositoryBase={`${repoUrl}/tree/main/doc-site`}
        >
          {children}
        </Layout>
      </body>
    </html>
  )
}
