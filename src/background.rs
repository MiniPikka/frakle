// Balatro-inspired background for UEFI.
//
// Balatro's actual background is intentionally INVISIBLE — it's so dark
// and subtle that you barely notice it. The visual impact comes from the
// UI elements on top: gold highlights, card shadows, particle effects.
//
// Strategy: solid dark gradient (top→bottom: near-black purple → near-black
// blue). Simple, fast, zero visual noise. Let the UI do the talking.

use crate::framebuffer::Framebuffer;

/// How many game frames between background redraws (always 1 — it's cheap).
const UPDATE_INTERVAL: u32 = 1;

pub struct Background {
    frame: u32,
}

impl Background {
    pub fn new() -> Self {
        Self { frame: 0 }
    }

    /// Render a vertical gradient: top (#0A0618) → bottom (#060A18).
    /// Both colors are near-black with a subtle purple/blue tint.
    /// Costs ~0.2ms at 1024×768 — just a fill loop.
    pub fn render(&mut self, fb: &mut Framebuffer) {
        self.frame = self.frame.wrapping_add(1);
        if !self.frame.is_multiple_of(UPDATE_INTERVAL) {
            return;
        }

        let w = fb.width();
        let h = fb.height();
        let buf = fb.buffer_direct();

        // Top color: dark purple   (R=10, G=6,  B=24)
        // Bottom color: dark blue  (R=6,  G=10, B=24)
        let (r0, g0, b0) = (10u32, 6u32, 24u32);
        let (r1, g1, b1) = (6u32, 10u32, 24u32);
        let h_u32 = h.max(1) as u32;

        let dr = r1 as i32 - r0 as i32;
        let dg = g1 as i32 - g0 as i32;
        let db = b1 as i32 - b0 as i32;

        for y in 0..h {
            let t = (y as i32 * 256) / h_u32 as i32;  // 0..256
            let r = (r0 as i32 + ((dr * t) >> 8)).clamp(0, 255) as u8;
            let g = (g0 as i32 + ((dg * t) >> 8)).clamp(0, 255) as u8;
            let b = (b0 as i32 + ((db * t) >> 8)).clamp(0, 255) as u8;
            let pixel = (b as u32) | ((g as u32) << 8) | ((r as u32) << 16);
            let row_start = y * w;
            buf[row_start..row_start + w].fill(pixel);
        }

        fb.mark_dirty();
    }
}

impl Default for Background {
    fn default() -> Self { Self::new() }
}
