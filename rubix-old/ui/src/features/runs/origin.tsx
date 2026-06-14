import { MessageSquare, Radio, Plug, type LucideIcon } from 'lucide-react'
import type { RunOrigin } from '@/api/types'

/**
 * What raised a run, with the label + icon used across the runs surface.
 * `chat` is an operator asking awaken; `dispatch` is a spark-triggered
 * investigation; `mcp` is an external client over the MCP adapter. Centralised
 * here so the list row, detail header, and filter chips stay consistent.
 */
export const ORIGIN_META: Record<RunOrigin, { label: string; icon: LucideIcon }> = {
  chat: { label: 'Chat', icon: MessageSquare },
  dispatch: { label: 'Dispatch', icon: Radio },
  mcp: { label: 'MCP', icon: Plug },
}
