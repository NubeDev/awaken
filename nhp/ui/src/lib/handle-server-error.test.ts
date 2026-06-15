import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ApiError } from '@/api/client'
import { handleServerError } from './handle-server-error'

const toastError = vi.hoisted(() => vi.fn())

vi.mock('sonner', () => ({
  toast: {
    error: toastError,
  },
}))

beforeEach(() => {
  vi.mocked(toastError).mockClear()
})

describe('handleServerError', () => {
  it('shows a generic message when the error is not recognised', () => {
    handleServerError(new Error('network'))

    expect(toastError).toHaveBeenCalledWith('Something went wrong!')
  })

  it('maps an ApiError with status 204 to the no-content message', () => {
    handleServerError(new ApiError(204, ''))

    expect(toastError).toHaveBeenCalledWith('No content.')
  })

  it('prefers the server error message carried by ApiError', () => {
    handleServerError(new ApiError(422, 'Validation failed'))

    expect(toastError).toHaveBeenCalledWith('Validation failed')
  })

  it('falls back to the generic message when ApiError has no message', () => {
    handleServerError(new ApiError(500, ''))

    expect(toastError).toHaveBeenCalledWith('Something went wrong!')
  })

  it('logs the error to the console in development', () => {
    const log = vi.spyOn(console, 'log').mockImplementation(() => {})
    const err = new Error('logged')

    handleServerError(err)

    expect(log).toHaveBeenCalledTimes(1)
    expect(log).toHaveBeenCalledWith(err)

    log.mockRestore()
  })

  it('does not log the error to the console in production', () => {
    vi.stubEnv('DEV', false)

    const log = vi.spyOn(console, 'log').mockImplementation(() => {})
    const err = new Error('not logged')

    handleServerError(err)

    expect(log).not.toHaveBeenCalled()

    log.mockRestore()
  })
})
