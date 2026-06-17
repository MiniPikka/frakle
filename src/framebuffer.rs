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
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            buffer: vec![0u32; width * height],
            width,
            height,
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

    pub fn clear(&mut self, color: Rgb888) {
        let pixel = rgb_to_u32(color);
        self.buffer.fill(pixel);
    }

    /// Copy the double-buffer to the GOP framebuffer directly.
    /// Internal buffer is BGRx format — identical to GOP FB memory layout —
    /// so each row is one `copy_nonoverlapping` call. Zero per-pixel overhead.
    pub fn present(&mut self, gop: &mut GraphicsOutput) {
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
        let needed = (h.saturating_sub(1)) * stride * 4 + row_bytes;
        if fb_size < needed || dst.is_null() {
            return;
        }

        for y in 0..h {
            let src = self.buffer[y * self.width..].as_ptr() as *const u8;
            unsafe {
                core::ptr::copy_nonoverlapping(src, dst.add(y * stride * 4), row_bytes);
            }
        }

        // Flush CPU cache lines so writes reach the GOP framebuffer in memory.
        unsafe {
            let mut p = dst;
            let end = dst.add(h * stride * 4);
            while p < end {
                core::arch::asm!("clflush [{}]", in(reg) p, options(nostack, preserves_flags));
                p = p.add(64);
            }
            core::arch::asm!("mfence", options(nostack, preserves_flags));
        }
    }

    /// Set a single pixel directly (bypasses embedded-graphics for speed).
    #[inline]
    pub fn set_pixel(&mut self, x: i32, y: i32, color: Rgb888) {
        if x >= 0 && y >= 0 {
            let xu = x as usize;
            let yu = y as usize;
            if xu < self.width && yu < self.height {
                self.buffer[yu * self.width + xu] = rgb_to_u32(color);
            }
        }
    }

    /// Draw a small character at (x, y) with given color
    /// Returns the width of the character drawn
    fn draw_char(&mut self, x: usize, y: usize, c: char, color: Rgb888) -> usize {
        // Simple 5x7 font for digits and basic chars
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
            _ => &[0x7F, 0x7F, 0x7F, 0x7F, 0x7F], // Default block
        };

        let pixel = rgb_to_u32(color);
        for (col, &byte) in font_data.iter().enumerate() {
            for row in 0..7 {
                if byte & (1 << row) != 0 {
                    let px = x + col;
                    let py = y + row;
                    if px < self.width && py < self.height {
                        self.buffer[py * self.width + px] = pixel;
                    }
                }
            }
        }
        6 // Character width + spacing
    }

    /// Draw a small text string at (x, y)
    pub fn draw_text_small(&mut self, x: usize, y: usize, text: &str, color: Rgb888) {
        let mut cx = x;
        for c in text.chars() {
            cx += self.draw_char(cx, y, c, color);
        }
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
        let pixel = rgb_to_u32(color);
        for y in start_y..end_y {
            let row_start = y * self.width;
            // Safe: bounds already checked above
            self.buffer[row_start + start_x..row_start + end_x].fill(pixel);
        }
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
        'outer: for y in start_y..end_y {
            let row_start = y * self.width;
            for x in start_x..end_x {
                if let Some(c) = color_iter.next() {
                    self.buffer[row_start + x] = rgb_to_u32(c);
                } else {
                    break 'outer;
                }
            }
        }
        Ok(())
    }
}

/// Convert Rgb888 to BGRx u32 (matches GOP framebuffer format on x86_64).
/// Stored as BGR in memory so `present()` can memcpy directly to GOP FB.
#[inline]
pub fn rgb_to_u32(color: Rgb888) -> u32 {
    (color.b() as u32) | ((color.g() as u32) << 8) | ((color.r() as u32) << 16)
}

pub const COLOR_BG: Rgb888 = Rgb888::new(0x16, 0x16, 0x2A);
pub const COLOR_DICE_FACE: Rgb888 = Rgb888::new(0xF5, 0xF0, 0xE8);
pub const COLOR_DICE_PIP: Rgb888 = Rgb888::new(0x1E, 0x1E, 0x2A);
pub const COLOR_SELECTED: Rgb888 = Rgb888::new(0xFF, 0xC8, 0x47);
pub const COLOR_HELD: Rgb888 = Rgb888::new(0x9E, 0x9E, 0xB0);
pub const COLOR_TEXT: Rgb888 = Rgb888::new(0xF0, 0xF0, 0xF5);
pub const COLOR_TURN_SCORE: Rgb888 = Rgb888::new(0x50, 0xE3, 0xC2);
pub const COLOR_FARKLE: Rgb888 = Rgb888::new(0xFF, 0x5C, 0x5C);
pub const COLOR_BUTTON_ROLL: Rgb888 = Rgb888::new(0x38, 0xD9, 0x7C);
pub const COLOR_BUTTON_BANK: Rgb888 = Rgb888::new(0xFF, 0x8C, 0x42);
pub const COLOR_TITLE: Rgb888 = Rgb888::new(0xFF, 0xAA, 0x2B);
