'use client'

// Interactive OpenAPI reference, rendered client-side from the static spec.
//
// The build emits `public/openapi.json` (see scripts/generate-openapi.mjs) from
// the server's utoipa document. Scalar fetches it in the browser and renders the
// full reference + "try it" UI, so this works in the static export with no
// server at runtime. Marked 'use client' because Scalar is browser-only.
import { ApiReferenceReact } from '@scalar/api-reference-react'
import '@scalar/api-reference-react/style.css'

// Honour the export base path (e.g. "/rubix" on a project site) so the spec URL
// resolves whether the docs are served from "/" or a sub-path.
const basePath = process.env.NEXT_PUBLIC_DOCS_BASE_PATH || ''

export default function ApiExplorer() {
  return (
    // `rubix-api-explorer` (see app/globals.css) breaks this embed out of the
    // centered prose column so Scalar's two-pane reference gets the full width.
    <div className="rubix-api-explorer">
      <ApiReferenceReact
        configuration={{
          url: `${basePath}/openapi.json`,
          // Embed cleanly inside the docs page rather than taking over the layout.
          hideClientButton: true,
        }}
      />
    </div>
  )
}
