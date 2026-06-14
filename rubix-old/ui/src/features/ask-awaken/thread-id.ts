const STORAGE_KEY = 'rubix.ask-awaken.thread'

/**
 * The persistent agent thread the Ask-awaken sheet posts on. One thread per
 * browser keeps the operator's ad-hoc conversation coherent across sessions;
 * the id is opaque to the server (any stable string addresses a thread).
 */
export function askThreadId(): string {
  try {
    const existing = localStorage.getItem(STORAGE_KEY)
    if (existing) return existing
    const fresh = `ask-${crypto.randomUUID()}`
    localStorage.setItem(STORAGE_KEY, fresh)
    return fresh
  } catch {
    // Private-mode / disabled storage: fall back to a per-load thread so chat
    // still works; it simply will not persist across reloads.
    return `ask-${crypto.randomUUID()}`
  }
}
