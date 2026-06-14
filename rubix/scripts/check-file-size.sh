#!/usr/bin/env bash
# File-size guard from docs/FILE-LAYOUT.md §9: no tracked *.rs / *.ts / *.tsx
# may exceed 400 lines, excluding generated code under src/generated/.
#
# Run from the rubix workspace root. Exits non-zero (listing offenders) if any
# tracked source file is over the limit.
set -euo pipefail

LIMIT=400
fail=0

# Scope the guard to the rubix workspace (the directory above this script),
# regardless of the caller's CWD. rubix lives inside the awaken git repo, so the
# guard must not range over the wider repo.
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
rubix_root="$(dirname "$script_dir")"
cd "$rubix_root"

while IFS= read -r file; do
    case "$file" in
        */src/generated/*) continue ;;
    esac
    [ -f "$file" ] || continue
    lines=$(wc -l < "$file")
    if [ "$lines" -gt "$LIMIT" ]; then
        printf '%s: %d lines (limit %d)\n' "$file" "$lines" "$LIMIT"
        fail=1
    fi
done < <(git ls-files '*.rs' '*.ts' '*.tsx')

if [ "$fail" -ne 0 ]; then
    echo "file-size guard failed: split files above the ${LIMIT}-line limit" >&2
    exit 1
fi

echo "file-size guard passed: no tracked source over ${LIMIT} lines"
