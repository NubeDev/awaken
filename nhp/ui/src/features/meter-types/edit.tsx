/**
 * Meter-type editor (ADMIN.md §1): name/manufacturer + the register-map table.
 * Saving an existing type BUMPS its `version` (DOMAIN-MODEL §versioning) and does
 * not touch deployed meters; creating writes a new `kind:"meter-type"` record at
 * version 1. Both cross the gate via the records API. Used for create, edit, and
 * clone (clone seeds the form from a source type with a fresh key + version 1).
 */
import { useState } from 'react'
import { ArrowLeft } from 'lucide-react'
import type { MeterType, MeterTypeRecord, RegisterDef } from '@/api/records'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { RegisterTable } from './register-table'
import { useCreateMeterType, useUpdateMeterType } from './hooks'

type EditMode =
  | { mode: 'create' }
  | { mode: 'edit'; record: MeterTypeRecord }
  | { mode: 'clone'; source: MeterTypeRecord }

type MeterTypeEditorProps = {
  state: EditMode
  onDone: () => void
}

function initialContent(state: EditMode): MeterType {
  if (state.mode === 'edit') return state.record.content
  if (state.mode === 'clone') {
    return {
      ...state.source.content,
      key: `${state.source.content.key}-copy`,
      name: `${state.source.content.name} (copy)`,
      version: 1,
    }
  }
  return {
    kind: 'meter-type',
    key: '',
    name: '',
    manufacturer: '',
    version: 1,
    registers: [],
  }
}

export function MeterTypeEditor({ state, onDone }: MeterTypeEditorProps) {
  const start = initialContent(state)
  const [key, setKey] = useState(start.key)
  const [name, setName] = useState(start.name)
  const [manufacturer, setManufacturer] = useState(start.manufacturer ?? '')
  const [registers, setRegisters] = useState<RegisterDef[]>(start.registers)

  const create = useCreateMeterType()
  const update = useUpdateMeterType()
  const pending = create.isPending || update.isPending

  const titleFor = {
    create: 'New meter-type',
    edit: 'Edit meter-type',
    clone: 'Clone meter-type',
  }[state.mode]

  const save = () => {
    if (state.mode === 'edit') {
      const content: MeterType = {
        ...state.record.content,
        key,
        name,
        manufacturer,
        registers,
        // editing bumps the version (DOMAIN-MODEL §versioning)
        version: state.record.content.version + 1,
      }
      update.mutate({ id: state.record.id, content }, { onSuccess: onDone })
      return
    }
    const content: MeterType = {
      kind: 'meter-type',
      key,
      name,
      manufacturer,
      version: 1,
      registers,
    }
    create.mutate(content, { onSuccess: onDone })
  }

  const valid = key.trim() !== '' && name.trim() !== ''

  return (
    <div className='space-y-6'>
      <div className='flex items-center gap-3'>
        <Button variant='ghost' size='icon' onClick={onDone}>
          <ArrowLeft className='size-4' />
        </Button>
        <h2 className='text-xl font-semibold'>{titleFor}</h2>
        {state.mode === 'edit' ? (
          <span className='text-muted-foreground text-sm'>
            current version {state.record.content.version} → saves as{' '}
            {state.record.content.version + 1}
          </span>
        ) : null}
      </div>

      <div className='grid max-w-2xl gap-4 sm:grid-cols-3'>
        <div className='grid gap-1'>
          <Label htmlFor='mt-key'>Key</Label>
          <Input
            id='mt-key'
            value={key}
            onChange={(e) => setKey(e.target.value)}
            placeholder='acme-pm5560'
          />
        </div>
        <div className='grid gap-1'>
          <Label htmlFor='mt-name'>Name</Label>
          <Input
            id='mt-name'
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder='Acme PM5560'
          />
        </div>
        <div className='grid gap-1'>
          <Label htmlFor='mt-mfr'>Manufacturer</Label>
          <Input
            id='mt-mfr'
            value={manufacturer}
            onChange={(e) => setManufacturer(e.target.value)}
            placeholder='Acme'
          />
        </div>
      </div>

      <div>
        <h3 className='mb-2 text-base font-medium'>Register map</h3>
        <RegisterTable registers={registers} onChange={setRegisters} />
      </div>

      <div className='flex gap-2'>
        <Button onClick={save} disabled={!valid || pending}>
          {pending ? 'Saving…' : 'Save'}
        </Button>
        <Button variant='ghost' onClick={onDone}>
          Cancel
        </Button>
      </div>
    </div>
  )
}

export type { EditMode }
