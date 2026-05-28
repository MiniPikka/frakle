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
        .draw_styled(
            &PrimitiveStyle::with_fill(Rgb888::new(0x10, 0x10, 0x20)),
            fb,
        );

    // Die face
    let _ = Rectangle::new(Point::new(x, y), Size::new(s as u32, s as u32))
        .draw_styled(&PrimitiveStyle::with_fill(face_color), fb);

    // Die border
    let border_color = if is_selected {
        COLOR_SELECTED
    } else if is_cursor {
        Rgb888::new(0xCC, 0xCC, 0xFF)
    } else {
        COLOR_DICE_PIP
    };
    let border = PrimitiveStyle::with_stroke(border_color, 2);
    let _ = Rectangle::new(Point::new(x, y), Size::new(s as u32, s as u32))
        .draw_styled(&border, fb);

    // Draw pips
    if (1..=6).contains(&value) {
        let pip_color = if is_held {
            Rgb888::new(0x66, 0x66, 0x66)
        } else {
            COLOR_DICE_PIP
        };
        let pip_style = PrimitiveStyle::with_fill(pip_color);
        for &(px, py) in layout::pip_positions(value) {
            let cx = x + px;
            let cy = y + py;
            let _ = Circle::new(Point::new(cx, cy), layout::PIP_RADIUS)
                .draw_styled(&pip_style, fb);
        }
    }

    // Selection indicator below die
    if is_selected {
        let marker_y = y + s + 6;
        let marker_x = x + s / 2;
        let marker_style = PrimitiveStyle::with_fill(COLOR_SELECTED);
        let _ = Circle::new(Point::new(marker_x, marker_y), 5)
            .draw_styled(&marker_style, fb);
    } else if is_cursor {
        let marker_y = y + s + 6;
        let marker_x = x + s / 2;
        let marker_outline = PrimitiveStyle::with_stroke(Rgb888::new(0xCC, 0xCC, 0xFF), 1);
        let _ = Circle::new(Point::new(marker_x, marker_y), 5)
            .draw_styled(&marker_outline, fb);
    }
}

pub fn draw_roll_animation(fb: &mut Framebuffer, game: &Game, frame: u32, _total: u32) {
    let fb_width = fb.width();
    let dice_y = layout::dice_row_y(fb.height() as i32);
    let seed = frame.wrapping_mul(0x9E3779B9).wrapping_add(0xDEADBEEF);
    for i in 0..6 {
        if game.held_dice[i] {
            continue;
        }
        let x = layout::die_x(i, fb_width);
        let y = dice_y;
        let val = ((seed >> (i * 5)) & 0x7) as u8 + 1;
        let v = if val > 6 { ((val - 1) % 6) + 1 } else { val };
        draw_die(fb, x, y, v, false, false, false);
    }
}
