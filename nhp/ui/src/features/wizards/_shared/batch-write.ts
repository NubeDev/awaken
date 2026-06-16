/**
 * The batched, resumable record writer every wizard ends with (WIZARDS.md
 * §Principles: "Batched, atomic-ish, resumable. … a failed step reports per-record
 * and the wizard can resume rather than restart").
 *
 * A wizard builds a list of `PlannedRecord`s (each a `content` object for the
 * generic records API, plus a label + a `parentRef` for records whose parent id is
 * only known once an earlier record is written — e.g. a meter's `network`, or a
 * register's `meter`). `runBatch` writes them IN ORDER, resolving parentRefs from
 * the ids of already-written records, and returns a per-record `BatchResult`. On a
 * partial failure the caller re-runs with the SAME plan and the SAME `results` —
 * already-`ok` records are skipped (resume, not restart). Each write crosses the
 * gate (audited) via api/records.createRecord; nothing here is faked.
 */
import { createRecord, type RecordDto } from '@/api/records'

/** Marks a content field whose value is the record id of an earlier planned record. */
export interface ParentRef {
  /** The content field to fill (e.g. "network", "meter", "site"). */
  field: string
  /** The `id` of the PlannedRecord whose written record id goes in `field`. */
  planId: string
}

/** One record a wizard intends to create. `content` is the records-API payload. */
export interface PlannedRecord {
  /** Stable id WITHIN the plan (not the rubix record id) — for resume + parentRefs. */
  id: string
  /** Human label shown in the preview + result table (e.g. "network gw-01-net-3"). */
  label: string
  /** The record kind, for the preview grouping. */
  kind: string
  /** The records-API content payload (already carries `kind` and `tags`). */
  content: Record<string, unknown>
  /**
   * Fields whose value is another planned record's WRITTEN id, resolved at run
   * time (a child planned before its parent's id exists). Empty for roots.
   */
  parentRefs?: ParentRef[]
}

export type BatchStatus = 'pending' | 'ok' | 'error'

/** The outcome of one planned record, keyed by `PlannedRecord.id`. */
export interface BatchResult {
  status: BatchStatus
  /** The written rubix record id (when ok) — children resolve parentRefs from this. */
  recordId?: string
  error?: string
}

export type BatchResults = Record<string, BatchResult>

/** A fresh all-pending result map for a plan. */
export function initialResults(plan: PlannedRecord[]): BatchResults {
  const out: BatchResults = {}
  for (const p of plan) out[p.id] = { status: 'pending' }
  return out
}

/**
 * Write the plan in order, skipping records already `ok` (resume). Returns the
 * updated results; `onProgress` fires after each write so the UI can render live.
 * Stops at the first error? No — it continues so the user sees ALL failures in one
 * pass, but a child whose parent failed is itself marked error (can't resolve the
 * parent id). Re-running retries every non-ok record.
 */
export async function runBatch(
  plan: PlannedRecord[],
  results: BatchResults,
  onProgress?: (results: BatchResults) => void
): Promise<BatchResults> {
  const next: BatchResults = { ...results }
  // Map planId → written record id, seeded from any prior successful run (resume).
  const writtenIds = new Map<string, string>()
  for (const p of plan) {
    const r = next[p.id]
    if (r?.status === 'ok' && r.recordId) writtenIds.set(p.id, r.recordId)
  }

  for (const p of plan) {
    if (next[p.id]?.status === 'ok') continue // resume: already written

    // Resolve parent ids from earlier writes; a missing parent ⇒ can't write.
    const content = { ...p.content }
    let blockedBy: string | undefined
    for (const ref of p.parentRefs ?? []) {
      const id = writtenIds.get(ref.planId)
      if (!id) {
        blockedBy = ref.planId
        break
      }
      content[ref.field] = id
    }
    if (blockedBy) {
      next[p.id] = {
        status: 'error',
        error: `parent ${blockedBy} not created`,
      }
      onProgress?.({ ...next })
      continue
    }

    try {
      const rec: RecordDto = await createRecord(content)
      writtenIds.set(p.id, rec.id)
      next[p.id] = { status: 'ok', recordId: rec.id }
    } catch (e) {
      next[p.id] = {
        status: 'error',
        error: e instanceof Error ? e.message : String(e),
      }
    }
    onProgress?.({ ...next })
  }
  return next
}

/** Tally for the result summary banner. */
export function summarise(plan: PlannedRecord[], results: BatchResults) {
  let ok = 0
  let error = 0
  for (const p of plan) {
    const s = results[p.id]?.status
    if (s === 'ok') ok += 1
    else if (s === 'error') error += 1
  }
  return { total: plan.length, ok, error, done: ok + error === plan.length }
}
