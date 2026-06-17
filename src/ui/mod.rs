pub mod dice;
pub mod layout;
pub mod lang;
pub mod cn_font;

use embedded_graphics::{
    geometry::Point,
    mono_font::{MonoFont, MonoTextStyle,
                 ascii::FONT_10X20, ascii::FONT_7X13, ascii::FONT_9X15, ascii::FONT_9X15_BOLD},
    pixelcolor::Rgb888,
    prelude::*,
    primitives::{Circle, PrimitiveStyle, Rectangle, StyledDrawable},
    text::{Alignment, Text},
};

use crate::framebuffer::{
    Framebuffer, COLOR_BG, COLOR_BUTTON_BANK, COLOR_BUTTON_ROLL, COLOR_DICE_FACE,
    COLOR_DICE_PIP, COLOR_FARKLE, COLOR_HELD, COLOR_SELECTED, COLOR_TEXT, COLOR_TITLE, COLOR_TURN_SCORE,
};
use crate::game::{Game, GamePhase, TurnPhase};
use crate::{FmtBuf, fmt_replace};

use core::fmt::Write;

pub fn render(fb: &mut Framebuffer, game: &Game) {
    fb.clear(COLOR_BG);
    let l = lang::lang(game.lang_cn);

    match game.phase {
        GamePhase::Title => render_title(fb, game, l),
        GamePhase::PlayerTurn(tp) => render_player_turn(fb, game, tp, l),
        GamePhase::AiTurn => render_ai_turn_detail(fb, game, true, l),
        GamePhase::AiShowMeld { .. } => render_ai_turn_detail(fb, game, true, l),
        GamePhase::AiRolling { .. } => render_ai_turn_detail(fb, game, false, l),
        GamePhase::AiSelecting { .. } => render_ai_turn_detail(fb, game, false, l),
        GamePhase::GameOver => render_game_over(fb, game, l),
        GamePhase::Quit => {}
    }
}

/// Render centered text (EN via Alignment, CN via manual centering)
fn draw_text_center(fb: &mut Framebuffer, game: &Game, text: &str, center_x: i32, y: i32, color: Rgb888, font: &MonoFont) {
    if game.lang_cn {
        // Estimate width: CN chars ~13px, ASCII ~8px
        let w: i32 = text.chars().map(|c| if c.is_ascii() { 8 } else { 13 }).sum();
        let _ = cn_font::draw_mixed_text(fb, text, Point::new(center_x - w / 2, y), color, font);
    } else {
        let style = MonoTextStyle::new(font, color);
        let _ = Text::with_alignment(text, Point::new(center_x, y), style, Alignment::Center).draw(fb);
    }
}

fn render_title(fb: &mut Framebuffer, _game: &Game, l: &lang::Lang) {
    let w = fb.width() as i32;
    let h = fb.height() as i32;
    let center_x = w / 2;

    let _ = Rectangle::new(Point::new(0, 0), Size::new(w as u32, 36))
        .draw_styled(&PrimitiveStyle::with_fill(Rgb888::new(0x10, 0x10, 0x22)), fb);

    let title_style = MonoTextStyle::new(&FONT_10X20, COLOR_TITLE);
    let _ = Text::with_alignment("F A R K L E", Point::new(center_x, layout::title_y(h) + 20), title_style, Alignment::Center).draw(fb);
    let subtitle_style = MonoTextStyle::new(&FONT_9X15, COLOR_TEXT);
    let _ = Text::with_alignment("A Dice Game for UEFI", Point::new(center_x, layout::title_y(h) + 55), subtitle_style, Alignment::Center).draw(fb);

    let dice_y = h / 2 - 20;
    for (i, &val) in [1, 3, 5, 2, 6, 4].iter().enumerate() {
        let x = layout::die_x(i, fb.width());
        dice::draw_die(fb, x, dice_y, val, false, false, false);
    }

    draw_text_center(fb, _game, l.title_start, center_x, dice_y + layout::DIE_SIZE + 30, COLOR_TURN_SCORE, &FONT_9X15);
    let lang_style = MonoTextStyle::new(&FONT_7X13, Rgb888::new(0x7B, 0x7B, 0xE0));
    let _ = Text::with_alignment(l.lang_indicator, Point::new(center_x, dice_y + layout::DIE_SIZE + 52), lang_style, Alignment::Center).draw(fb);
    draw_text_center(fb, _game, l.title_ctrl, center_x, h - 20, Rgb888::new(0x98, 0x98, 0xB8), &FONT_7X13);
}

