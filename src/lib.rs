#![cfg_attr(not(test), no_std)]

extern crate alloc;

pub mod framebuffer;
pub mod game;
pub mod input;
pub mod effects;
pub mod sound;
pub mod ui;
pub mod logger;

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
        core::str::from_utf8(&self.buf[..self.len]).unwrap_or("?")
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
        self.buf[self.len..self.len + n].copy_from_slice(&bytes[..n]);
        self.len += n;
        Ok(())
    }
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
