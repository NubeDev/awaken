import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import type { BoardView } from '@/api/types'

type BoardPickerProps = {
  boards: BoardView[]
  selectedSlug: string | undefined
  onSelect: (slug: string) => void
}

/** Select listing stored boards by display name; selection loads its graph. */
export function BoardPicker({ boards, selectedSlug, onSelect }: BoardPickerProps) {
  return (
    <Select value={selectedSlug} onValueChange={onSelect}>
      <SelectTrigger size='sm' className='w-[220px]'>
        <SelectValue placeholder='Select a board' />
      </SelectTrigger>
      <SelectContent>
        {boards.map((b) => (
          <SelectItem key={b.slug} value={b.slug}>
            {b.display_name}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  )
}
