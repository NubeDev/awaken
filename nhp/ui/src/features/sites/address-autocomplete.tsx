/**
 * Address field with OpenStreetMap geocoding autocomplete. As the user types,
 * matches from Nominatim drop down (debounced + the previous request aborted);
 * picking one reports back the full address and its coordinates so the form can
 * fill `geo` and derive the timezone. Stays a plain editable text field — if
 * there's no internet the dropdown is just empty and the user types freely.
 */
import { useEffect, useRef, useState } from 'react'
import { Loader2, MapPin } from 'lucide-react'
import { cn } from '@/lib/utils'
import { Input } from '@/components/ui/input'
import {
  Popover,
  PopoverAnchor,
  PopoverContent,
} from '@/components/ui/popover'
import { geocodeAddress, type GeocodeResult } from './geo'

type AddressAutocompleteProps = {
  id?: string
  value: string
  onChange: (address: string) => void
  /** Fired when a suggestion is chosen — carries coordinates too. */
  onPick: (result: GeocodeResult) => void
  placeholder?: string
}

export function AddressAutocomplete({
  id,
  value,
  onChange,
  onPick,
  placeholder,
}: AddressAutocompleteProps) {
  const [results, setResults] = useState<GeocodeResult[]>([])
  const [open, setOpen] = useState(false)
  const [loading, setLoading] = useState(false)
  // True only while the user is actively typing — suppresses the search that a
  // programmatic onChange (e.g. selecting a result) would otherwise trigger.
  const typing = useRef(false)

  useEffect(() => {
    if (!typing.current) return
    const q = value.trim()
    const ctrl = new AbortController()
    // Debounce ~400ms to respect Nominatim's ~1 req/sec policy. All state updates
    // happen inside the async callback (never synchronously in the effect body).
    const t = setTimeout(async () => {
      if (q.length < 3) {
        setResults([])
        setOpen(false)
        return
      }
      setLoading(true)
      const found = await geocodeAddress(q, ctrl.signal)
      setResults(found)
      setOpen(found.length > 0)
      setLoading(false)
    }, 400)
    return () => {
      clearTimeout(t)
      ctrl.abort()
      setLoading(false)
    }
  }, [value])

  const choose = (r: GeocodeResult) => {
    typing.current = false
    onChange(r.label)
    onPick(r)
    setOpen(false)
    setResults([])
  }

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverAnchor asChild>
        <div className='relative'>
          <Input
            id={id}
            value={value}
            placeholder={placeholder}
            autoComplete='off'
            onChange={(e) => {
              typing.current = true
              onChange(e.target.value)
            }}
          />
          {loading ? (
            <Loader2 className='text-muted-foreground absolute end-2 top-1/2 size-4 -translate-y-1/2 animate-spin' />
          ) : null}
        </div>
      </PopoverAnchor>
      <PopoverContent
        align='start'
        className='w-[--radix-popover-trigger-width] p-1'
        onOpenAutoFocus={(e) => e.preventDefault()}
      >
        <ul className='max-h-60 overflow-auto'>
          {results.map((r, i) => (
            <li key={`${r.lat},${r.lng},${i}`}>
              <button
                type='button'
                onClick={() => choose(r)}
                className={cn(
                  'hover:bg-accent flex w-full items-start gap-2 rounded-sm px-2 py-1.5 text-left text-sm'
                )}
              >
                <MapPin className='text-muted-foreground mt-0.5 size-4 shrink-0' />
                <span className='leading-snug'>{r.label}</span>
              </button>
            </li>
          ))}
        </ul>
      </PopoverContent>
    </Popover>
  )
}
