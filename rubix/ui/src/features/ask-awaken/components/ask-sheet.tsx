import { useRef, useState } from 'react'
import { Send, Sparkles } from 'lucide-react'
import { useNavigate } from '@tanstack/react-router'
import { useAgentChat } from '@/api/hooks'
import { Button } from '@/components/ui/button'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Sheet, SheetContent, SheetHeader, SheetTitle } from '@/components/ui/sheet'
import { Textarea } from '@/components/ui/textarea'
import { useAskAwaken } from '../use-ask-awaken'
import { askThreadId } from '../thread-id'
import { AskTurn, type Turn } from './ask-turn'

/**
 * A minimal request/response chat with the embedded agent on a persistent
 * thread. The `/agent/chat` endpoint is synchronous (no streaming), so each
 * send appends one user turn and, on resolve, the agent's final reply; an
 * awaiting-approval reply carries a link to the suspended run.
 */
export function AskSheet() {
  const { open, setOpen } = useAskAwaken()
  const navigate = useNavigate()
  const chat = useAgentChat()
  const threadId = useRef(askThreadId())
  const [draft, setDraft] = useState('')
  const [turns, setTurns] = useState<Turn[]>([])

  const send = () => {
    const message = draft.trim()
    if (!message || chat.isPending) return
    setTurns((t) => [...t, { role: 'user', text: message }])
    setDraft('')
    chat.mutate(
      { thread_id: threadId.current, message },
      {
        onSuccess: (res) =>
          setTurns((t) => [
            ...t,
            { role: 'agent', text: res.response, runId: res.run_id, status: res.status },
          ]),
        onError: (e) =>
          setTurns((t) => [...t, { role: 'error', text: (e as Error).message }]),
      }
    )
  }

  return (
    <Sheet open={open} onOpenChange={setOpen}>
      <SheetContent className='sm:max-w-md'>
        <SheetHeader className='border-b'>
          <SheetTitle className='flex items-center gap-2 text-[15px]'>
            <Sparkles className='text-primary size-4' /> Ask awaken
          </SheetTitle>
        </SheetHeader>

        <ScrollArea className='min-h-0 flex-1 px-4'>
          {turns.length === 0 ? (
            <p className='text-muted-foreground py-12 text-center text-[13px]'>
              Ask about a finding, a point, or what awaken can do.
            </p>
          ) : (
            <div className='space-y-3 py-2'>
              {turns.map((turn, i) => (
                <AskTurn
                  key={i}
                  turn={turn}
                  onReview={(runId) => {
                    setOpen(false)
                    navigate({ to: '/runs/$runId', params: { runId } })
                  }}
                />
              ))}
              {chat.isPending ? (
                <p className='text-muted-foreground text-[12px]'>awaken is thinking…</p>
              ) : null}
            </div>
          )}
        </ScrollArea>

        <div className='flex items-end gap-2 border-t p-3'>
          <Textarea
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter' && !e.shiftKey) {
                e.preventDefault()
                send()
              }
            }}
            placeholder='Message awaken…'
            className='max-h-32 min-h-9 resize-none text-[13px]'
            rows={1}
          />
          <Button size='icon' onClick={send} disabled={!draft.trim() || chat.isPending}>
            <Send className='size-4' />
          </Button>
        </div>
      </SheetContent>
    </Sheet>
  )
}