fn render_player_turn(fb: &mut Framebuffer, game: &Game, phase: TurnPhase, l: &lang::Lang) {
    let w = fb.width() as i32;
    let h = fb.height() as i32;
    render_scoreboard(fb, game);

    match phase {
        TurnPhase::ReadyToRoll => {
            let msg = if game.roll_count_this_turn == 0 { l.roll_prompt } else { l.roll_again };
            draw_text_center(fb, game, msg, w / 2, layout::meld_hint_y(h), COLOR_TURN_SCORE, &FONT_9X15);
        }
        TurnPhase::Rolling { frames, frame_count } => {
            dice::draw_roll_animation(fb, game, frames, frame_count);
            draw_text_center(fb, game, l.rolling, w / 2, layout::meld_hint_y(h), COLOR_TURN_SCORE, &FONT_9X15);
        }
        TurnPhase::Selecting => {
            dice::draw_all_dice(fb, game);
            render_turn_info(fb, game, l);
            render_action_buttons(fb, game, l);

            if game.selected_dice.iter().any(|&s| s) {
                let meld_score = game.check_selection_is_valid_meld();
                let (color, msg_text) = if let Some(score) = meld_score {
                    let mut sbuf = FmtBuf::<16>::new();
                    let _ = write!(sbuf, "{}", score);
                    let mut msg = FmtBuf::<64>::new();
                    fmt_replace(&mut msg, l.select_meld, sbuf.as_str(), "", "", "");
                    (COLOR_SELECTED, msg)
                } else {
                    (COLOR_FARKLE, FmtBuf::<64>::new())
                };
                let text = if meld_score.is_some() { msg_text.as_str() } else { l.invalid_meld };
                draw_text_center(fb, game, text, w / 2, layout::flash_msg_y(h), color, &FONT_9X15);
            }
        }
        TurnPhase::Farkle { .. } => {
            dice::draw_all_dice(fb, game);
            draw_text_center(fb, game, l.farkle_msg, w / 2, layout::meld_hint_y(h), COLOR_FARKLE, &FONT_9X15);
        }
        TurnPhase::Banking { .. } => {
            draw_text_center(fb, game, l.banked_msg, w / 2, layout::meld_hint_y(h), COLOR_TURN_SCORE, &FONT_9X15);
        }
    }

    if !game.flash_msg.is_empty() {
        let style = MonoTextStyle::new(&FONT_9X15, COLOR_FARKLE);
        let _ = Text::with_alignment(game.flash_msg, Point::new(w / 2, layout::flash_msg_y(h)), style, Alignment::Center).draw(fb);
    }

    render_help_bar(fb, w, h);
}

fn render_ai_turn_detail(fb: &mut Framebuffer, game: &Game, show_meld: bool, l: &lang::Lang) {
    render_scoreboard(fb, game);
    let w = fb.width() as i32;
    let h = fb.height() as i32;

    if show_meld && !game.ai_meld_name.is_empty() {
        let mut msg = FmtBuf::<64>::new();
        if game.ai_meld_points > 0 {
            let _ = write!(msg, "{}: {} (+{})", game.players[1].name, game.ai_meld_name, game.ai_meld_points);
        } else {
            let _ = write!(msg, "{} Farkled!", game.players[1].name);
        }
        let color = if game.ai_meld_points > 0 { COLOR_BUTTON_BANK } else { COLOR_FARKLE };
        let style = MonoTextStyle::new(&FONT_9X15_BOLD, color);
        let _ = Text::with_alignment(msg.as_str(), Point::new(w / 2, layout::meld_hint_y(h)), style, Alignment::Center).draw(fb);
        draw_dice_with_ai_meld(fb, game);
    } else {
        let mut status = FmtBuf::<64>::new();
        let template = match game.phase {
            GamePhase::AiRolling { .. } => l.ai_roll,
            _ => l.ai_think,
        };
        fmt_replace(&mut status, template, "", "", "", game.players[1].name);
        if status.len > 0 {
            draw_text_center(fb, game, status.as_str(), w / 2, layout::meld_hint_y(h), COLOR_TURN_SCORE, &FONT_9X15);
        }
        dice::draw_all_dice(fb, game);
    }

    if game.turn_score > 0 {
        let style = MonoTextStyle::new(&FONT_9X15, COLOR_TURN_SCORE);
        let mut sbuf = FmtBuf::<16>::new();
        let _ = write!(sbuf, "{}", game.turn_score);
        let mut turn_text = FmtBuf::<64>::new();
        fmt_replace(&mut turn_text, l.ai_turn_score, sbuf.as_str(), "", "", game.players[1].name);
        let _ = Text::with_alignment(turn_text.as_str(), Point::new(w / 2, layout::turn_info_y(h)), style, Alignment::Center).draw(fb);
    }

    let held_count = game.held_dice.iter().filter(|&&h| h).count();
    if held_count > 0 {
        let style = MonoTextStyle::new(&FONT_9X15, COLOR_HELD);
        let mut text = FmtBuf::<64>::new();
        let _ = write!(text, "Held: {} dice", held_count);
        let _ = Text::with_alignment(text.as_str(), Point::new(w / 2, layout::turn_info_y(h) + 20), style, Alignment::Center).draw(fb);
    }
}

