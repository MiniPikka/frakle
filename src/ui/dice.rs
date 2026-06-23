use embedded_graphics::{
    geometry::Point,
    pixelcolor::Rgb888,
    prelude::*,
    primitives::{Circle, PrimitiveStyle, Rectangle, StyledDrawable},
};

use crate::framebuffer::{
    Framebuffer, COLOR_DICE_FACE, COLOR_DICE_PIP, COLOR_HELD, COLOR_SELECTED,
};
use crate::ui::layout;
use crate::game::Game;

pub fn draw_all_dice(fb: &mut Framebuffer, game: &Game) {
    let fb_width = fb.width();
    let dice_y = layout::dice_row_y(fb.height() as i32);
    for i in 0..6 {
        let x = layout::die_x(i, fb_width);
        let y = dice_y;
        let is_held = game.held_dice[i];
        let is_selected = game.selected_dice[i];
        let is_cursor = game.cursor == i;
        let value = game.dice[i];

        draw_die(fb, x, y, value, is_held, is_selected, is_cursor);
    }
}

// ── Balatro-style dice palette ──
const COLOR_CURSOR: Rgb888 = Rgb888::new(0x60, 0xA0, 0xE0);    // cool blue highlight
const COLOR_CURSOR_ON_SEL: Rgb888 = Rgb888::new(0xF0, 0xE0, 0xFF); // bright violet-white
const COLOR_SHADOW: Rgb888 = Rgb888::new(0x06, 0x04, 0x12);    // deep purple shadow
const COLOR_HELD_PIP: Rgb888 = Rgb888::new(0x38, 0x38, 0x50);  // muted indigo
const COLOR_GLOW: Rgb888 = Rgb888::new(0xF0, 0xB0, 0x40);      // gold glow halo
const COLOR_GLOW_DIM: Rgb888 = Rgb888::new(0x30, 0x20, 0x08);  // dimmed glow for soft halo

pub fn draw_die(
    fb: &mut Framebuffer,
    x: i32,
    y: i32,
    value: u8,
    is_held: bool,
    is_selected: bool,
    is_cursor: bool,
) {
    let s = layout::DIE_SIZE;
    let face_color = if is_held { COLOR_HELD } else { COLOR_DICE_FACE };

    // ── Glow halo (Balatro-style bloom) ──
    // Selected dice get a multi-layered golden glow behind them.
    // Three layers of decreasing opacity simulate Gaussian blur.
    if is_selected && is_cursor {
        let _ = Rectangle::new(Point::new(x - 6, y - 6), Size::new(s as u32 + 12, s as u32 + 12))
            .draw_styled(&PrimitiveStyle::with_fill(Rgb888::new(0x18, 0x10, 0x04)), fb);
        let _ = Rectangle::new(Point::new(x - 4, y - 4), Size::new(s as u32 + 8, s as u32 + 8))
            .draw_styled(&PrimitiveStyle::with_fill(COLOR_GLOW_DIM), fb);
        let _ = Rectangle::new(Point::new(x - 2, y - 2), Size::new(s as u32 + 4, s as u32 + 4))
            .draw_styled(&PrimitiveStyle::with_fill(Rgb888::new(0x50, 0x38, 0x10)), fb);
    } else if is_selected {
        let _ = Rectangle::new(Point::new(x - 5, y - 5), Size::new(s as u32 + 10, s as u32 + 10))
            .draw_styled(&PrimitiveStyle::with_fill(Rgb888::new(0x10, 0x08, 0x02)), fb);
        let _ = Rectangle::new(Point::new(x - 3, y - 3), Size::new(s as u32 + 6, s as u32 + 6))
            .draw_styled(&PrimitiveStyle::with_fill(COLOR_GLOW_DIM), fb);
    }

    // Die shadow (offset 2px down-right)
    let _ = Rectangle::new(Point::new(x + 2, y + 2), Size::new(s as u32, s as u32))
        .draw_styled(&PrimitiveStyle::with_fill(COLOR_SHADOW), fb);

    // Die face
    let _ = Rectangle::new(Point::new(x, y), Size::new(s as u32, s as u32))
        .draw_styled(&PrimitiveStyle::with_fill(face_color), fb);

    // Die border — Balatro style: gold for selected, blue for cursor, subtle for default
    let (border_color, border_w) = if is_cursor && is_selected {
        (COLOR_GLOW, 3)       // bright gold — maximum emphasis
    } else if is_selected {
        (COLOR_SELECTED, 2)   // gold
    } else if is_cursor {
        (COLOR_CURSOR, 2)     // cool blue
    } else {
        (COLOR_DICE_PIP, 1)   // subtle dark border
    };
    let border = PrimitiveStyle::with_stroke(border_color, border_w);
    let _ = Rectangle::new(Point::new(x, y), Size::new(s as u32, s as u32))
        .draw_styled(&border, fb);

    // Draw pips
    if (1..=6).contains(&value) {
        let pip_color = if is_held { COLOR_HELD_PIP } else { COLOR_DICE_PIP };
        let pip_style = PrimitiveStyle::with_fill(pip_color);
        for &(px, py) in layout::pip_positions(value) {
            let cx = x + px;
            let cy = y + py;
            let _ = Circle::new(Point::new(cx, cy), layout::PIP_RADIUS)
                .draw_styled(&pip_style, fb);
        }
    }

    // Marker below die — Balatro style indicators
    let marker_y = y + s + 6;
    let marker_x = x + s / 2;
    if is_selected && is_cursor {
        // Gold dot + bright ring
        let _ = Circle::new(Point::new(marker_x, marker_y), 5)
            .draw_styled(&PrimitiveStyle::with_fill(COLOR_GLOW), fb);
        let _ = Circle::new(Point::new(marker_x, marker_y), 7)
            .draw_styled(&PrimitiveStyle::with_stroke(COLOR_CURSOR_ON_SEL, 1), fb);
    } else if is_selected {
        let _ = Circle::new(Point::new(marker_x, marker_y), 5)
            .draw_styled(&PrimitiveStyle::with_fill(COLOR_SELECTED), fb);
    } else if is_cursor {
        let _ = Circle::new(Point::new(marker_x, marker_y), 5)
            .draw_styled(&PrimitiveStyle::with_stroke(COLOR_CURSOR, 1), fb);
    }
}

pub fn draw_roll_animation(fb: &mut Framebuffer, game: &Game, frame: u32, _total: u32) {
    let fb_width = fb.width();
    let dice_y = layout::dice_row_y(fb.height() as i32);
    let seed = frame.wrapping_mul(0x9E3779B9).wrapping_add(0xDEADBEEF);
    for i in 0..6 {
        let x = layout::die_x(i, fb_width);
        if game.held_dice[i] {
            // Show held dice in dimmed state so player can see what's set aside
            draw_die(fb, x, dice_y, game.dice[i], true, false, false);
        } else {
            let v = (((seed >> (i * 5)) & 0xFF) % 6) as u8 + 1;
            draw_die(fb, x, dice_y, v, false, false, false);
        }
    }
}
