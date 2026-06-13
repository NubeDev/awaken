import { useCallback, useMemo, useState } from 'react'
import { useSearch } from '@tanstack/react-router'
import { Database, Plus, ScrollText, Search } from 'lucide-react'
import { useRules } from '@/api/hooks'
import type { RuleView } from '@/api/types'
import { Main } from '@/components/layout/main'
import { PageHeader } from '@/components/layout/page-header'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { useActiveSite } from '@/hooks/use-active-site'
import { RuleDebugger } from './components/rule-debugger'
import { RuleEditor } from './components/rule-editor'
import { RuleList } from './components/rule-list'
import { SqlWorkbench } from './components/sql-workbench'

type StudioTab = 'rules' | 'query'

/**
 * Rules Studio: author Rhai rules, dry-run them against real point history with
 * charts (the debugger), and explore the rows a rule sees with a SQL workbench —
 * all on the Sparks master/detail visual language. Org is the active tenant
 * (the site switcher's `site.org`).
 */
export function Rules() {
  const { site } = useActiveSite()
  const org = site?.org
  const { data: rules = [], isLoading } = useRules(org)
  const search = useSearch({ from: '/_authenticated/rules/' })
  const [tab, setTab] = useState<StudioTab>(search.tab === 'query' ? 'query' : 'rules')
  const [filter, setFilter] = useState('')
  // `null` selection with `creating` true = the "new rule" draft.
  const [selectedName, setSelectedName] = useState<string | undefined>()
  const [creating, setCreating] = useState(false)
  // The live editor draft, so the debugger dry-runs the on-screen (unsaved) script.
  const [draft, setDraft] = useState<{
    script: string
    params: Record<string, unknown>
  }>({ script: '', params: {} })
  const [debugKeyexpr, setDebugKeyexpr] = useState('')

  const filtered = useMemo(
    () =>
      rules
        .filter((r) => r.name.toLowerCase().includes(filter.toLowerCase()))
        .sort((a, b) => a.name.localeCompare(b.name)),
    [rules, filter]
  )

  const selected: RuleView | null =
    creating || selectedName === undefined
      ? null
      : (rules.find((r) => r.name === selectedName) ?? null)
  // Fall back to the first rule once loaded, unless mid-create.
  const effective =
    selected ?? (!creating && filtered.length ? filtered[0]! : null)

  const onDraftChange = useCallback(
    (d: { script: string; params: Record<string, unknown> }) => setDraft(d),
    []
  )

  const startNew = () => {
    setCreating(true)
    setSelectedName(undefined)
  }
  const onSelect = (name: string) => {
    setCreating(false)
    setSelectedName(name)
  }

  return (
    <>
      <PageHeader title='Rules Studio' sub='Author, debug, and explore rules' />
      <Main fluid fixed className='flex min-h-0'>
        <Tabs
          value={tab}
          onValueChange={(v) => setTab(v as StudioTab)}
          className='flex min-h-0 flex-1 flex-col gap-3'
        >
          <TabsList className='h-8 w-fit'>
            <TabsTrigger value='rules' className='gap-1.5 px-3 text-xs'>
              <ScrollText className='size-3.5' /> Rules
            </TabsTrigger>
            <TabsTrigger value='query' className='gap-1.5 px-3 text-xs'>
              <Database className='size-3.5' /> Query workbench
            </TabsTrigger>
          </TabsList>

          {tab === 'rules' ? (
            <div className='grid min-h-0 w-full flex-1 gap-4 lg:grid-cols-[300px_1fr]'>
              {/* master list */}
              <div className='flex min-h-0 flex-col gap-2.5'>
                <div className='flex items-center gap-2'>
                  <div className='relative flex-1'>
                    <Search className='text-muted-foreground absolute top-1/2 left-2 size-3.5 -translate-y-1/2' />
                    <Input
                      value={filter}
                      onChange={(e) => setFilter(e.target.value)}
                      placeholder='Search rules'
                      aria-label='Search rules'
                      className='h-8 ps-7 text-[12px]'
                    />
                  </div>
                  <Button
                    size='sm'
                    className='h-8 gap-1'
                    onClick={startNew}
                    aria-label='New rule'
                  >
                    <Plus className='size-3.5' /> New
                  </Button>
                </div>
                <Card className='scroll min-h-0 flex-1 gap-1 overflow-y-auto p-1.5'>
                  {isLoading ? (
                    <ListSkeleton />
                  ) : (
                    <RuleList
                      rules={filtered}
                      selectedName={creating ? undefined : effective?.name}
                      onSelect={onSelect}
                    />
                  )}
                </Card>
              </div>

              {/* detail: editor + debugger */}
              <div className='scroll min-h-0 space-y-4 overflow-y-auto pe-1'>
                {!org ? (
                  <Card className='grid h-full place-items-center'>
                    <p className='text-muted-foreground text-sm'>
                      Select a tenant to manage rules.
                    </p>
                  </Card>
                ) : creating || effective ? (
                  <>
                    <RuleEditor
                      key={creating ? '__new__' : effective?.name}
                      org={org}
                      rule={creating ? null : effective}
                      onDraftChange={onDraftChange}
                      onSaved={(name) => {
                        setCreating(false)
                        setSelectedName(name)
                      }}
                      onDeleted={() => {
                        setCreating(false)
                        setSelectedName(undefined)
                      }}
                    />
                    <RuleDebugger
                      org={org}
                      source={{ script: draft.script }}
                      params={draft.params}
                      keyexpr={debugKeyexpr}
                      onKeyexprChange={setDebugKeyexpr}
                    />
                  </>
                ) : (
                  <Card className='grid h-full place-items-center p-8'>
                    <div className='text-center'>
                      <ScrollText className='text-muted-foreground mx-auto size-8' />
                      <p className='text-muted-foreground mt-3 text-sm'>
                        Select a rule or create a new one.
                      </p>
                      <Button size='sm' className='mt-3 gap-1' onClick={startNew}>
                        <Plus className='size-3.5' /> New rule
                      </Button>
                    </div>
                  </Card>
                )}
              </div>
            </div>
          ) : (
            <div className='min-h-0 flex-1'>
              <SqlWorkbench onUseKeyexpr={setDebugKeyexpr} />
            </div>
          )}
        </Tabs>
      </Main>
    </>
  )
}

function ListSkeleton() {
  return (
    <div className='space-y-1.5 p-1'>
      {Array.from({ length: 6 }).map((_, i) => (
        <div key={i} className='bg-muted/50 h-11 animate-pulse rounded-md' />
      ))}
    </div>
  )
}