fn draw_dice_with_ai_meld(fb: &mut Framebuffer, game: &Game) {
    let h = fb.height() as i32;
    let dy = layout::dice_row_y(h);
    for i in 0..6 {
        let x = layout::die_x(i, fb.width());
        let y = dy;
        let is_held = game.held_dice[i];
        let is_ai_meld = game.ai_meld_dice[i];
        let value = game.dice[i];
        if is_ai_meld && !is_held {
            let s = layout::DIE_SIZE;
            let _ = Rectangle::new(Point::new(x + 2, y + 2), Size::new(s as u32, s as u32)).draw_styled(&PrimitiveStyle::with_fill(Rgb888::new(0x0C, 0x0C, 0x1A)), fb);
            let _ = Rectangle::new(Point::new(x, y), Size::new(s as u32, s as u32)).draw_styled(&PrimitiveStyle::with_fill(COLOR_DICE_FACE), fb);
            let border = PrimitiveStyle::with_stroke(COLOR_BUTTON_BANK, 3);
            let _ = Rectangle::new(Point::new(x, y), Size::new(s as u32, s as u32)).draw_styled(&border, fb);
            if (1..=6).contains(&value) {
                let pip_style = PrimitiveStyle::with_fill(COLOR_DICE_PIP);
                for &(px, py) in layout::pip_positions(value) {
                    let _ = Circle::new(Point::new(x + px, y + py), layout::PIP_RADIUS).draw_styled(&pip_style, fb);
                }
            }
        } else {
            dice::draw_die(fb, x, y, value, is_held, false, false);
        }
    }
}

fn render_game_over(fb: &mut Framebuffer, game: &Game, l: &lang::Lang) {
    let w = fb.width() as i32;
    let h = fb.height() as i32;
    let center_x = w / 2;
    let winner = game.winner.map(|idx| &game.players[idx]);
    let is_tie = game.players[0].total_score == game.players[1].total_score && game.players[0].total_score >= 5000;

    draw_text_center(fb, game, l.game_over, center_x, 50, COLOR_TITLE, &FONT_10X20);
    let win_style = MonoTextStyle::new(&FONT_9X15_BOLD, COLOR_TURN_SCORE);
    if is_tie {
        let _ = Text::with_alignment("It's a Tie!", Point::new(center_x, 90), win_style, Alignment::Center).draw(fb);
    } else if let Some(winner) = winner {
        let mut msg = FmtBuf::<64>::new();
        let _ = write!(msg, "{} Wins!", winner.name);
        let _ = Text::with_alignment(msg.as_str(), Point::new(center_x, 90), win_style, Alignment::Center).draw(fb);
    }
    let score_style = MonoTextStyle::new(&FONT_9X15, COLOR_TEXT);
    for (i, player) in game.players.iter().enumerate() {
        let mut text = FmtBuf::<64>::new();
        let _ = write!(text, "{}: {} points", player.name, player.total_score);
        let _ = Text::with_alignment(text.as_str(), Point::new(center_x, 130 + i as i32 * 24), score_style, Alignment::Center).draw(fb);
    }
    draw_text_center(fb, game, l.press_restart, center_x, h / 2, COLOR_TURN_SCORE, &FONT_9X15);
}

