/**
 * The printable meter-type barcode label (WS-09 Part A) — the graphic that goes
 * "on the box". Renders the meter-type's scan code (enums/barcode.ts) as a QR via
 * the `qrcode` lib (QR over 1D so phone cameras decode it reliably — WS-09.md
 * §Library choices), with print + PNG-download affordances.
 *
 * Shown in a dialog from the meter-type list. The QR encodes exactly what the scan
 * wizard decodes back to this meter-type — the round-trip the unit test guards.
 */
import { useEffect, useRef, useState } from 'react'
import QRCode from 'qrcode'
import { Download, Printer } from 'lucide-react'
import type { MeterTypeRecord } from '@/api/records'
import { barcodeFor } from '@/enums/barcode'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'

interface BarcodeLabelProps {
  type: MeterTypeRecord
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function BarcodeLabel({ type, open, onOpenChange }: BarcodeLabelProps) {
  const code = barcodeFor(type)
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const [dataUrl, setDataUrl] = useState('')

  // Render the QR once the dialog is open and the canvas mounted. `qrcode` draws
  // to the canvas and we also keep a PNG data-URL for download/print.
  useEffect(() => {
    if (!open || !canvasRef.current) return
    QRCode.toCanvas(canvasRef.current, code, { width: 240, margin: 2 }).catch(
      () => undefined
    )
    QRCode.toDataURL(code, { width: 480, margin: 2 })
      .then(setDataUrl)
      .catch(() => setDataUrl(''))
  }, [open, code])

  const download = () => {
    if (!dataUrl) return
    const a = document.createElement('a')
    a.href = dataUrl
    a.download = `${type.content.key}-barcode.png`
    a.click()
  }

  // Print just the label (code + QR) in a clean window — what a user sticks on the box.
  const print = () => {
    if (!dataUrl) return
    const w = window.open('', '_blank', 'width=420,height=520')
    if (!w) return
    w.document.write(
      `<html><head><title>${type.content.name} — barcode</title></head>` +
        `<body style="font-family:sans-serif;text-align:center;padding:24px">` +
        `<h3 style="margin:0 0 4px">${type.content.name}</h3>` +
        `<div style="font:12px monospace;color:#555;margin-bottom:12px">${code}</div>` +
        `<img src="${dataUrl}" width="360" height="360" alt="barcode" />` +
        `</body></html>`
    )
    w.document.close()
    w.focus()
    w.print()
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className='sm:max-w-sm'>
        <DialogHeader>
          <DialogTitle>{type.content.name} — scan label</DialogTitle>
          <DialogDescription>
            Print and stick this on the box. The scan-to-add-a-device wizard reads
            it to stamp a new meter from this type.
          </DialogDescription>
        </DialogHeader>
        <div className='flex flex-col items-center gap-3'>
          <canvas ref={canvasRef} className='rounded border' />
          <code className='text-muted-foreground text-xs'>{code}</code>
          <div className='flex gap-2'>
            <Button variant='outline' size='sm' onClick={print} disabled={!dataUrl}>
              <Printer className='mr-1 size-4' /> Print
            </Button>
            <Button
              variant='outline'
              size='sm'
              onClick={download}
              disabled={!dataUrl}
            >
              <Download className='mr-1 size-4' /> PNG
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  )
}
