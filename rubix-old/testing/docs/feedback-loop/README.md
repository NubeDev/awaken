# Feedback Loop — Capture → Triage → Fix

When a ✅ check fails, you run a fixed loop instead of guessing:

1. [CAPTURE.md](CAPTURE.md) — produce the standard evidence bundle **before**
   theorizing. A consistent bundle makes triage mechanical and lets a fresh AI
   session reason without re-running.
2. [TRIAGE.md](TRIAGE.md) — match the symptom to a likely cause, run the
   confirming check (the obvious cause is often not the real one).
3. [FIX_LOOP.md](FIX_LOOP.md) — decide bug vs doc-bug vs expected, make the
   smallest change, and **prove** it (tests green + the failing ✅ re-passes +
   before/after bundle).

The loop is the same discipline as the `docs/sessions/WS-xx` scope work: evidence
first, smallest correct change, prove it, record it as institutional memory.

Evidence lands in `testing/.evidence/<scenario>/<timestamp>/` (git-ignored, one
dir per failure so before/after are comparable).
