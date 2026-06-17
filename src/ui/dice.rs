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

const COLOR_CURSOR: Rgb888 = Rgb888::new(0x80, 0xD0, 0xFF);
const COLOR_CURSOR_ON_SEL: Rgb888 = Rgb888::new(0xFF, 0xFF, 0xFF);
const COLOR_SHADOW: Rgb888 = Rgb888::new(0x0C, 0x0C, 0x1A);
const COLOR_HELD_PIP: Rgb888 = Rgb888::new(0x6A, 0x6A, 0x80);

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

    // Die shadow
    let _ = Rectangle::new(Point::new(x + 2, y + 2), Size::new(s as u32, s as u32))
        .draw_styled(&PrimitiveStyle::with_fill(COLOR_SHADOW), fb);

    // Die face
    let _ = Rectangle::new(Point::new(x, y), Size::new(s as u32, s as u32))
        .draw_styled(&PrimitiveStyle::with_fill(face_color), fb);

    // Die border — cursor-on-selected gets a bright white thick border
    let (border_color, border_w) = if is_cursor && is_selected {
        (COLOR_CURSOR_ON_SEL, 3)
    } else if is_selected {
        (COLOR_SELECTED, 2)
    } else if is_cursor {
        (COLOR_CURSOR, 2)
    } else {
        (COLOR_DICE_PIP, 1)
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

    // Marker below die: selected = gold dot, cursor = blue ring,
    // cursor-on-selected = gold dot with white ring around it
    let marker_y = y + s + 6;
    let marker_x = x + s / 2;
    if is_selected && is_cursor {
        let _ = Circle::new(Point::new(marker_x, marker_y), 5)
            .draw_styled(&PrimitiveStyle::with_fill(COLOR_SELECTED), fb);
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
