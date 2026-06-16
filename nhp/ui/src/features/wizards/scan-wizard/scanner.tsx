/**
 * Camera barcode scanner with a manual-entry fallback (WS-09 Part B). Uses
 * @zxing/browser to decode a QR from the device camera; calls `onCode` with the
 * raw decoded string (the wizard resolves it via enums/barcode.ts). The REAL cases
 * are handled honestly (WS-09.md §Part B):
 *   - camera permission denied / no camera → the camera path reports the error and
 *     the manual text entry stays available (a laptop with no camera still works).
 *   - the scanner runs continuously until it reads a code, then stops the stream.
 *
 * This file owns ONLY the input mechanics (camera + manual). Resolving the code to a
 * meter-type and the unknown-barcode error live in the wizard (scan-wizard.tsx).
 */
import { useEffect, useRef, useState } from 'react'
import { BrowserQRCodeReader, type IScannerControls } from '@zxing/browser'
import { Camera, CameraOff, Keyboard } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'

interface ScannerProps {
  /** Fired with the raw decoded/typed code. */
  onCode: (code: string) => void
}

export function Scanner({ onCode }: ScannerProps) {
  const videoRef = useRef<HTMLVideoElement>(null)
  const controlsRef = useRef<IScannerControls | null>(null)
  const [scanning, setScanning] = useState(false)
  const [cameraError, setCameraError] = useState<string | null>(null)
  const [manual, setManual] = useState('')

  // Always stop the camera stream on unmount (frees the device + privacy light).
  useEffect(() => () => controlsRef.current?.stop(), [])

  const startCamera = async () => {
    setCameraError(null)
    setScanning(true)
    try {
      const reader = new BrowserQRCodeReader()
      controlsRef.current = await reader.decodeFromVideoDevice(
        undefined, // default camera
        videoRef.current ?? undefined,
        (result, _err, controls) => {
          // _err fires once per frame with no code — only act on a real result.
          if (result) {
            controls.stop()
            controlsRef.current = null
            setScanning(false)
            onCode(result.getText())
          }
        }
      )
    } catch (e) {
      // Permission denied, no camera, or insecure context — fall back to manual.
      controlsRef.current?.stop()
      controlsRef.current = null
      setScanning(false)
      setCameraError(
        e instanceof Error
          ? `Camera unavailable (${e.name || e.message}). Type the code below instead.`
          : 'Camera unavailable. Type the code below instead.'
      )
    }
  }

  const stopCamera = () => {
    controlsRef.current?.stop()
    controlsRef.current = null
    setScanning(false)
  }

  return (
    <div className='grid gap-4'>
      <div className='grid gap-2'>
        <div className='flex items-center justify-between'>
          <Label>Scan with camera</Label>
          {scanning ? (
            <Button variant='ghost' size='sm' onClick={stopCamera}>
              <CameraOff className='mr-1 size-4' /> Stop
            </Button>
          ) : (
            <Button variant='outline' size='sm' onClick={startCamera}>
              <Camera className='mr-1 size-4' /> Start camera
            </Button>
          )}
        </div>
        {/* The video is only meaningful while scanning; keep it mounted so the ref
            is available the instant decodeFromVideoDevice attaches the stream. */}
        <video
          ref={videoRef}
          className={
            scanning
              ? 'aspect-video w-full rounded-md border bg-black'
              : 'hidden'
          }
          muted
          playsInline
        />
        {cameraError ? (
          <p className='text-destructive text-sm'>{cameraError}</p>
        ) : null}
      </div>

      <div className='grid gap-1'>
        <Label htmlFor='scan-manual' className='flex items-center gap-1'>
          <Keyboard className='size-4' /> …or type the code
        </Label>
        <div className='flex gap-2'>
          <Input
            id='scan-manual'
            value={manual}
            onChange={(e) => setManual(e.target.value)}
            placeholder='nhp-mt:acme-pm5560'
            className='font-mono text-sm'
            onKeyDown={(e) => {
              if (e.key === 'Enter' && manual.trim()) onCode(manual.trim())
            }}
          />
          <Button
            variant='secondary'
            disabled={!manual.trim()}
            onClick={() => onCode(manual.trim())}
          >
            Use code
          </Button>
        </div>
      </div>
    </div>
  )
}
