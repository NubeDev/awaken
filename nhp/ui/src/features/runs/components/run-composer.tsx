import { useRef, useState } from 'react'
import { Send, Sparkles } from 'lucide-react'
import { useNavigate } from '@tanstack/react-router'
import { toast } from 'sonner'
import { useQueryClient } from '@tanstack/react-query'
import { useScope } from '@/context/scope-provider'
import { useAgentChat } from '@/api/hooks'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { Textarea } from '@/components/ui/textarea'
import { cn } from '@/lib/utils'

/** A stable per-tab thread so consecutive prompts from this page stay coherent. */
function runsThreadId(): string {
  try {
    const key = 'rubix.runs.thread'
    const existing = sessionStorage.getItem(key)
    if (existing) return existing
    const fresh = `runs-${crypto.randomUUID()}`
    sessionStorage.setItem(key, fresh)
    return fresh
  } catch {
    return `runs-${crypto.randomUUID()}`
  }
}

const AI_OFF_HINT =
  'The agent is off. Start the server with RUBIX_AI=1 (and an OPENAI_API_KEY) to enable runs.'

/**
 * Start an agent run from the Agent Runs page itself — no hunting for the
 * sidebar sparkle. Posts to `/agent/chat`; a completed reply refreshes the list
 * (the new run appears as a row), an awaiting-approval reply jumps straight to
 * its suspended run so the operator can approve. A 503 is surfaced as the
 * RUBIX_AI-off hint rather than a raw error.
 */
export function RunComposer() {
  const navigate = useNavigate()
  const qc = useQueryClient()
  const { org, site } = useScope()
  const chat = useAgentChat()
  const threadId = useRef(runsThreadId())
  const [draft, setDraft] = useState('')

  const send = () => {
    const message = draft.trim()
    if (!message || chat.isPending) return
    chat.mutate(
      { thread_id: threadId.current, message },
      {
        onSuccess: (res) => {
          setDraft('')
          qc.invalidateQueries({ queryKey: ['runs'] })
          if (res.status === 'awaiting_approval' && res.run_id && org && site) {
            toast.warning('Run awaiting approval', {
              description: 'The agent holds a write for your review.',
            })
            navigate({
              to: '/o/$org/s/$siteSlug/runs/$runId',
              params: { org, siteSlug: site.slug, runId: res.run_id },
            })
          } else {
            toast.success('Run completed', {
              description: `awaken finished in ${res.steps} step${res.steps === 1 ? '' : 's'}.`,
            })
          }
        },
        onError: (e) => {
          const msg = (e as Error).message
          toast.error('Run failed', {
            description: /503/.test(msg) ? AI_OFF_HINT : msg,
          })
        },
      }
    )
  }

  return (
    <Card className='gap-0 p-0'>
      <div className='flex items-end gap-2 p-2.5'>
        <Sparkles className='text-primary mb-2 ms-1 size-4 shrink-0' />
        <Textarea
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter' && !e.shiftKey) {
              e.preventDefault()
              send()
            }
          }}
          placeholder='Ask awaken to investigate a point, run a board, or command a write…'
          className={cn(
            'max-h-40 min-h-9 resize-none border-0 px-1 text-[13px] shadow-none',
            'focus-visible:ring-0'
          )}
          rows={1}
          disabled={chat.isPending}
        />
        <Button
          size='sm'
          onClick={send}
          disabled={!draft.trim() || chat.isPending}
          className='mb-0.5'
        >
          {chat.isPending ? (
            'Running…'
          ) : (
            <>
              <Send className='size-3.5' /> Run
            </>
          )}
        </Button>
      </div>
    </Card>
  )
}
