import { cn } from '@/lib/utils'

export function Logo({
  className,
  ...props
}: React.ImgHTMLAttributes<HTMLImageElement>) {
  return (
    <img
      src='/images/nhp-logo.png'
      alt='NHP'
      className={cn('size-6 rounded-md object-contain', className)}
      {...props}
    />
  )
}
