import { createFileRoute } from '@tanstack/react-router'
import { WiresheetPage } from '@/features/wiresheet'

/**
 * Logic Studio — the react-flow wiresheet editor (sidebar → Configure → Logic
 * Studio). A demo surface for composing extra EMS logic (custom alarming /
 * reporting) over the live portfolio; see features/wiresheet.
 */
export const Route = createFileRoute('/_authenticated/logic-studio')({
  component: WiresheetPage,
})
