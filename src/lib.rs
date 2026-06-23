#![cfg_attr(not(test), no_std)]

extern crate alloc;

pub mod framebuffer;
pub mod game;
pub mod input;
pub mod effects;
pub mod sound;
pub mod ui;
pub mod logger;
pub mod background;

use core::fmt::Write;

/// Stack-allocated formatting buffer — avoids heap allocations for short strings.
/// Used by both main loop (debug overlay) and UI renderer (score display, etc.).
pub struct FmtBuf<const N: usize> {
    buf: [u8; N],
    len: usize,
}

impl<const N: usize> FmtBuf<N> {
    pub fn new() -> Self { Self { buf: [0u8; N], len: 0 } }
    pub fn as_str(&self) -> &str {
        // `clamp_utf8` in `write_str` guarantees we never split a UTF-8
        // sequence, so this is always valid.
        core::str::from_utf8(&self.buf[..self.len]).unwrap_or("")
    }
    pub fn clear(&mut self) { self.len = 0; }
}

impl<const N: usize> Default for FmtBuf<N> {
    fn default() -> Self { Self::new() }
}

impl<const N: usize> core::fmt::Write for FmtBuf<N> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let bytes = s.as_bytes();
        let avail = N.saturating_sub(self.len);
        let n = bytes.len().min(avail);
        // Clamp to a valid UTF-8 boundary to prevent `as_str()` from
        // returning an error on a mid-character truncation. Walk back to
        // the first continuation byte (0b10xxxxxx) boundary.
        let safe_n = clamp_utf8(&bytes[..n]);
        self.buf[self.len..self.len + safe_n].copy_from_slice(&bytes[..safe_n]);
        self.len += safe_n;
        Ok(())
    }
}

/// Find the longest prefix of `buf` that is valid UTF-8.
/// If `buf` ends mid-character, returns the index just before the incomplete
/// sequence so `from_utf8(buf[..n])` always succeeds.
fn clamp_utf8(buf: &[u8]) -> usize {
    if buf.is_empty() { return 0; }
    let mut end = buf.len();
    // Walk backwards past continuation bytes (10xxxxxx)
    while end > 0 && (buf[end - 1] & 0xC0) == 0x80 {
        end -= 1;
    }
    // If we walked back to the start, the entire buffer was continuation
    // bytes — meaning a single character was split. Drop it entirely.
    if end == 0 { return 0; }
    // Also drop the leading byte of the incomplete sequence
    if end < buf.len() { end -= 1; }
    end
}

/// Replace {s}, {r}, {h}, {n} placeholders in a template into a FmtBuf.
/// Produces no heap allocations — everything is stack-based.
pub fn fmt_replace<const N: usize>(
    buf: &mut FmtBuf<N>,
    template: &str,
    s_val: &str,
    r_val: &str,
    h_val: &str,
    n_val: &str,
) {
    let mut rest = template;
    while let Some(pos) = rest.find('{') {
        let _ = buf.write_str(&rest[..pos]);
        rest = &rest[pos..];
        if let Some(end) = rest.find('}') {
            let val = match &rest[1..end] {
                "s" => s_val,
                "r" => r_val,
                "h" => h_val,
                "n" => n_val,
                _ => &rest[..=end],
            };
            let _ = buf.write_str(val);
            rest = &rest[end + 1..];
        } else {
            let _ = buf.write_str("{");
            rest = &rest[1..];
        }
    }
    let _ = buf.write_str(rest);
}
