extern crate alloc;
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
use uefi::proto::console::gop::{BltOp, BltPixel, BltRegion, GraphicsOutput};

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

    pub fn clear(&mut self, color: Rgb888) {
        let pixel = rgb_to_u32(color);
        self.buffer.fill(pixel);
    }

    pub fn present(&self, gop: &mut GraphicsOutput) {
        let blt_pixels: &[BltPixel] = unsafe {
            core::slice::from_raw_parts(
                self.buffer.as_ptr() as *const BltPixel,
                self.buffer.len(),
            )
        };
        let _ = gop.blt(BltOp::BufferToVideo {
            buffer: blt_pixels,
            src: BltRegion::Full,
            dest: (0, 0),
            dims: (self.width, self.height),
        });
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
            unsafe {
                let row = core::slice::from_raw_parts_mut(
                    self.buffer.as_mut_ptr().add(row_start),
                    self.width,
                );
                row[start_x..end_x].fill(pixel);
            }
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

#[inline]
pub fn rgb_to_u32(color: Rgb888) -> u32 {
    ((color.r() as u32) << 16) | ((color.g() as u32) << 8) | (color.b() as u32)
}

pub const COLOR_BG: Rgb888 = Rgb888::new(0x1A, 0x1A, 0x2E);
pub const COLOR_DICE_FACE: Rgb888 = Rgb888::new(0xE6, 0xE6, 0xE6);
pub const COLOR_DICE_PIP: Rgb888 = Rgb888::new(0x2D, 0x2D, 0x2D);
pub const COLOR_SELECTED: Rgb888 = Rgb888::new(0xF0, 0xC0, 0x40);
pub const COLOR_HELD: Rgb888 = Rgb888::new(0x88, 0x88, 0x88);
pub const COLOR_TEXT: Rgb888 = Rgb888::new(0xFF, 0xFF, 0xFF);
pub const COLOR_TURN_SCORE: Rgb888 = Rgb888::new(0x4E, 0xCC, 0xA3);
pub const COLOR_FARKLE: Rgb888 = Rgb888::new(0xE7, 0x4C, 0x3C);
pub const COLOR_BUTTON_ROLL: Rgb888 = Rgb888::new(0x2E, 0xCC, 0x71);
pub const COLOR_BUTTON_BANK: Rgb888 = Rgb888::new(0xE6, 0x7E, 0x22);
pub const COLOR_TITLE: Rgb888 = Rgb888::new(0xF0, 0xC0, 0x40);
