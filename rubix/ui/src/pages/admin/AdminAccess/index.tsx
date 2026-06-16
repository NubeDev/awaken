// Admin · Access — the single console for *who* (principals/people) and *how they
// are grouped* (teams), merging what used to be two routes (/admin/principals and
// /admin/teams) under one surface with tabs. A principal is the substrate identity
// (subject + namespace + kind + role); a team groups principals and a grant on a
// team is inherited by every member (the gate's effective-grants union). The Teams
// tab previews each team's members and inherited access inline, so the blast radius
// of a team is visible without opening it. Backs crates/rubix-server/src/http/admin.

import { getRouteApi } from '@tanstack/react-router'
import { useState } from 'react'
import { KeyRound, Plus, Trash2, Users, UserPlus, X } from 'lucide-react'
import { useApi } from '../../../api/ConnectionContext'
import { useQuery } from '@tanstack/react-query'
import { listGrants, listTeamMembers, listTeamGrants } from '../../../api/admin'
import { usePrincipals, useTeams } from '../../../hooks/useAdmin'
import {
  usePrincipalMutations,
  useGrantMutations,
  useTeamMutations,
  useTeamDetailMutations,
} from '../../../hooks/useAdminMutations'
import { usePageHeader } from '../../../components/shell/page-header'
import { ErrorView, LoadingView, EmptyView } from '../../../components/ui/StateView'
import { Button } from '../../../components/ui/button'
import { Input } from '../../../components/ui/input'
import { Label } from '../../../components/ui/label'
import { Badge } from '../../../components/ui/badge'
import { Checkbox } from '../../../components/ui/checkbox'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../../../components/ui/tabs'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../../../components/ui/dialog'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '../../../components/ui/select'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '../../../components/ui/table'
import { useToast } from '../../../components/ui/toast'
import {
  CAPABILITIES,
  PRINCIPAL_KINDS,
  ROLES,
  type Principal,
  type CreatePrincipalRequest,
  type Team,
} from '../../../types/Admin'

const route = getRouteApi('/t/$tenant/admin/access')

export function AdminAccess() {
  const { tenant } = route.useParams()
  usePageHeader({ crumbs: ['Admin', 'Access'] })

  return (
    <div className="px-6 py-6">
      <div className="mx-auto max-w-[1100px]">
        <div className="mb-5 flex items-center gap-3">
          <div className="grid size-11 place-items-center rounded-xl border border-border bg-card">
            <KeyRound size={20} className="text-muted-foreground" />
          </div>
          <div>
            <h1 className="text-[22px] font-semibold tracking-tight">Access</h1>
            <div className="text-[13px] text-muted-foreground">
              People and the teams that grant them access in {tenant}.
            </div>
          </div>
        </div>

        <Tabs defaultValue="people">
          <TabsList>
            <TabsTrigger value="people" className="gap-1.5">
              <KeyRound size={14} /> People
            </TabsTrigger>
            <TabsTrigger value="teams" className="gap-1.5">
              <Users size={14} /> Teams
            </TabsTrigger>
          </TabsList>

          <TabsContent value="people">
            <PeopleTab tenant={tenant} />
          </TabsContent>
          <TabsContent value="teams">
            <TeamsTab tenant={tenant} />
          </TabsContent>
        </Tabs>
      </div>
    </div>
  )
}

// — People ————————————————————————————————————————————————————————————————————

