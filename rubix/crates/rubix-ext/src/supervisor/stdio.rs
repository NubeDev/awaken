//! `Content-Length`-framed JSON-RPC over a child's stdio pipes.
//!
//! The process-flavour control channel is JSON-RPC framed exactly like LSP /
//! the Debug Adapter Protocol: each message is an ASCII `Content-Length: N\r\n`
//! header, a blank `\r\n`, then `N` bytes of JSON body. starter delegated this
//! to a shared `starter-jsonrpc-stdio` crate; rubix has no such crate and adding
//! one would couple us to starter's wire types, so the runtime owns a tiny,
//! self-contained framing here (`rubix/docs/design/EXTENSION-RUNTIME.md`,
//! "Recommendation: port").
//!
//! Length-prefix framing (not newline-delimited) is deliberate: an extension
//! child may legitimately emit embedded newlines or large payloads, and a
//! length prefix makes the read unambiguous and bounded.

use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt};

use crate::error::{ExtError, Result};

/// Defensive ceiling on a single inbound frame (16 MiB). A child announcing a
/// larger `Content-Length` is treated as a protocol error rather than allowed to
/// drive an unbounded allocation.
const MAX_FRAME_BYTES: usize = 16 * 1024 * 1024;

/// Write one `Content-Length`-framed frame: header, blank line, then `body`.
///
/// # Errors
/// Returns [`ExtError::Command`] if the underlying pipe write fails.
pub(crate) async fn write_frame<W>(writer: &mut W, body: &[u8]) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    writer
        .write_all(header.as_bytes())
        .await
        .map_err(|e| ExtError::Command(format!("write frame header: {e}")))?;
    writer
        .write_all(body)
        .await
        .map_err(|e| ExtError::Command(format!("write frame body: {e}")))?;
    writer
        .flush()
        .await
        .map_err(|e| ExtError::Command(format!("flush frame: {e}")))?;
    Ok(())
}

/// Serialise `value` to JSON and write it as one frame.
///
/// # Errors
/// Returns [`ExtError::Command`] on a serialisation or pipe-write failure.
pub(crate) async fn write_value<W>(writer: &mut W, value: &serde_json::Value) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    let body = serde_json::to_vec(value)
        .map_err(|e| ExtError::Command(format!("serialising frame: {e}")))?;
    write_frame(writer, &body).await
}

/// Read one frame's body bytes, or `None` at a clean EOF (the child closed
/// stdout between frames).
///
/// # Errors
/// Returns [`ExtError::Command`] on a malformed header, an oversized frame, or a
/// pipe-read failure mid-frame.
pub(crate) async fn read_frame<R>(reader: &mut R) -> Result<Option<Vec<u8>>>
where
    R: AsyncBufRead + Unpin,
{
    let mut content_length: Option<usize> = None;
    let mut line = String::new();

    loop {
        line.clear();
        let n = reader
            .read_line(&mut line)
            .await
            .map_err(|e| ExtError::Command(format!("read frame header: {e}")))?;
        if n == 0 {
            // EOF. A clean EOF between frames (no header seen yet) is `None`; an
            // EOF mid-header is a truncated frame.
            return if content_length.is_none() {
                Ok(None)
            } else {
                Err(ExtError::Command("eof mid-frame header".to_owned()))
            };
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            // Blank line terminates the header block.
            break;
        }
        if let Some(rest) = trimmed
            .strip_prefix("Content-Length:")
            .or_else(|| trimmed.strip_prefix("content-length:"))
        {
            let len: usize = rest
                .trim()
                .parse()
                .map_err(|e| ExtError::Command(format!("bad Content-Length: {e}")))?;
            if len > MAX_FRAME_BYTES {
                return Err(ExtError::Command(format!(
                    "frame of {len} bytes exceeds the {MAX_FRAME_BYTES}-byte ceiling"
                )));
            }
            content_length = Some(len);
        }
        // Unknown headers are ignored (forward-compatible with future fields).
    }

    let len = content_length
        .ok_or_else(|| ExtError::Command("frame header block had no Content-Length".to_owned()))?;
    let mut body = vec![0_u8; len];
    use tokio::io::AsyncReadExt;
    reader
        .read_exact(&mut body)
        .await
        .map_err(|e| ExtError::Command(format!("read frame body: {e}")))?;
    Ok(Some(body))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::BufReader;

    #[tokio::test]
    async fn write_then_read_round_trips() {
        let mut buf: Vec<u8> = Vec::new();
        write_value(&mut buf, &serde_json::json!({ "ok": true, "n": 7 }))
            .await
            .unwrap();
        let s = std::str::from_utf8(&buf).unwrap();
        assert!(s.starts_with("Content-Length: "));
        assert!(s.contains("\r\n\r\n"));

        let mut reader = BufReader::new(&buf[..]);
        let frame = read_frame(&mut reader).await.unwrap().expect("one frame");
        let v: serde_json::Value = serde_json::from_slice(&frame).unwrap();
        assert_eq!(v["ok"], true);
        assert_eq!(v["n"], 7);
    }

    #[tokio::test]
    async fn clean_eof_between_frames_is_none() {
        let empty: &[u8] = b"";
        let mut reader = BufReader::new(empty);
        assert!(read_frame(&mut reader).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn two_frames_read_in_sequence() {
        let mut buf: Vec<u8> = Vec::new();
        write_value(&mut buf, &serde_json::json!({ "i": 1 }))
            .await
            .unwrap();
        write_value(&mut buf, &serde_json::json!({ "i": 2 }))
            .await
            .unwrap();
        let mut reader = BufReader::new(&buf[..]);
        let f1 = read_frame(&mut reader).await.unwrap().unwrap();
        let f2 = read_frame(&mut reader).await.unwrap().unwrap();
        assert_eq!(serde_json::from_slice::<serde_json::Value>(&f1).unwrap()["i"], 1);
        assert_eq!(serde_json::from_slice::<serde_json::Value>(&f2).unwrap()["i"], 2);
        assert!(read_frame(&mut reader).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn oversized_frame_is_refused() {
        let header = b"Content-Length: 99999999999\r\n\r\n";
        let mut reader = BufReader::new(&header[..]);
        assert!(read_frame(&mut reader).await.is_err());
    }
}
