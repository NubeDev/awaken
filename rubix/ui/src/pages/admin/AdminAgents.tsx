// Admin · Agents — provision and manage AI agents, driving the agent surface
// (crates/rubix-server/src/http/agent). An agent is a scoped service-account
// principal at a tier (analyst ⊂ operator ⊂ actuator, AGENT.md); provisioning it
// is the human-admin action that grants that tier. No domain concept appears here.

import { getRouteApi } from '@tanstack/react-router'
import { useState } from 'react'
import { Bot, Plus } from 'lucide-react'
import { useAgents, useAgentMutations } from '../../hooks/useAgents'
import { AdminLayout } from '../../components/admin/AdminLayout'
import { ErrorView, LoadingView, EmptyView } from '../../components/ui/StateView'
import { Button } from '../../components/ui/button'
import { Input } from '../../components/ui/input'
import { Label } from '../../components/ui/label'
import { Badge } from '../../components/ui/badge'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../../components/ui/dialog'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '../../components/ui/select'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '../../components/ui/table'
import { useToast } from '../../components/ui/toast'
import { AGENT_TIERS, TIER_SUMMARY, type AgentTier, type ProvisionAgentRequest } from '../../types/Agent'

const route = getRouteApi('/t/$tenant/admin/agents')

// The tier badge colour deepens with authority, so an actuator (commands
// hardware) reads as the most privileged at a glance.
const TIER_VARIANT: Record<string, 'secondary' | 'default' | 'destructive'> = {
  analyst: 'secondary',
  operator: 'default',
  actuator: 'destructive',
}

export function AdminAgents() {
  const { tenant } = route.useParams()
  const agents = useAgents(tenant)
  const { toast } = useToast()
  const [createOpen, setCreateOpen] = useState(false)

  return (
    <AdminLayout active="agents">
      <div className="mx-auto max-w-[1100px]">
        <div className="mb-5 flex items-center justify-between gap-3">
          <div className="flex items-center gap-3">
            <div className="grid size-11 place-items-center rounded-xl border border-border bg-card">
              <Bot size={20} className="text-muted-foreground" />
            </div>
            <div>
              <h1 className="text-[22px] font-semibold tracking-tight">Agents</h1>
              <div className="text-[13px] text-muted-foreground">
                AI agents provisioned as scoped principals in {tenant}.
              </div>
            </div>
          </div>
          <Button onClick={() => setCreateOpen(true)} className="gap-1.5">
            <Plus size={16} /> New agent
          </Button>
        </div>

        {agents.isLoading && <LoadingView label="Loading agents…" />}
        {agents.error && <ErrorView error={agents.error} />}
        {agents.data && agents.data.length === 0 && (
          <EmptyView title="No agents" hint="Provision one to give it a tier and let it answer." />
        )}

        {agents.data && agents.data.length > 0 && (
          <div className="rounded-xl border border-border bg-card/40">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Subject</TableHead>
                  <TableHead>Tier</TableHead>
                  <TableHead>What it can do</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {agents.data.map((a) => (
                  <TableRow key={a.subject}>
                    <TableCell className="mono font-medium">{a.subject}</TableCell>
                    <TableCell>
                      <Badge variant={TIER_VARIANT[a.tier] ?? 'secondary'} className="text-[10px]">
                        {a.tier}
                      </Badge>
                    </TableCell>
                    <TableCell className="text-[12.5px] text-muted-foreground">
                      {TIER_SUMMARY[a.tier as AgentTier] ?? '—'}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
        )}

        <p className="mt-4 text-[12px] text-muted-foreground">
          Tiers are layered — promoting an agent means granting the next tier, never a rebuild.
          An agent can never escalate its own authority; only an admin confers a tier.
        </p>
      </div>

      <ProvisionAgentDialog
        tenant={tenant}
        open={createOpen}
        onOpenChange={setCreateOpen}
        onProvisioned={() => toast('Agent provisioned')}
      />
    </AdminLayout>
  )
}

function ProvisionAgentDialog({
  tenant,
  open,
  onOpenChange,
  onProvisioned,
}: {
  tenant: string
  open: boolean
  onOpenChange: (open: boolean) => void
  onProvisioned: () => void
}) {
  const { provision } = useAgentMutations(tenant)
  const { toast } = useToast()
  const [form, setForm] = useState<ProvisionAgentRequest>({ subject: '', tier: 'analyst', secret: '' })
  const [mintedSecret, setMintedSecret] = useState<string | null>(null)

  function set<K extends keyof ProvisionAgentRequest>(key: K, value: ProvisionAgentRequest[K]) {
    setForm((f) => ({ ...f, [key]: value }))
  }

  async function handleProvision() {
    try {
      // Empty secret ⇒ omit it so the server mints one and returns it once.
      const body: ProvisionAgentRequest = { ...form, secret: form.secret || undefined }
      const created = await provision.mutateAsync(body)
      if (created.secret) {
        setMintedSecret(created.secret)
        toast('Agent provisioned — copy the secret now')
      } else {
        onProvisioned()
        onOpenChange(false)
      }
      setForm({ subject: '', tier: 'analyst', secret: '' })
    } catch (e) {
      toast(e instanceof Error ? e.message : 'Provisioning failed', 'error')
    }
  }

  return (
    <Dialog
      open={open}
      onOpenChange={(o) => {
        if (!o) setMintedSecret(null)
        onOpenChange(o)
      }}
    >
      <DialogContent>
        <DialogHeader>
          <DialogTitle>New agent</DialogTitle>
          <DialogDescription>
            Provision an AI agent at a tier. Leave the secret blank to have the server
            generate one (shown once).
          </DialogDescription>
        </DialogHeader>

        {mintedSecret ? (
          <div className="flex flex-col gap-2">
            <Label>Generated secret — copy it now, it is shown only once</Label>
            <code className="mono select-all break-all rounded-md border border-border bg-card px-3 py-2 text-xs">
              {mintedSecret}
            </code>
          </div>
        ) : (
          <div className="flex flex-col gap-4">
            <div className="flex flex-col gap-2">
              <Label htmlFor="a-subject">Subject</Label>
              <Input
                id="a-subject"
                value={form.subject}
                onChange={(e) => set('subject', e.target.value)}
                placeholder="avery"
                autoComplete="off"
              />
            </div>
            <div className="flex flex-col gap-2">
              <Label>Tier</Label>
              <Select value={form.tier} onValueChange={(v) => set('tier', v)}>
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {AGENT_TIERS.map((t) => (
                    <SelectItem key={t} value={t}>
                      {t}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <p className="text-[12px] text-muted-foreground">
                {TIER_SUMMARY[form.tier as AgentTier]}
              </p>
            </div>
            <div className="flex flex-col gap-2">
              <Label htmlFor="a-secret">Secret (optional)</Label>
              <Input
                id="a-secret"
                value={form.secret}
                onChange={(e) => set('secret', e.target.value)}
                placeholder="leave blank to generate"
                autoComplete="off"
              />
            </div>
          </div>
        )}

        <DialogFooter>
          {mintedSecret ? (
            <Button
              onClick={() => {
                onProvisioned()
                onOpenChange(false)
              }}
            >
              Done
            </Button>
          ) : (
            <>
              <Button variant="ghost" onClick={() => onOpenChange(false)} disabled={provision.isPending}>
                Cancel
              </Button>
              <Button onClick={handleProvision} disabled={provision.isPending || !form.subject}>
                {provision.isPending ? 'Provisioning…' : 'Provision'}
              </Button>
            </>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
