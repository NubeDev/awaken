import { createFileRoute } from '@tanstack/react-router'
import { Builder } from '@/features/builder'

// The dashboards surface: pick or create a board, then pin tiles. The builder's
// in-page picker drives selection; `/dashboards/$dashSlug` deep-links one.
export const Route = createFileRoute('/_authenticated/o/$org/dashboards/')({
  component: Builder,
})
