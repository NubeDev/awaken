//! A per-principal scanned-context cache (§4a, the security-critical optimisation).
//!
//! Today every query rescans every canonical table into a `MemTable`
//! (`build_context`). That is the dominant cost across a board tick and across
//! different SQL hitting the same tables. This cache memoizes the **scanned
//! providers per principal** so a tick is a hit and different SQL on the same
//! tables reuses the scan.
//!
//! ⚠ **The cache key must include the scope identity.** `scan_table` reads through
//! the principal's gate-issued scoped session, so two principals running identical
//! SQL legitimately see different rows. A results-by-SQL cache would serve one
//! principal's rows to another — a permission bypass. So the key is the
//! **principal identity** plus the table-set, and each SQL runs **fresh** on top
//! of the cached scan (`rubix/docs/design/DASHBOARDS-SCOPE.md` §4a). The cache
//! holds **raw canonical values** — never unit-converted or formatted — so
//! conversion/formatting stays a post-cache per-caller layer (§2) and the unit
//! system never enters the key.
//!
//! Freshness is bounded two ways: a **TTL** tied to the minimum poll interval (so
//! a tick within the window is a guaranteed hit, and a stale scan ages out), and
//! **live invalidation** — a write on the data-change channel evicts the affected
//! namespace's entries (see [`ContextCache::invalidate_namespace`]) so a board
//! does not stay stale until the TTL despite fresh data. Size is bounded by an
//! **LRU** cap so the cache cannot grow unboundedly across principals.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use super::ScannedTable;

/// The default entry cap — bounds the cache across principals (§4a, "size cap").
const DEFAULT_CAPACITY: usize = 100;

/// The default TTL, aligned to the minimum board poll interval (5s, §6) so a tick
/// is a guaranteed hit and a scan never serves data older than one poll.
const DEFAULT_TTL: Duration = Duration::from_secs(5);

/// The identity a cached scan is keyed by — the principal, never just the SQL.
///
/// Two principals must never share a cached scan (the cross-principal leak §4a
/// flags), so the identity is the principal's stable subject plus its namespace.
/// The namespace is carried separately so a data-change on a namespace can evict
/// every principal scoped to it in one pass.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ScopeIdentity {
    /// The principal's namespace (tenant) — the invalidation unit.
    pub namespace: String,
    /// The principal's stable subject id.
    pub subject: String,
}

impl ScopeIdentity {
    /// Build a scope identity from a namespace and subject.
    #[must_use]
    pub fn new(namespace: impl Into<String>, subject: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
            subject: subject.into(),
        }
    }
}

/// One cached scan: the scanned providers plus the metadata the bound needs.
struct Entry {
    /// The scanned canonical-table providers (raw canonical values).
    tables: Vec<ScannedTable>,
    /// When the scan was taken — the TTL is measured from here.
    created: Instant,
    /// A monotonic stamp of the last access — the LRU eviction order.
    last_access: u64,
}

/// The cache's mutable interior, guarded by one mutex.
struct Inner {
    entries: HashMap<ScopeIdentity, Entry>,
    /// Monotonic access counter feeding `last_access` (independent of the clock so
    /// LRU ordering is stable even if two accesses share an instant).
    tick: u64,
}

/// A bounded, TTL'd, per-principal scanned-context cache.
///
/// Cloneable handles are not provided; share it behind an `Arc` (the transport
/// holds one in `AppState`). All methods take `&self` and lock internally, so it
/// is `Sync` and safe to share across request handlers.
pub struct ContextCache {
    inner: Mutex<Inner>,
    capacity: usize,
    ttl: Duration,
}

impl Default for ContextCache {
    fn default() -> Self {
        Self::new(DEFAULT_CAPACITY, DEFAULT_TTL)
    }
}

impl ContextCache {
    /// Build a cache bounded by `capacity` entries and a `ttl` per entry.
    #[must_use]
    pub fn new(capacity: usize, ttl: Duration) -> Self {
        Self {
            inner: Mutex::new(Inner {
                entries: HashMap::new(),
                tick: 0,
            }),
            capacity: capacity.max(1),
            ttl,
        }
    }

    /// Return the cached scan for `scope`, if present and not past its TTL.
    ///
    /// A hit clones the `Arc`-wrapped providers (cheap — pointer bumps, not data)
    /// and refreshes the entry's LRU position. A miss or an expired entry returns
    /// `None`; an expired entry is dropped on access so it cannot be served.
    #[must_use]
    pub fn get(&self, scope: &ScopeIdentity) -> Option<Vec<ScannedTable>> {
        let mut inner = self.inner.lock().expect("context cache mutex");
        let now = Instant::now();
        match inner.entries.get(scope) {
            Some(entry) if now.duration_since(entry.created) <= self.ttl => {
                let tables = entry.tables.clone();
                inner.tick += 1;
                let tick = inner.tick;
                if let Some(entry) = inner.entries.get_mut(scope) {
                    entry.last_access = tick;
                }
                Some(tables)
            }
            Some(_) => {
                inner.entries.remove(scope);
                None
            }
            None => None,
        }
    }