function PeopleTab({ tenant }: { tenant: string }) {
  const principals = usePrincipals(tenant)
  const { setRole, remove } = usePrincipalMutations(tenant)
  const { toast } = useToast()

  const [createOpen, setCreateOpen] = useState(false)
  const [selected, setSelected] = useState<Principal | null>(null)

  async function handleRole(subject: string, role: string) {
    try {
      await setRole.mutateAsync({ subject, role })
      toast('Role updated')
    } catch (e) {
      toast(e instanceof Error ? e.message : 'Update failed', 'error')
    }
  }

  async function handleDelete(p: Principal) {
    if (!window.confirm(`Delete principal ${p.subject}?`)) return
    try {
      await remove.mutateAsync(p.subject)
      toast('Principal deleted')
    } catch (e) {
      toast(e instanceof Error ? e.message : 'Delete failed', 'error')
    }
  }

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center justify-between gap-3">
        <p className="text-[13px] text-muted-foreground">
          Identities and their capability grants.
        </p>
        <Button onClick={() => setCreateOpen(true)} className="gap-1.5">
          <Plus size={16} /> New principal
        </Button>
      </div>

      {principals.isLoading && <LoadingView label="Loading principals…" />}
      {principals.error && <ErrorView error={principals.error} />}
      {principals.data && principals.data.length === 0 && (
        <EmptyView title="No principals" hint="Create one to grant it capabilities." />
      )}

      {principals.data && principals.data.length > 0 && (
        <div className="rounded-xl border border-border bg-card/40">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Subject</TableHead>
                <TableHead>Kind</TableHead>
                <TableHead>Role</TableHead>
                <TableHead>Grants</TableHead>
                <TableHead />
              </TableRow>
            </TableHeader>
            <TableBody>
              {principals.data.map((p) => (
                <TableRow key={p.subject}>
                  <TableCell className="mono font-medium">{p.subject}</TableCell>
                  <TableCell>
                    <Badge variant="secondary" className="text-[10px]">
                      {p.kind}
                    </Badge>
                  </TableCell>
                  <TableCell>
                    <Select value={p.role} onValueChange={(role) => handleRole(p.subject, role)}>
                      <SelectTrigger className="h-8 w-[120px]">
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        {ROLES.map((r) => (
                          <SelectItem key={r} value={r}>
                            {r}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </TableCell>
                  <TableCell>
                    <Button variant="outline" size="sm" onClick={() => setSelected(p)}>
                      Manage
                    </Button>
                  </TableCell>
                  <TableCell className="text-right">
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={() => handleDelete(p)}
                      aria-label="Delete principal"
                    >
                      <Trash2 size={14} className="text-destructive" />
                    </Button>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      )}

      <CreatePrincipalDialog tenant={tenant} open={createOpen} onOpenChange={setCreateOpen} />
      {selected && (
        <GrantsDialog
          tenant={tenant}
          principal={selected}
          open={selected !== null}
          onOpenChange={(o) => !o && setSelected(null)}
        />
      )}
    </div>
  )
}

function CreatePrincipalDialog({
  tenant,
  open,
  onOpenChange,
}: {
  tenant: string
  open: boolean
  onOpenChange: (open: boolean) => void
}) {
  const { create } = usePrincipalMutations(tenant)
  const { toast } = useToast()
  const [form, setForm] = useState<CreatePrincipalRequest>({
    subject: '',
    kind: 'user',
    role: 'viewer',
  })
  const [mintedSecret, setMintedSecret] = useState<string | null>(null)

  function set<K extends keyof CreatePrincipalRequest>(key: K, value: CreatePrincipalRequest[K]) {
    setForm((f) => ({ ...f, [key]: value }))
  }

  async function handleCreate() {
    try {
      // The secret is always server-minted and shown once — never typed here
      // (a hand-entered shared secret is the awkward UX we're avoiding). Omitting
      // it tells the server to generate and return one.
      const created = await create.mutateAsync({ ...form, secret: undefined })
      setMintedSecret(created.secret ?? '(no secret returned)')
      toast('Created — copy the secret now')
      setForm({ subject: '', kind: 'user', role: 'viewer' })
    } catch (e) {
      toast(e instanceof Error ? e.message : 'Create failed', 'error')
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
          <DialogTitle>New principal</DialogTitle>
          <DialogDescription>
            The server generates a secret and shows it once — copy it before closing.
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
              <Label htmlFor="p-subject">Subject</Label>
              <Input
                id="p-subject"
                value={form.subject}
                onChange={(e) => set('subject', e.target.value)}
                placeholder="alice"
                autoComplete="off"
              />
            </div>
            <div className="grid grid-cols-2 gap-3">
              <div className="flex flex-col gap-2">
                <Label>Kind</Label>
                <Select value={form.kind} onValueChange={(v) => set('kind', v)}>
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {PRINCIPAL_KINDS.map((k) => (
                      <SelectItem key={k} value={k}>
                        {k}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
              <div className="flex flex-col gap-2">
                <Label>Role</Label>
                <Select value={form.role} onValueChange={(v) => set('role', v)}>
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {ROLES.map((r) => (
                      <SelectItem key={r} value={r}>
                        {r}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
            </div>
          </div>
        )}

        <DialogFooter>
          {mintedSecret ? (
            <Button onClick={() => onOpenChange(false)}>Done</Button>
          ) : (
            <>
              <Button variant="ghost" onClick={() => onOpenChange(false)} disabled={create.isPending}>
                Cancel
              </Button>
              <Button onClick={handleCreate} disabled={create.isPending || !form.subject}>
                {create.isPending ? 'Creating…' : 'Create'}
              </Button>
            </>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

function GrantsDialog({
  tenant,
  principal,
  open,
  onOpenChange,
}: {
  tenant: string
  principal: Principal
  open: boolean
  onOpenChange: (open: boolean) => void
}) {
  const api = useApi(tenant)
  const grants = useQuery({
    queryKey: ['grants', tenant, principal.subject],
    queryFn: () => listGrants(api, principal.subject),
  })
  const { grant, revoke } = useGrantMutations(tenant, principal.subject)
  const { toast } = useToast()

  const held = new Set((grants.data ?? []).map((g) => g.capability))

  async function toggle(capability: string, on: boolean) {
    try {
      if (on) await grant.mutateAsync(capability)
      else await revoke.mutateAsync(capability)
      toast(on ? `Granted ${capability}` : `Revoked ${capability}`)
    } catch (e) {
      toast(e instanceof Error ? e.message : 'Grant change failed', 'error')
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>
            Grants · <span className="mono">{principal.subject}</span>
          </DialogTitle>
          <DialogDescription>Toggle the capabilities this principal holds.</DialogDescription>
        </DialogHeader>

        {grants.isLoading && <p className="text-sm text-muted-foreground">Loading grants…</p>}
        {grants.error && <ErrorView error={grants.error} />}

        {grants.data && (
          <div className="flex flex-col gap-1.5">
            {CAPABILITIES.map((cap) => {
              const on = held.has(cap)
              return (
                <label
                  key={cap}
                  className="flex cursor-pointer items-center justify-between rounded-lg border border-border bg-card/40 px-3 py-2 text-sm"
                >
                  <span className="mono text-xs">{cap}</span>
                  <Checkbox
                    checked={on}
                    disabled={grant.isPending || revoke.isPending}
                    onCheckedChange={(c) => toggle(cap, c === true)}
                  />
                </label>
              )
            })}
          </div>
        )}

        <DialogFooter>
          <Button onClick={() => onOpenChange(false)}>Done</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

// — Teams ————————————————————————————————————————————————————————————————————

function TeamsTab({ tenant }: { tenant: string }) {
  const teams = useTeams(tenant)
  const { remove } = useTeamMutations(tenant)
  const { toast } = useToast()

  const [createOpen, setCreateOpen] = useState(false)
  const [selected, setSelected] = useState<Team | null>(null)

  async function handleDelete(t: Team) {
    if (!window.confirm(`Delete team ${t.display_name}? Members keep their accounts.`)) return
    try {
      await remove.mutateAsync(t.slug)
      toast('Team deleted')
    } catch (e) {
      toast(e instanceof Error ? e.message : 'Delete failed', 'error')
    }
  }

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center justify-between gap-3">
        <p className="text-[13px] text-muted-foreground">
          Group people and grant access once — every member inherits it.
        </p>
        <Button onClick={() => setCreateOpen(true)} className="gap-1.5">
          <Plus size={16} /> New team
        </Button>
      </div>

      {teams.isLoading && <LoadingView label="Loading teams…" />}
      {teams.error && <ErrorView error={teams.error} />}
      {teams.data && teams.data.length === 0 && (
        <EmptyView title="No teams yet" hint="Create a team, add members, then grant it access." />
      )}

      {teams.data && teams.data.length > 0 && (
        <div className="flex flex-col gap-3">
          {teams.data.map((t) => (
            <TeamCard
              key={t.slug}
              tenant={tenant}
              team={t}
              onManage={() => setSelected(t)}
              onDelete={() => handleDelete(t)}
            />
          ))}
        </div>
      )}

      <CreateTeamDialog tenant={tenant} open={createOpen} onOpenChange={setCreateOpen} />
      {selected && (
        <ManageTeamDialog
          tenant={tenant}
          team={selected}
          open={selected !== null}
          onOpenChange={(o) => !o && setSelected(null)}
        />
      )}
    </div>
  )
}

// A team's at-a-glance card: who's in it and what they inherit. Reads the same
// per-team member/grant queries the manage dialog uses, so opening Manage is warm.
function TeamCard({
  tenant,
  team,
  onManage,
  onDelete,
}: {
  tenant: string
  team: Team
  onManage: () => void
  onDelete: () => void
}) {
  const api = useApi(tenant)
  const members = useQuery({
    queryKey: ['team-members', tenant, team.slug],
    queryFn: () => listTeamMembers(api, team.slug),
  })
  const grants = useQuery({
    queryKey: ['team-grants', tenant, team.slug],
    queryFn: () => listTeamGrants(api, team.slug),
  })

  return (
    <div className="rounded-xl border border-border bg-card/40 p-4">
      <div className="flex items-start justify-between gap-3">
        <div className="flex items-center gap-3">
          <div className="grid size-9 place-items-center rounded-lg border border-border bg-card">
            <Users size={16} className="text-muted-foreground" />
          </div>
          <div>
            <div className="font-medium">{team.display_name}</div>
            <div className="mono text-xs text-muted-foreground">{team.slug}</div>
          </div>
        </div>
        <div className="flex items-center gap-1.5">
          <Button variant="outline" size="sm" onClick={onManage}>
            Manage
          </Button>
          <Button variant="ghost" size="icon" onClick={onDelete} aria-label="Delete team">
            <Trash2 size={14} className="text-destructive" />
          </Button>
        </div>
      </div>

      <div className="mt-4 grid gap-4 sm:grid-cols-2">
        {/* Members preview */}
        <div className="flex flex-col gap-1.5">
          <div className="text-xs font-medium text-muted-foreground">
            Members{members.data ? ` · ${members.data.length}` : ''}
          </div>
          {members.isLoading && <span className="text-xs text-muted-foreground">Loading…</span>}
          {members.error && <span className="text-xs text-destructive">Failed to load</span>}
          {members.data && members.data.length === 0 && (
            <span className="text-xs text-muted-foreground">No members yet.</span>
          )}
          {members.data && members.data.length > 0 && (
            <div className="flex flex-wrap gap-1.5">
              {members.data.map((m) => (
                <Badge key={m.subject} variant="secondary" className="mono text-[10px]">
                  {m.subject}
                </Badge>
              ))}
            </div>
          )}
        </div>

        {/* Access preview */}
        <div className="flex flex-col gap-1.5">
          <div className="text-xs font-medium text-muted-foreground">
            Inherited access{grants.data ? ` · ${grants.data.length}` : ''}
          </div>
          {grants.isLoading && <span className="text-xs text-muted-foreground">Loading…</span>}
          {grants.error && <span className="text-xs text-destructive">Failed to load</span>}
          {grants.data && grants.data.length === 0 && (
            <span className="text-xs text-muted-foreground">No access granted.</span>
          )}
          {grants.data && grants.data.length > 0 && (
            <div className="flex flex-wrap gap-1.5">
              {grants.data.map((g) => (
                <Badge key={g.capability} variant="outline" className="mono text-[10px]">
                  {g.capability}
                </Badge>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  )
}

// Slugify a display name into a URL-safe key the way the backend expects: lower,
// spaces/punctuation → single hyphens, trimmed. So "Field Engineers" → "field-engineers".
function slugify(input: string): string {
  return input
    .toLowerCase()
    .trim()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
}

function CreateTeamDialog({
  tenant,
  open,
  onOpenChange,
}: {
  tenant: string
  open: boolean
  onOpenChange: (open: boolean) => void
}) {
  const { create } = useTeamMutations(tenant)
  const { toast } = useToast()
  const [name, setName] = useState('')
  // The slug auto-derives from the name but stays editable once the user touches it.
  const [slug, setSlug] = useState('')
  const [slugEdited, setSlugEdited] = useState(false)
  const effectiveSlug = slugEdited ? slug : slugify(name)

  function reset() {
    setName('')
    setSlug('')
    setSlugEdited(false)
  }

  async function handleCreate() {
    try {
      await create.mutateAsync({ slug: effectiveSlug, display_name: name.trim() || effectiveSlug })
      toast('Team created')
      reset()
      onOpenChange(false)
    } catch (e) {
      toast(e instanceof Error ? e.message : 'Create failed', 'error')
    }
  }

  return (
    <Dialog
      open={open}
      onOpenChange={(o) => {
        if (!o) reset()
        onOpenChange(o)
      }}
    >
      <DialogContent>
        <DialogHeader>
          <DialogTitle>New team</DialogTitle>
          <DialogDescription>
            A team groups people. Add members and grant it access after creating.
          </DialogDescription>
        </DialogHeader>

        <div className="flex flex-col gap-4">
          <div className="flex flex-col gap-2">
            <Label htmlFor="t-name">Name</Label>
            <Input
              id="t-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Field Engineers"
              autoComplete="off"
            />
          </div>
          <div className="flex flex-col gap-2">
            <Label htmlFor="t-slug">Slug</Label>
            <Input
              id="t-slug"
              value={effectiveSlug}
              onChange={(e) => {
                setSlugEdited(true)
                setSlug(slugify(e.target.value))
              }}
              placeholder="field-engineers"
              autoComplete="off"
              className="mono text-xs"
            />
            <p className="text-xs text-muted-foreground">
              The stable key used in grants. Lowercase, hyphenated.
            </p>
          </div>
        </div>

        <DialogFooter>
          <Button variant="ghost" onClick={() => onOpenChange(false)} disabled={create.isPending}>
            Cancel
          </Button>
          <Button onClick={handleCreate} disabled={create.isPending || !effectiveSlug}>
            {create.isPending ? 'Creating…' : 'Create'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

function ManageTeamDialog({
  tenant,
  team,
  open,
  onOpenChange,
}: {
  tenant: string
  team: Team
  open: boolean
  onOpenChange: (open: boolean) => void
}) {
  const api = useApi(tenant)
  const principals = usePrincipals(tenant)
  const members = useQuery({
    queryKey: ['team-members', tenant, team.slug],
    queryFn: () => listTeamMembers(api, team.slug),
  })
  const grants = useQuery({
    queryKey: ['team-grants', tenant, team.slug],
    queryFn: () => listTeamGrants(api, team.slug),
  })
  const { addMember, removeMember, grant, revoke } = useTeamDetailMutations(tenant, team.slug)
  const { toast } = useToast()

  const [toAdd, setToAdd] = useState('')

  const memberSet = new Set((members.data ?? []).map((m) => m.subject))
  // Only user principals not already on the team can be added.
  const addable = (principals.data ?? []).filter(
    (p) => p.kind === 'user' && !memberSet.has(p.subject),
  )
  const heldCaps = new Set((grants.data ?? []).map((g) => g.capability))

  async function handleAdd() {
    if (!toAdd) return
    try {
      await addMember.mutateAsync(toAdd)
      toast('Member added')
      setToAdd('')
    } catch (e) {
      toast(e instanceof Error ? e.message : 'Add failed', 'error')
    }
  }

  async function handleRemove(subject: string) {
    try {
      await removeMember.mutateAsync(subject)
      toast('Member removed')
    } catch (e) {
      toast(e instanceof Error ? e.message : 'Remove failed', 'error')
    }
  }

  async function toggleCap(capability: string, on: boolean) {
    try {
      if (on) await grant.mutateAsync(capability)
      else await revoke.mutateAsync(capability)
      toast(on ? `Granted ${capability}` : `Revoked ${capability}`)
    } catch (e) {
      toast(e instanceof Error ? e.message : 'Grant change failed', 'error')
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-[640px]">
        <DialogHeader>
          <DialogTitle>{team.display_name}</DialogTitle>
          <DialogDescription>
            Members and the capabilities every member inherits.
          </DialogDescription>
        </DialogHeader>

        <div className="flex flex-col gap-6">
          {/* Members */}
          <section className="flex flex-col gap-3">
            <h3 className="text-sm font-semibold">Members</h3>

            <div className="flex items-center gap-2">
              <Select value={toAdd} onValueChange={setToAdd}>
                <SelectTrigger className="h-9 flex-1">
                  <SelectValue placeholder={addable.length ? 'Add a person…' : 'No one to add'} />
                </SelectTrigger>
                <SelectContent>
                  {addable.map((p) => (
                    <SelectItem key={p.subject} value={p.subject}>
                      {p.subject}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <Button
                size="sm"
                className="gap-1.5"
                onClick={handleAdd}
                disabled={!toAdd || addMember.isPending}
              >
                <UserPlus size={14} /> Add
              </Button>
            </div>

            {members.isLoading && <p className="text-sm text-muted-foreground">Loading members…</p>}
            {members.error && <ErrorView error={members.error} />}
            {members.data && members.data.length === 0 && (
              <p className="rounded-lg border border-dashed border-border px-3 py-2 text-sm text-muted-foreground">
                No members yet.
              </p>
            )}
            {members.data && members.data.length > 0 && (
              <div className="flex flex-wrap gap-1.5">
                {members.data.map((m) => (
                  <Badge key={m.subject} variant="secondary" className="gap-1 py-1 pl-2.5 pr-1">
                    <span className="mono text-xs">{m.subject}</span>
                    <button
                      type="button"
                      aria-label={`Remove ${m.subject}`}
                      className="grid size-4 place-items-center rounded hover:bg-muted-foreground/20"
                      onClick={() => handleRemove(m.subject)}
                      disabled={removeMember.isPending}
                    >
                      <X size={11} />
                    </button>
                  </Badge>
                ))}
              </div>
            )}
          </section>

          {/* Capabilities */}
          <section className="flex flex-col gap-3">
            <h3 className="text-sm font-semibold">Team access</h3>
            <p className="text-xs text-muted-foreground">
              Capabilities granted here are inherited by every member.
            </p>
            {grants.isLoading && <p className="text-sm text-muted-foreground">Loading grants…</p>}
            {grants.error && <ErrorView error={grants.error} />}
            {grants.data && (
              <div className="flex flex-col gap-1.5">
                {CAPABILITIES.map((cap) => {
                  const on = heldCaps.has(cap)
                  return (
                    <label
                      key={cap}
                      className="flex cursor-pointer items-center justify-between rounded-lg border border-border bg-card/40 px-3 py-2 text-sm"
                    >
                      <span className="mono text-xs">{cap}</span>
                      <Checkbox
                        checked={on}
                        disabled={grant.isPending || revoke.isPending}
                        onCheckedChange={(c) => toggleCap(cap, c === true)}
                      />
                    </label>
                  )
                })}
              </div>
            )}
          </section>
        </div>

        <DialogFooter>
          <Button onClick={() => onOpenChange(false)}>Done</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
