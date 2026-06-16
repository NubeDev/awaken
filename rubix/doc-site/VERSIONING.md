# Docs Versioning Strategy

Rubix is pre-1.0 (`0.1.0`) with no releases tagged yet. Maintaining multiple
documentation versions now would be pure overhead, so:

## Today: latest-only

- The site documents **one version: the current `main`**.
- There is no version switcher and no per-version content snapshots.
- Breaking changes between `0.x` releases are tracked in
  [`../CHANGELOG.md`](../CHANGELOG.md), surfaced as the site's Changelog page.

This keeps authoring friction near zero while the API is still moving.

## Later: per-release snapshots

Introduce versioned docs **only when both are true**:

1. You cut tagged releases on a real cadence (`rubix-v0.2.0`, …), and
2. Users run older versions and need docs that match.

When that happens, the lowest-overhead path with Nextra is **directory-based
snapshots**:

```
content/            # = "latest" (unversioned, tracks main)
versioned/
  v0.2/             # frozen copy of content/ at the v0.2 tag
  v0.1/
```

Procedure per release:

1. `git tag rubix-v0.2.0` at the release commit.
2. Copy `content/` → `versioned/v0.2/`.
3. Add a version switcher (Nextra navbar component) listing `latest` + frozen
   versions.
4. Never edit frozen snapshots except for critical corrections.

Until step 1 is part of your release process, **do not** add this — it only
adds maintenance cost. Revisit at the first `0.2.0` tag.

## Relationship to git tags

Tag Rubix releases as `v0.2.0`, `v1.0.0`, etc. If Rubix ships from a monorepo
shared with unrelated projects, prefix the tags (e.g. `rubix-v0.2.0`) so they
don't collide, and keep the prefix consistent across releases.
