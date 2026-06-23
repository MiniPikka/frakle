use alloc::vec::Vec;
use alloc::vec;

use core::convert::Infallible;
use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::{OriginDimensions, Size},
    pixelcolor::Rgb888,
    prelude::RgbColor,
    primitives::Rectangle,
    Pixel,
};
use uefi::proto::console::gop::GraphicsOutput;

pub struct Framebuffer {
    buffer: Vec<u32>,
    width: usize,
    height: usize,
    /// Tracks whether any pixel was written since the last `present()`.
    /// Avoids expensive `clflush` when the screen hasn't changed (idle frames).
    dirty: bool,
}

/// 5×7 bitmap font data lookup — shared by all text renderers.
fn char_font(c: char) -> &'static [u8] {
    match c {
        '0' => &[0x3E, 0x51, 0x49, 0x45, 0x3E],
        '1' => &[0x00, 0x42, 0x7F, 0x40, 0x00],
        '2' => &[0x42, 0x61, 0x51, 0x49, 0x46],
        '3' => &[0x21, 0x41, 0x45, 0x4B, 0x31],
        '4' => &[0x18, 0x14, 0x12, 0x7F, 0x10],
        '5' => &[0x27, 0x45, 0x45, 0x45, 0x39],
        '6' => &[0x3C, 0x4A, 0x49, 0x49, 0x30],
        '7' => &[0x01, 0x71, 0x09, 0x05, 0x03],
        '8' => &[0x36, 0x49, 0x49, 0x49, 0x36],
        '9' => &[0x06, 0x49, 0x49, 0x29, 0x1E],
        'A' | 'a' => &[0x7E, 0x11, 0x11, 0x11, 0x7E],
        'B' | 'b' => &[0x7F, 0x49, 0x49, 0x49, 0x36],
        'C' | 'c' => &[0x3E, 0x41, 0x41, 0x41, 0x22],
        'D' | 'd' => &[0x7F, 0x41, 0x41, 0x22, 0x1C],
        'E' | 'e' => &[0x7F, 0x49, 0x49, 0x49, 0x41],
        'F' | 'f' => &[0x7F, 0x09, 0x09, 0x09, 0x01],
        'G' | 'g' => &[0x3E, 0x41, 0x49, 0x49, 0x7A],
        'H' | 'h' => &[0x7F, 0x08, 0x08, 0x08, 0x7F],
        'I' | 'i' => &[0x00, 0x41, 0x7F, 0x41, 0x00],
        'J' | 'j' => &[0x20, 0x40, 0x41, 0x3F, 0x01],
        'K' | 'k' => &[0x7F, 0x08, 0x14, 0x22, 0x41],
        'L' | 'l' => &[0x7F, 0x40, 0x40, 0x40, 0x40],
        'M' | 'm' => &[0x7F, 0x02, 0x0C, 0x02, 0x7F],
        'N' | 'n' => &[0x7F, 0x04, 0x08, 0x10, 0x7F],
        'O' | 'o' => &[0x3E, 0x41, 0x41, 0x41, 0x3E],
        'P' | 'p' => &[0x7F, 0x09, 0x09, 0x09, 0x06],
        'Q' | 'q' => &[0x3E, 0x41, 0x51, 0x21, 0x5E],
        'R' | 'r' => &[0x7F, 0x09, 0x19, 0x29, 0x46],
        'S' | 's' => &[0x46, 0x49, 0x49, 0x49, 0x31],
        'T' | 't' => &[0x01, 0x01, 0x7F, 0x01, 0x01],
        'U' | 'u' => &[0x3F, 0x40, 0x40, 0x40, 0x3F],
        'V' | 'v' => &[0x1F, 0x20, 0x40, 0x20, 0x1F],
        'W' | 'w' => &[0x3F, 0x40, 0x38, 0x40, 0x3F],
        'X' | 'x' => &[0x63, 0x14, 0x08, 0x14, 0x63],
        'Y' | 'y' => &[0x07, 0x08, 0x70, 0x08, 0x07],
        'Z' | 'z' => &[0x61, 0x51, 0x49, 0x45, 0x43],
        ':' => &[0x00, 0x36, 0x36, 0x00, 0x00],
        ' ' => &[0x00, 0x00, 0x00, 0x00, 0x00],
        _ => &[0x7F, 0x7F, 0x7F, 0x7F, 0x7F],
    }
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            buffer: vec![0u32; width * height],
            width,
            height,
            dirty: true,  // first frame always needs flush
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    /// Raw pointer to pixel buffer — for panic handler crash screen only.
    pub fn buffer_ptr(&mut self) -> *mut u32 {
        self.buffer.as_mut_ptr()
    }

    /// Direct mutable slice to the pixel buffer — for background renderer.
    #[inline]
    pub fn buffer_direct(&mut self) -> &mut [u32] {
        &mut self.buffer
    }

    /// Manually mark the buffer as changed (for direct pixel writes).
    #[inline]
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn clear(&mut self, color: Rgb888) {
        let pixel = rgb_to_u32(color);
        self.buffer.fill(pixel);
        self.dirty = true;
    }

    /// Copy the double-buffer to the GOP framebuffer directly.
    /// Internal buffer is BGRx format — identical to GOP FB memory layout —
    /// so each row is one `copy_nonoverlapping` call. Zero per-pixel overhead.
    ///
    /// Skips `clflush` entirely when no pixels have changed since the last
    /// call (dirty flag optimization — avoids ~58k cache flushes per idle frame).
    pub fn present(&mut self, gop: &mut GraphicsOutput) {
        // Early exit: nothing changed since last present().
        if !self.dirty {
            return;
        }

        let mode = gop.current_mode_info();
        let stride = mode.stride();
        let (fb_w, fb_h) = mode.resolution();
        let mut fb = gop.frame_buffer();
        let dst = fb.as_mut_ptr();
        let fb_size = fb.size();

        let h = self.height.min(fb_h);
        let w = self.width.min(fb_w);
        let row_bytes = w * 4;

        // Guard: verify the GOP framebuffer is large enough for our writes
        let needed = h.saturating_sub(1).saturating_mul(stride.saturating_mul(4))
            .saturating_add(row_bytes);
        if fb_size < needed || dst.is_null() {
            return;
        }

        for y in 0..h {
            let src = self.buffer[y * self.width..].as_ptr() as *const u8;
            // Safe: bounds verified by the `needed` check above.
            // `y * stride * 4` cannot overflow because `h` and `stride` are
            // limited by the validated GOP mode resolution.
            let dst_offset = y.saturating_mul(stride.saturating_mul(4));
            unsafe {
                core::ptr::copy_nonoverlapping(src, dst.add(dst_offset), row_bytes);
            }
        }

        // Flush CPU cache lines so writes reach the GOP framebuffer in memory.
        // Uses clflush (universally available on x86-64) + mfence for
        // cross-device visibility. 64-byte cache line granularity.
        unsafe {
            let fb_bytes = h.saturating_mul(stride.saturating_mul(4));
            let end = dst.add(fb_bytes);
            let mut p = dst;
            while p < end {
                core::arch::asm!("clflush [{}]", in(reg) p, options(nostack, preserves_flags));
                p = p.add(64);
            }
            core::arch::asm!("mfence", options(nostack, preserves_flags));
        }

        self.dirty = false;
    }

    /// Set a single pixel directly (bypasses embedded-graphics for speed).
    #[inline]
    pub fn set_pixel(&mut self, x: i32, y: i32, color: Rgb888) {
        if x >= 0 && y >= 0 {
            let xu = x as usize;
            let yu = y as usize;
            if xu < self.width && yu < self.height {
                self.buffer[yu * self.width + xu] = rgb_to_u32(color);
                self.dirty = true;
            }
        }
    }

    /// CRT scanline post-processing: darken every other row by ~5%.
    ///
    /// Very subtle — just enough to give the retro CRT texture without
    /// making dark backgrounds disappear.
    pub fn apply_scanlines(&mut self) {
        let w = self.width;
        for y in (1..self.height).step_by(2) {
            let row_start = y * w;
            for x in 0..w {
                let px = self.buffer[row_start + x];
                let b = ((px & 0xFF) * 243) >> 8;
                let g = (((px >> 8) & 0xFF) * 243) >> 8;
                let r = (((px >> 16) & 0xFF) * 243) >> 8;
                self.buffer[row_start + x] = b | (g << 8) | (r << 16);
            }
        }
        self.dirty = true;
    }

    fn draw_char(&mut self, x: usize, y: usize, c: char, color: Rgb888) -> usize {
        if x + 5 > self.width || y + 7 > self.height {
            return 6;
        }
        let font_data = char_font(c);
        let pixel = rgb_to_u32(color);
        for (col, &byte) in font_data.iter().enumerate() {
            if byte == 0 { continue; }
            for row in 0..7u8 {
                if byte & (1 << row) != 0 {
                    self.buffer[(y + row as usize) * self.width + x + col] = pixel;
                }
            }
        }
        6
    }

    /// Draw a small text string at (x, y)
    pub fn draw_text_small(&mut self, x: usize, y: usize, text: &str, color: Rgb888) {
        let mut cx = x;
        for c in text.chars() {
            cx += self.draw_char(cx, y, c, color);
        }
    }

    /// Draw a big title string at 4× scale with golden glow shadow.
    /// Each font pixel becomes a 4×4 block. A dim glow is drawn at (x+2, y+2)
    /// behind the bright text, creating a Balatro-style bloom effect.
    ///
    /// Intended for "F A R K L E" — the 3-char-wide spacing gives the
    /// characteristic spaced-out look.
    pub fn draw_title_big(&mut self, x: usize, y: usize, text: &str, color: Rgb888, glow: Rgb888) {
        let scale = 4usize;
        let char_w = 5 * scale;  // 20px per character
        let space_w = 3 * scale; // 12px for space (3× wider than normal)
        let mut cx = x;

        for c in text.chars() {
            let font_data: &[u8] = char_font(c);
            let advance = if c == ' ' {
                space_w
            } else {
                // Draw glow shadow first (offset +2, +2)
                for (col, &byte) in font_data.iter().enumerate() {
                    if byte == 0 { continue; }
                    for row in 0..7u8 {
                        if byte & (1 << row) != 0 {
                            let bx = cx + 2 + col * scale;
                            let by = y + 2 + row as usize * scale;
                            for dy in 0..scale {
                                for dx in 0..scale {
                                    self.set_pixel_unchecked(bx + dx, by + dy, glow);
                                }
                            }
                        }
                    }
                }
                // Draw bright foreground
                for (col, &byte) in font_data.iter().enumerate() {
                    if byte == 0 { continue; }
                    for row in 0..7u8 {
                        if byte & (1 << row) != 0 {
                            let bx = cx + col * scale;
                            let by = y + row as usize * scale;
                            for dy in 0..scale {
                                for dx in 0..scale {
                                    self.set_pixel_unchecked(bx + dx, by + dy, color);
                                }
                            }
                        }
                    }
                }
                char_w
            };
            cx += advance;
        }
    }

    /// Set pixel without bounds checking — caller guarantees coordinates are valid.
    #[inline(always)]
    fn set_pixel_unchecked(&mut self, x: usize, y: usize, color: Rgb888) {
        if x < self.width && y < self.height {
            self.buffer[y * self.width + x] = rgb_to_u32(color);
        }
    }

    /// Draw text at 2× scale — each pixel becomes a 2×2 block.
    /// Used for the debug overlay so it's readable at higher resolutions.
    pub fn draw_text_small_2x(&mut self, x: usize, y: usize, text: &str, color: Rgb888) {
        let mut cx = x;
        for c in text.chars() {
            cx += self.draw_char_2x(cx, y, c, color);
        }
    }

    /// Draw a single character at 2× scale (each font pixel → 2×2 block).
    fn draw_char_2x(&mut self, x: usize, y: usize, c: char, color: Rgb888) -> usize {
        if x + 10 > self.width || y + 14 > self.height {
            return 12;
        }
        let font_data: &[u8] = match c {
            '0' => &[0x3E, 0x51, 0x49, 0x45, 0x3E],
            '1' => &[0x00, 0x42, 0x7F, 0x40, 0x00],
            '2' => &[0x42, 0x61, 0x51, 0x49, 0x46],
            '3' => &[0x21, 0x41, 0x45, 0x4B, 0x31],
            '4' => &[0x18, 0x14, 0x12, 0x7F, 0x10],
            '5' => &[0x27, 0x45, 0x45, 0x45, 0x39],
            '6' => &[0x3C, 0x4A, 0x49, 0x49, 0x30],
            '7' => &[0x01, 0x71, 0x09, 0x05, 0x03],
            '8' => &[0x36, 0x49, 0x49, 0x49, 0x36],
            '9' => &[0x06, 0x49, 0x49, 0x29, 0x1E],
            'A' | 'a' => &[0x7E, 0x11, 0x11, 0x11, 0x7E],
            'B' | 'b' => &[0x7F, 0x49, 0x49, 0x49, 0x36],
            'C' | 'c' => &[0x3E, 0x41, 0x41, 0x41, 0x22],
            'D' | 'd' => &[0x7F, 0x41, 0x41, 0x22, 0x1C],
            'E' | 'e' => &[0x7F, 0x49, 0x49, 0x49, 0x41],
            'F' | 'f' => &[0x7F, 0x09, 0x09, 0x09, 0x01],
            'G' | 'g' => &[0x3E, 0x41, 0x49, 0x49, 0x7A],
            'H' | 'h' => &[0x7F, 0x08, 0x08, 0x08, 0x7F],
            'I' | 'i' => &[0x00, 0x41, 0x7F, 0x41, 0x00],
            'J' | 'j' => &[0x20, 0x40, 0x41, 0x3F, 0x01],
            'K' | 'k' => &[0x7F, 0x08, 0x14, 0x22, 0x41],
            'L' | 'l' => &[0x7F, 0x40, 0x40, 0x40, 0x40],
            'M' | 'm' => &[0x7F, 0x02, 0x0C, 0x02, 0x7F],
            'N' | 'n' => &[0x7F, 0x04, 0x08, 0x10, 0x7F],
            'O' | 'o' => &[0x3E, 0x41, 0x41, 0x41, 0x3E],
            'P' | 'p' => &[0x7F, 0x09, 0x09, 0x09, 0x06],
            'Q' | 'q' => &[0x3E, 0x41, 0x51, 0x21, 0x5E],
            'R' | 'r' => &[0x7F, 0x09, 0x19, 0x29, 0x46],
            'S' | 's' => &[0x46, 0x49, 0x49, 0x49, 0x31],
            'T' | 't' => &[0x01, 0x01, 0x7F, 0x01, 0x01],
            'U' | 'u' => &[0x3F, 0x40, 0x40, 0x40, 0x3F],
            'V' | 'v' => &[0x1F, 0x20, 0x40, 0x20, 0x1F],
            'W' | 'w' => &[0x3F, 0x40, 0x38, 0x40, 0x3F],
            'X' | 'x' => &[0x63, 0x14, 0x08, 0x14, 0x63],
            'Y' | 'y' => &[0x07, 0x08, 0x70, 0x08, 0x07],
            'Z' | 'z' => &[0x61, 0x51, 0x49, 0x45, 0x43],
            ':' => &[0x00, 0x36, 0x36, 0x00, 0x00],
            ' ' => &[0x00, 0x00, 0x00, 0x00, 0x00],
            _ => &[0x7F, 0x7F, 0x7F, 0x7F, 0x7F],
        };

        let pixel = rgb_to_u32(color);
        for (col, &byte) in font_data.iter().enumerate() {
            if byte == 0 { continue; }
            for row in 0..7u8 {
                if byte & (1 << row) != 0 {
                    // Write 2×2 block for each font pixel
                    let bx = x + col * 2;
                    let by = y + row as usize * 2;
                    self.buffer[by * self.width + bx] = pixel;
                    self.buffer[by * self.width + bx + 1] = pixel;
                    self.buffer[(by + 1) * self.width + bx] = pixel;
                    self.buffer[(by + 1) * self.width + bx + 1] = pixel;
                }
            }
        }
        12  // character width at 2× (5 cols × 2 + 2 spacing)
    }
}