fn render_scoreboard(fb: &mut Framebuffer, game: &Game) {
    let w = fb.width() as i32;
    let h = fb.height() as i32;
    let center_x = w / 2;
    let y = layout::scoreboard_y(h);
    let player_style = MonoTextStyle::new(&FONT_9X15_BOLD, COLOR_TEXT);
    let score_style = MonoTextStyle::new(&FONT_9X15_BOLD, COLOR_TURN_SCORE);
    let small_style = MonoTextStyle::new(&FONT_9X15, COLOR_HELD);
    let p0 = &game.players[0];
    let p1 = &game.players[1];
    let _ = Text::with_alignment(p0.name, Point::new(center_x - 120, y), player_style, Alignment::Center).draw(fb);

    let mut buf = FmtBuf::<32>::new();
    let _ = write!(buf, "{}", p0.total_score);
    let _ = Text::with_alignment(buf.as_str(), Point::new(center_x - 120, y + 20), score_style, Alignment::Center).draw(fb);

    let _ = Text::with_alignment("vs", Point::new(center_x, y + 10), small_style, Alignment::Center).draw(fb);
    let _ = Text::with_alignment(p1.name, Point::new(center_x + 120, y), player_style, Alignment::Center).draw(fb);

    let mut buf = FmtBuf::<32>::new();
    let _ = write!(buf, "{}", p1.total_score);
    let _ = Text::with_alignment(buf.as_str(), Point::new(center_x + 120, y + 20), score_style, Alignment::Center).draw(fb);

    let active_x = if game.current_player == 0 { center_x - 90 } else { center_x + 30 };
    let _ = Rectangle::new(Point::new(active_x, y + 36), Size::new(60, 2)).draw_styled(&PrimitiveStyle::with_fill(COLOR_TURN_SCORE), fb);
}

fn render_turn_info(fb: &mut Framebuffer, game: &Game, l: &lang::Lang) {
    let w = fb.width() as i32;
    let h = fb.height() as i32;
    let center_x = w / 2;
    let y = layout::turn_info_y(h);
    let unheld = (0..6).filter(|&i| !game.held_dice[i]).count();
    let held_count = game.held_dice.iter().filter(|&&h| h).count();

    // Stack-format: score + remaining into tiny fixed buffers
    let mut sbuf = FmtBuf::<16>::new();
    let _ = write!(sbuf, "{}", game.turn_score);
    let mut rbuf = FmtBuf::<16>::new();
    let _ = write!(rbuf, "{}", unheld);

    let mut line1 = FmtBuf::<64>::new();
    fmt_replace(&mut line1, l.turn_fmt, sbuf.as_str(), rbuf.as_str(), "", "");
    draw_text_center(fb, game, line1.as_str(), center_x, y, COLOR_TEXT, &FONT_9X15);

    if held_count > 0 {
        let mut hbuf = FmtBuf::<16>::new();
        let _ = write!(hbuf, "{}", held_count);
        let mut line2 = FmtBuf::<64>::new();
        fmt_replace(&mut line2, l.held_fmt, "", "", hbuf.as_str(), "");
        draw_text_center(fb, game, line2.as_str(), center_x, y + 20, COLOR_HELD, &FONT_9X15);
    }
}

fn render_action_buttons(fb: &mut Framebuffer, game: &Game, l: &lang::Lang) {
    let w = fb.width() as i32;
    let h = fb.height() as i32;
    let total_w = layout::BUTTON_WIDTH * 2 + layout::BUTTON_GAP;
    let start_x = (w - total_w) / 2;
    let y = layout::button_y(h);
    let rx = start_x;
    let _ = Rectangle::new(Point::new(rx, y), Size::new(layout::BUTTON_WIDTH as u32, layout::BUTTON_HEIGHT as u32)).draw_styled(&PrimitiveStyle::with_fill(COLOR_BUTTON_ROLL), fb);
    draw_text_center(fb, game, l.roll_btn, rx + layout::BUTTON_WIDTH / 2, y + 15, COLOR_TEXT, &FONT_9X15_BOLD);
    let bx = start_x + layout::BUTTON_WIDTH + layout::BUTTON_GAP;
    let _ = Rectangle::new(Point::new(bx, y), Size::new(layout::BUTTON_WIDTH as u32, layout::BUTTON_HEIGHT as u32)).draw_styled(&PrimitiveStyle::with_fill(COLOR_BUTTON_BANK), fb);
    draw_text_center(fb, game, l.bank_btn, bx + layout::BUTTON_WIDTH / 2, y + 15, COLOR_TEXT, &FONT_9X15_BOLD);
}

fn render_help_bar(fb: &mut Framebuffer, w: i32, h: i32) {
    let style = MonoTextStyle::new(&FONT_7X13, Rgb888::new(0x70, 0x70, 0x98));
    let _ = Text::with_alignment("[arrows]Move  [Space]Pick  [B]Bank  [R]Roll  [Q]Quit  [L]Lang",
        Point::new(w / 2, layout::help_y(h)), style, Alignment::Center).draw(fb);
}
