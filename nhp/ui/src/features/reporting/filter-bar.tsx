/**
 * The shared scope filter for the reporting + alarms surfaces: tenant → site →
 * meter-type → quantity, cascading. Picking a tenant narrows the site list (and
 * clears a now-out-of-scope site); changing any scope clears a quantity that is no
 * longer present. A controlled component — it owns no state, only renders the
 * current `ScopeFilter` and emits the next one — so a page can drive it from URL
 * search params or local state.
 *
 * "All" is modelled as the `ALL` sentinel because a Radix Select item cannot carry
 * an empty value; the sentinel maps to `undefined` on the way out.
 */
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import {
  quantitiesInScope,
  sitesForTenant,
  type PortfolioIndex,
  type ScopeFilter,
} from './scope'

const ALL = '__all__'
const fromSentinel = (v: string) => (v === ALL ? undefined : v)
const toSentinel = (v: string | undefined) => v ?? ALL

export function FilterBar({
  index,
  filter,
  onChange,
  showQuantity = true,
}: {
  index: PortfolioIndex
  filter: ScopeFilter
  onChange: (next: ScopeFilter) => void
  showQuantity?: boolean
}) {
  const tenants = index.data.tenants
  const sites = sitesForTenant(index, filter.tenantId)
  const meterTypes = index.data.meterTypes
  const quantities = quantitiesInScope(index, filter)

  const setTenant = (v: string) => {
    const tenantId = fromSentinel(v)
    // A site that no longer belongs to the chosen tenant is cleared.
    const siteStillValid =
      filter.siteId &&
      sitesForTenant(index, tenantId).some((s) => s.id === filter.siteId)
    onChange({
      ...filter,
      tenantId,
      siteId: siteStillValid ? filter.siteId : undefined,
      quantity: undefined,
    })
  }
  const setSite = (v: string) =>
    onChange({ ...filter, siteId: fromSentinel(v), quantity: undefined })
  const setType = (v: string) =>
    onChange({ ...filter, meterTypeId: fromSentinel(v), quantity: undefined })
  const setQuantity = (v: string) =>
    onChange({ ...filter, quantity: fromSentinel(v) })

  return (
    <div className='grid gap-3 sm:grid-cols-2 lg:grid-cols-4'>
      <Field label='Tenant'>
        <Select value={toSentinel(filter.tenantId)} onValueChange={setTenant}>
          <SelectTrigger>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value={ALL}>All tenants</SelectItem>
            {tenants.map((t) => (
              <SelectItem key={t.id} value={t.id}>
                {t.content.name}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </Field>

      <Field label='Site'>
        <Select value={toSentinel(filter.siteId)} onValueChange={setSite}>
          <SelectTrigger>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value={ALL}>
              {filter.tenantId ? 'All sites (tenant)' : 'All sites'}
            </SelectItem>
            {sites.map((s) => (
              <SelectItem key={s.id} value={s.id}>
                {s.content.name}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </Field>

      <Field label='Meter-type'>
        <Select value={toSentinel(filter.meterTypeId)} onValueChange={setType}>
          <SelectTrigger>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value={ALL}>All meter-types</SelectItem>
            {meterTypes.map((t) => (
              <SelectItem key={t.id} value={t.id}>
                {t.content.name}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </Field>

      {showQuantity ? (
        <Field label='Quantity'>
          <Select value={toSentinel(filter.quantity)} onValueChange={setQuantity}>
            <SelectTrigger>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value={ALL}>All quantities</SelectItem>
              {quantities.map((q) => (
                <SelectItem key={q} value={q}>
                  {q}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </Field>
      ) : null}
    </div>
  )
}

function Field({
  label,
  children,
}: {
  label: string
  children: React.ReactNode
}) {
  return (
    <div className='grid gap-1'>
      <Label className='text-xs'>{label}</Label>
      {children}
    </div>
  )
}