impl OriginDimensions for Framebuffer {
    fn size(&self) -> Size {
        Size::new(self.width as u32, self.height as u32)
    }
}

impl DrawTarget for Framebuffer {
    type Color = Rgb888;
    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(coord, color) in pixels {
            let x = coord.x;
            let y = coord.y;
            if x >= 0 && y >= 0 {
                let xu = x as usize;
                let yu = y as usize;
                if xu < self.width && yu < self.height {
                    self.buffer[yu * self.width + xu] = rgb_to_u32(color);
                    self.dirty = true;
                }
            }
        }
        Ok(())
    }

    fn fill_solid(&mut self, area: &Rectangle, color: Self::Color) -> Result<(), Self::Error> {
        let start_x = area.top_left.x.max(0) as usize;
        let start_y = area.top_left.y.max(0) as usize;
        let end_x = ((area.top_left.x + area.size.width as i32) as usize).min(self.width);
        let end_y = ((area.top_left.y + area.size.height as i32) as usize).min(self.height);
        if start_x >= end_x || start_y >= end_y { return Ok(()); }
        let pixel = rgb_to_u32(color);
        for y in start_y..end_y {
            let row_start = y * self.width;
            // Safe: bounds already checked above
            self.buffer[row_start + start_x..row_start + end_x].fill(pixel);
        }
        self.dirty = true;
        Ok(())
    }

    fn fill_contiguous<I>(&mut self, area: &Rectangle, colors: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Self::Color>,
    {
        let start_x = area.top_left.x.max(0) as usize;
        let start_y = area.top_left.y.max(0) as usize;
        let end_x = ((area.top_left.x + area.size.width as i32) as usize).min(self.width);
        let end_y = ((area.top_left.y + area.size.height as i32) as usize).min(self.height);

        let mut color_iter = colors.into_iter();
        let mut wrote = false;
        'outer: for y in start_y..end_y {
            let row_start = y * self.width;
            for x in start_x..end_x {
                if let Some(c) = color_iter.next() {
                    self.buffer[row_start + x] = rgb_to_u32(c);
                    wrote = true;
                } else {
                    break 'outer;
                }
            }
        }
        if wrote { self.dirty = true; }
        Ok(())
    }
}

