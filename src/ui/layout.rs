// Layout functions that scale with screen height.
// Designed for 480px minimum, scales up proportionally.

pub const DIE_SIZE: i32 = 70;
pub const DIE_GAP: i32 = 12;
pub const PIP_RADIUS: u32 = 7;
pub const BUTTON_WIDTH: i32 = 160;
pub const BUTTON_HEIGHT: i32 = 50;
pub const BUTTON_GAP: i32 = 40;

pub fn dice_start_x(screen_width: usize) -> i32 {
    let total_width = 6 * DIE_SIZE + 5 * DIE_GAP;
    ((screen_width as i32) - total_width) / 2
}
pub fn die_x(index: usize, screen_width: usize) -> i32 {
    dice_start_x(screen_width) + index as i32 * (DIE_SIZE + DIE_GAP)
}

pub fn title_y(h: i32) -> i32 { h / 24 }
pub fn scoreboard_y(h: i32) -> i32 { h / 10 }
pub fn dice_row_y(h: i32) -> i32 { h / 4 + 15 }
pub fn turn_info_y(h: i32) -> i32 { h * 5 / 9 }
pub fn meld_hint_y(h: i32) -> i32 { h * 6 / 9 }
pub fn flash_msg_y(h: i32) -> i32 { h * 7 / 9 - 5 }
pub fn button_y(h: i32) -> i32 { h * 7 / 9 + 10 }
pub fn help_y(h: i32) -> i32 { h - 25 }

pub fn pip_positions(value: u8) -> &'static [(i32, i32)] {
    match value {
        1 => &[(35, 35)],
        2 => &[(18, 52), (52, 18)],
        3 => &[(18, 52), (35, 35), (52, 18)],
        4 => &[(18, 18), (52, 18), (18, 52), (52, 52)],
        5 => &[(18, 18), (52, 18), (35, 35), (18, 52), (52, 52)],
        6 => &[(18, 18), (35, 18), (52, 18), (18, 52), (35, 52), (52, 52)],
        _ => &[],
    }
}
