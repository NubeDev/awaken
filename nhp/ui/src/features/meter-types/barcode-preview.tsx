/**
 * A live QR preview of a barcode string (WS-09 Part A). Renders the given code as a
 * QR to a canvas via the `qrcode` lib and re-renders whenever the code changes — so
 * the meter-type editor shows the scannable graphic as you type the key, before
 * saving. Pure presentational: takes the code string, draws it. The printable
 * dialog (barcode-label.tsx) reuses this; it adds the print/download chrome.
 */
import { useEffect, useRef } from 'react'
import QRCode from 'qrcode'

interface BarcodePreviewProps {
  /** The exact string to encode (enums/barcode.ts `nhp-mt:<key>`). */
  code: string
  /** Canvas pixel size. */
  size?: number
}

export function BarcodePreview({ code, size = 120 }: BarcodePreviewProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null)

  useEffect(() => {
    if (!canvasRef.current || !code) return
    QRCode.toCanvas(canvasRef.current, code, { width: size, margin: 2 }).catch(
      () => undefined
    )
  }, [code, size])

  return <canvas ref={canvasRef} className='rounded border bg-white' />
}