/// Convert Rgb888 to BGRx u32 (matches GOP framebuffer format on x86_64).
/// Stored as BGR in memory so `present()` can memcpy directly to GOP FB.
#[inline]
pub fn rgb_to_u32(color: Rgb888) -> u32 {
    (color.b() as u32) | ((color.g() as u32) << 8) | ((color.r() as u32) << 16)
}

// ── Balatro-inspired color palette ──────────────────────────────────────────
// Deep, moody backgrounds with vibrant neon accents. The original Balatro
// uses #0D0620 (deep purple) → #0A1628 (deep blue) as its core range, with
// saturated accent colors that "pop" against the darkness.

/// Background fallback — matches the gradient's top color.
pub const COLOR_BG: Rgb888 = Rgb888::new(0x0A, 0x06, 0x18);

/// Dice face — warm off-white with a slight blue tint (Balatro card feel)
pub const COLOR_DICE_FACE: Rgb888 = Rgb888::new(0xE8, 0xE4, 0xF0);
/// Dice pip — deep indigo (not pure black — keeps the moody aesthetic)
pub const COLOR_DICE_PIP: Rgb888 = Rgb888::new(0x1A, 0x14, 0x30);
/// Selected die — Balatro gold (#F0B040), the universal "important" highlight
pub const COLOR_SELECTED: Rgb888 = Rgb888::new(0xF0, 0xB0, 0x40);
/// Held die — muted slate (visually recedes without disappearing)
pub const COLOR_HELD: Rgb888 = Rgb888::new(0x50, 0x50, 0x70);
/// Body text — cool white with blue cast (never harsh pure white)
pub const COLOR_TEXT: Rgb888 = Rgb888::new(0xE0, 0xE0, 0xF0);
/// Turn score / positive info — Balatro teal-green (#40D0C0)
pub const COLOR_TURN_SCORE: Rgb888 = Rgb888::new(0x40, 0xD0, 0xC0);
/// Farkle / negative feedback — Balatro red (#E04040)
pub const COLOR_FARKLE: Rgb888 = Rgb888::new(0xE0, 0x40, 0x40);
/// Roll button — Balatro green (#4AE07A)
pub const COLOR_BUTTON_ROLL: Rgb888 = Rgb888::new(0x4A, 0xE0, 0x7A);
/// Bank button — Balatro amber (#F0A030)
pub const COLOR_BUTTON_BANK: Rgb888 = Rgb888::new(0xF0, 0xA0, 0x30);
/// Title / gold accent — warm gold (#F0B040)
pub const COLOR_TITLE: Rgb888 = Rgb888::new(0xF0, 0xB0, 0x40);
