import { createFileRoute } from '@tanstack/react-router'
import { Builder } from '@/features/builder'

// Deep-link to one dashboard; the builder selects it from the `dashSlug` param.
export const Route = createFileRoute('/_authenticated/o/$org/dashboards/$dashSlug/')({
  component: Builder,
})
