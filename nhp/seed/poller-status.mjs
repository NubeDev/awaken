// The faked poller status for gateways and meters — the other half of NHP standing
// in for the (not-built) polling service (DOMAIN-MODEL §"Status fields are
// poller-owned"). NHP never produces these in production; the seed fabricates a
// plausible snapshot so dashboards can render online/offline rollups and
// last_seen.
//
// Deterministic, not random: a stable index decides who is offline so a re-seed is
// reproducible. Most devices are online; roughly one in seven is offline (so a
// site-level "degraded if any gateway offline" rollup has something to show), and
// `last_seen` is recent for online devices, stale for offline ones.

const OFFLINE_EVERY = 7;

// Decide a device's status from its ordinal position. Keeps the online/offline mix
// stable across runs and spreads the offline devices out.
export function statusFor(index) {
  return index % OFFLINE_EVERY === OFFLINE_EVERY - 1 ? 'offline' : 'online';
}

// last_seen for a device: now for an online device, a few hours ago for an offline
// one (it stopped reporting). RFC 3339 string — DOMAIN-MODEL `last_seen` is a date.
export function lastSeenFor(status, now = new Date()) {
  if (status === 'offline') {
    return new Date(now.getTime() - 4 * 3600_000).toISOString();
  }
  return now.toISOString();
}

// The poller-owned content patch for a device (gateway or meter) at ordinal
// `index`. Spread onto the record content at create time so the record carries a
// realistic status without a second write (the poller would PATCH it live; the
// seed bakes it in).
export function pollerFields(index, now = new Date()) {
  const status = statusFor(index);
  return { status, last_seen: lastSeenFor(status, now) };
}