    /// Insert (or replace) the cached scan for `scope`, evicting the
    /// least-recently-used entry if the cache is at capacity.
    pub fn put(&self, scope: ScopeIdentity, tables: Vec<ScannedTable>) {
        let mut inner = self.inner.lock().expect("context cache mutex");
        inner.tick += 1;
        let tick = inner.tick;
        let created = Instant::now();
        if !inner.entries.contains_key(&scope) && inner.entries.len() >= self.capacity {
            evict_lru(&mut inner);
        }
        inner.entries.insert(
            scope,
            Entry {
                tables,
                created,
                last_access: tick,
            },
        );
    }

    /// Evict every cached scan for `namespace` — the live-invalidation hook.
    ///
    /// A write on the data-change channel calls this for the changed record's
    /// namespace, so every principal scoped to that tenant re-scans on its next
    /// query rather than serving stale rows until the TTL (§4a/§6). A record
    /// change can affect any principal's row-permitted view, so the whole
    /// namespace is evicted rather than guessing which principals saw the row.
    pub fn invalidate_namespace(&self, namespace: &str) {
        let mut inner = self.inner.lock().expect("context cache mutex");
        inner.entries.retain(|scope, _| scope.namespace != namespace);
    }

    /// The number of live entries — for tests and metrics.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.lock().expect("context cache mutex").entries.len()
    }

    /// Whether the cache holds no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Remove the least-recently-used entry from `inner`.
fn evict_lru(inner: &mut Inner) {
    if let Some(victim) = inner
        .entries
        .iter()
        .min_by_key(|(_, entry)| entry.last_access)
        .map(|(scope, _)| scope.clone())
    {
        inner.entries.remove(&victim);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use datafusion::arrow::datatypes::{DataType, Field, Schema};
    use datafusion::arrow::record_batch::RecordBatch;
    use datafusion::datasource::MemTable;

    use super::{ContextCache, ScannedTable, ScopeIdentity};

    fn a_table() -> Vec<ScannedTable> {
        let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Utf8, false)]));
        let batch = RecordBatch::new_empty(schema.clone());
        let provider = MemTable::try_new(schema, vec![vec![batch]]).unwrap();
        vec![("record", Arc::new(provider))]
    }

    fn scope(ns: &str, subject: &str) -> ScopeIdentity {
        ScopeIdentity::new(ns, subject)
    }

    #[test]
    fn a_put_is_a_subsequent_hit() {
        let cache = ContextCache::default();
        cache.put(scope("rubix", "alice"), a_table());
        assert!(cache.get(&scope("rubix", "alice")).is_some());
    }

    #[test]
    fn different_principals_do_not_share_an_entry() {
        let cache = ContextCache::default();
        cache.put(scope("rubix", "alice"), a_table());
        // Bob in the same namespace has his own (absent) entry — no cross hit.
        assert!(cache.get(&scope("rubix", "bob")).is_none());
    }

    #[test]
    fn an_expired_entry_is_a_miss() {
        let cache = ContextCache::new(10, Duration::from_millis(0));
        cache.put(scope("rubix", "alice"), a_table());
        std::thread::sleep(Duration::from_millis(1));
        assert!(cache.get(&scope("rubix", "alice")).is_none());
    }

    #[test]
    fn invalidating_a_namespace_evicts_its_principals_only() {
        let cache = ContextCache::default();
        cache.put(scope("rubix", "alice"), a_table());
        cache.put(scope("rubix", "bob"), a_table());
        cache.put(scope("other", "carol"), a_table());
        cache.invalidate_namespace("rubix");
        assert!(cache.get(&scope("rubix", "alice")).is_none());
        assert!(cache.get(&scope("rubix", "bob")).is_none());
        assert!(cache.get(&scope("other", "carol")).is_some());
    }

    #[test]
    fn the_lru_entry_is_evicted_at_capacity() {
        let cache = ContextCache::new(2, Duration::from_secs(60));
        cache.put(scope("rubix", "a"), a_table());
        cache.put(scope("rubix", "b"), a_table());
        // Touch `a` so `b` becomes the least-recently-used.
        assert!(cache.get(&scope("rubix", "a")).is_some());
        cache.put(scope("rubix", "c"), a_table());
        assert_eq!(cache.len(), 2);
        assert!(cache.get(&scope("rubix", "b")).is_none(), "lru victim evicted");
        assert!(cache.get(&scope("rubix", "a")).is_some());
        assert!(cache.get(&scope("rubix", "c")).is_some());
    }
}
