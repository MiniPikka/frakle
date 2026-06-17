// Layout that scales with screen resolution.
// Base design: 480px height. All sizes scale proportionally.

const BASE_H: i32 = 480;

pub struct Layout {
    pub die_size: i32,
    pub die_gap: i32,
    pub pip_radius: u32,
    pub button_w: i32,
    pub button_h: i32,
    pub button_gap: i32,
    pub screen_w: i32,
    pub screen_h: i32,
}

impl Layout {
    pub fn new(screen_w: usize, screen_h: usize) -> Self {
        let h = screen_h as i32;
        let s = |base: i32| -> i32 { (base * h.max(BASE_H) + BASE_H / 2) / BASE_H };
        Self {
            die_size: s(70),
            die_gap: s(12),
            pip_radius: s(7) as u32,
            button_w: s(160),
            button_h: s(50),
            button_gap: s(40),
            screen_w: screen_w as i32,
            screen_h: h,
        }
    }

    pub fn dice_start_x(&self) -> i32 {
        let total = 6 * self.die_size + 5 * self.die_gap;
        (self.screen_w - total) / 2
    }
    pub fn die_x(&self, index: usize) -> i32 {
        self.dice_start_x() + index as i32 * (self.die_size + self.die_gap)
    }
    pub fn title_y(&self) -> i32 { self.screen_h / 24 }
    pub fn scoreboard_y(&self) -> i32 { self.screen_h / 10 }
    pub fn dice_row_y(&self) -> i32 { self.screen_h / 4 + self.scale(15) }
    pub fn turn_info_y(&self) -> i32 { self.screen_h * 5 / 9 }
    pub fn meld_hint_y(&self) -> i32 { self.screen_h * 6 / 9 }
    pub fn flash_msg_y(&self) -> i32 { self.screen_h * 7 / 9 - self.scale(5) }
    pub fn button_y(&self) -> i32 { self.screen_h * 7 / 9 + self.scale(10) }
    pub fn help_y(&self) -> i32 { self.screen_h - self.scale(25) }

    fn scale(&self, base: i32) -> i32 {
        (base * self.screen_h.max(BASE_H) + BASE_H / 2) / BASE_H
    }

    pub fn pip_positions(&self, value: u8) -> [(i32, i32); 6] {
        let ds = self.die_size;
        let r = |v: i32| v * ds / 70;
        match value {
            1 => [(r(35), r(35)), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)],
            2 => [(r(18), r(52)), (r(52), r(18)), (0, 0), (0, 0), (0, 0), (0, 0)],
            3 => [(r(18), r(52)), (r(35), r(35)), (r(52), r(18)), (0, 0), (0, 0), (0, 0)],
            4 => [(r(18), r(18)), (r(52), r(18)), (r(18), r(52)), (r(52), r(52)), (0, 0), (0, 0)],
            5 => [(r(18), r(18)), (r(52), r(18)), (r(35), r(35)), (r(18), r(52)), (r(52), r(52)), (0, 0)],
            6 => [(r(18), r(18)), (r(35), r(18)), (r(52), r(18)), (r(18), r(52)), (r(35), r(52)), (r(52), r(52))],
            _ => [(0, 0); 6],
        }
    }

    pub fn pip_count(&self, value: u8) -> usize {
        match value { 1 => 1, 2 => 2, 3 => 3, 4 => 4, 5 => 5, 6 => 6, _ => 0 }
    }
}

// Backward-compatible constants (for code that doesn't have Layout yet)
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
