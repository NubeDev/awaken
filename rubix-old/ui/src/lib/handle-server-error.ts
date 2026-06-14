import { toast } from 'sonner'
import { ApiError } from '@/api/client'

/**
 * Surface a failed request as a toast. The rubix-server fetch client raises
 * `ApiError` carrying the server's `ErrorBody` message, so we prefer that text.
 */
export function handleServerError(error: unknown) {
  if (import.meta.env.DEV) {
    // eslint-disable-next-line no-console
    console.log(error)
  }

  let errMsg = 'Something went wrong!'

  if (error instanceof ApiError) {
    if (error.status === 204) errMsg = 'No content.'
    else if (error.message) errMsg = error.message
  }

  toast.error(errMsg)
}
