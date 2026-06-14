//! Reject multi-statement SQL before it reaches the backend.
//!
//! The datasource safety model (docs/design/datasources.md "One statement per
//! call") requires that exactly one statement is sent per call, so a
//! `SELECT`-only role cannot be bypassed by a trailing statement in the same
//! string. This is a structural guard, not a SQL parser: it strips string
//! literals, dollar-quoted bodies, and comments, then rejects any remaining
//! statement-separating `;` that is not merely a trailing terminator.
//!
//! Ambiguity noted (per the doc's intent): a full SQL grammar is out of scope
//! for this crate. The read-only DB role is the primary write guard; this check
//! is the belt-and-braces second line the doc calls for, so it errs toward
//! rejecting anything it cannot prove is a single statement.

use crate::error::{DatasourceError, DatasourceResult};

/// Accept SQL that is exactly one statement, returning it trimmed of a trailing
/// `;`. Rejects empty input and any interior statement separator.
pub fn ensure_single_statement(sql: &str) -> DatasourceResult<&str> {
    let trimmed = sql.trim();
    if trimmed.is_empty() {
        return Err(DatasourceError::EmptyStatement);
    }
    if has_interior_separator(trimmed) {
        return Err(DatasourceError::MultiStatement);
    }
    Ok(trimmed)
}

/// True if a `;` appears anywhere other than as the sole trailing terminator,
/// ignoring `;` inside single/double-quoted strings, dollar-quoted bodies, and
/// `--` / `/* */` comments.
fn has_interior_separator(sql: &str) -> bool {
    let bytes = sql.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        match c {
            '\'' | '"' => i = skip_quoted(bytes, i, c),
            '$' => match dollar_tag(bytes, i) {
                Some(tag_end) => i = skip_dollar_body(bytes, i, tag_end),
                None => i += 1,
            },
            '-' if bytes.get(i + 1) == Some(&b'-') => i = skip_line_comment(bytes, i),
            '/' if bytes.get(i + 1) == Some(&b'*') => i = skip_block_comment(bytes, i),
            ';' => {
                // A `;` is allowed only if everything after it is blank.
                return !sql[i + 1..].trim().is_empty();
            }
            _ => i += 1,
        }
    }
    false
}

/// Advance past a `'`- or `"`-quoted run starting at `open`, honoring doubled
/// quotes (`''`/`""`) as escapes. Returns the index just past the close.
fn skip_quoted(bytes: &[u8], open: usize, quote: char) -> usize {
    let q = quote as u8;
    let mut i = open + 1;
    while i < bytes.len() {
        if bytes[i] == q {
            if bytes.get(i + 1) == Some(&q) {
                i += 2; // doubled quote = literal, stay inside
                continue;
            }
            return i + 1;
        }
        i += 1;
    }
    bytes.len()
}

/// If `start` opens a dollar-quote tag (`$$` or `$tag$`), return the index just
/// past the opening tag; otherwise `None`.
fn dollar_tag(bytes: &[u8], start: usize) -> Option<usize> {
    let mut i = start + 1;
    while i < bytes.len() {
        match bytes[i] {
            b'$' => return Some(i + 1),
            b'_' => i += 1,
            c if c.is_ascii_alphanumeric() => i += 1,
            _ => return None,
        }
    }
    None
}

/// Advance past a dollar-quoted body whose opening tag spans `open..tag_end`.
fn skip_dollar_body(bytes: &[u8], open: usize, tag_end: usize) -> usize {
    let tag = &bytes[open..tag_end];
    let mut i = tag_end;
    while i + tag.len() <= bytes.len() {
        if &bytes[i..i + tag.len()] == tag {
            return i + tag.len();
        }
        i += 1;
    }
    bytes.len()
}

/// Advance past a `-- ...` line comment to the end of line (or input).
fn skip_line_comment(bytes: &[u8], start: usize) -> usize {
    let mut i = start + 2;
    while i < bytes.len() && bytes[i] != b'\n' {
        i += 1;
    }
    i
}

/// Advance past a `/* ... */` block comment (non-nested, matching Postgres'
/// actual nesting is out of scope; nested comments only over-skip, never
/// under-skip, so the guard stays conservative).
fn skip_block_comment(bytes: &[u8], start: usize) -> usize {
    let mut i = start + 2;
    while i + 1 < bytes.len() {
        if bytes[i] == b'*' && bytes[i + 1] == b'/' {
            return i + 2;
        }
        i += 1;
    }
    bytes.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_single_statement() {
        let sql = "SELECT 1";
        assert_eq!(ensure_single_statement(sql).unwrap(), "SELECT 1");
    }

    #[test]
    fn accepts_trailing_semicolon() {
        assert_eq!(
            ensure_single_statement("SELECT 1 ;  ").unwrap(),
            "SELECT 1 ;"
        );
    }

    #[test]
    fn rejects_empty() {
        assert!(matches!(
            ensure_single_statement("   "),
            Err(DatasourceError::EmptyStatement)
        ));
    }

    #[test]
    fn rejects_two_statements() {
        assert!(matches!(
            ensure_single_statement("SELECT 1; DROP TABLE t"),
            Err(DatasourceError::MultiStatement)
        ));
    }

    #[test]
    fn semicolon_inside_string_is_not_a_separator() {
        let sql = "SELECT ';' AS x, ''';''' AS y";
        assert_eq!(ensure_single_statement(sql).unwrap(), sql);
    }

    #[test]
    fn semicolon_inside_dollar_quote_is_not_a_separator() {
        let sql = "SELECT $$a;b$$ AS x";
        assert_eq!(ensure_single_statement(sql).unwrap(), sql);
    }

    #[test]
    fn tagged_dollar_quote_is_not_a_separator() {
        let sql = "SELECT $tag$ one; two $tag$ AS x";
        assert_eq!(ensure_single_statement(sql).unwrap(), sql);
    }

    #[test]
    fn semicolon_in_line_comment_is_not_a_separator() {
        let sql = "SELECT 1 -- a; b\n";
        assert!(ensure_single_statement(sql).is_ok());
    }

    #[test]
    fn semicolon_in_block_comment_is_not_a_separator() {
        let sql = "SELECT /* a; b */ 1";
        assert!(ensure_single_statement(sql).is_ok());
    }

    #[test]
    fn interior_semicolon_after_close_is_rejected() {
        assert!(matches!(
            ensure_single_statement("SELECT 'x'; SELECT 'y'"),
            Err(DatasourceError::MultiStatement)
        ));
    }
}
