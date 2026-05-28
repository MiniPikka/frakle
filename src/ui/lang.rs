// Bilingual labels: EN (ASCII) and CN (Unicode Chinese).
// CN strings are rendered via cn_font::draw_mixed_text.
// EN strings are rendered via embedded-graphics Text.

pub struct Lang<'a> {
    pub roll_btn: &'a str,
    pub bank_btn: &'a str,
    pub roll_prompt: &'a str,
    pub roll_again: &'a str,
    pub rolling: &'a str,
    pub turn_fmt: &'a str,
    pub held_fmt: &'a str,
    pub farkle_msg: &'a str,
    pub banked_msg: &'a str,
    pub invalid_meld: &'a str,
    pub select_meld: &'a str,
    pub title_start: &'a str,
    pub title_ctrl: &'a str,
    pub lang_indicator: &'a str,
    pub game_over: &'a str,
    pub press_restart: &'a str,
    pub ai_think: &'a str,
    pub ai_roll: &'a str,
}

pub const LANG_EN: Lang<'static> = Lang {
    roll_btn: "Roll [R]",
    bank_btn: "Bank [B]",
    roll_prompt: "Press R to Roll",
    roll_again: "[B] Bank  |  [R] Roll again",
    rolling: "Rolling...",
    turn_fmt: "Turn: {s}  |  Rem: {r}",
    held_fmt: "Held: {h} dice",
    farkle_msg: "FARKLE! No points.",
    banked_msg: "Points banked!",
    invalid_meld: "Invalid - need 1s or 5s",
    select_meld: "+{s}  [B]Bank  [R]Roll",
    title_start: "Press ENTER to Start",
    title_ctrl: "Arrows=Move  Space=Pick  B=Bank  R=Roll  Q=Quit  L=Lang",
    lang_indicator: "EN [L]",
    game_over: "G A M E  O V E R",
    press_restart: "Press ENTER to Play Again",
    ai_think: "{n} is thinking...",
    ai_roll: "{n} is rolling...",
};

pub const LANG_CN: Lang<'static> = Lang {
    roll_btn: "\u{63B7}\u{9AB0} [R]",
    bank_btn: "\u{5B58}\u{5206} [B]",
    roll_prompt: "\u{6309} R \u{63B7}\u{9AB0}",
    roll_again: "[B] \u{5B58}\u{5206}  |  [R] \u{518D}\u{63B7}",
    rolling: "\u{63B7}\u{9AB0}\u{4E2D}...",
    turn_fmt: "\u{5206}:{s}  |  \u{5269}:{r}",
    held_fmt: "\u{6301}:{h} \u{4E2A}",
    farkle_msg: "\u{96F6}\u{5206}! (\u{65E0}\u{6548}\u{7EC4}\u{5408})",
    banked_msg: "\u{5DF2}\u{5B58}\u{5206}!",
    invalid_meld: "\u{65E0}\u{6548} - \u{9700} 1 \u{6216} 5",
    select_meld: "+{s}  [B]\u{5B58}\u{5206}  [R]\u{63B7}\u{9AB0}",
    title_start: "\u{6309} ENTER \u{5F00}\u{59CB}",
    title_ctrl: "\u{65B9}\u{5411}=\u{79FB}  \u{7A7A}\u{683C}=\u{9009}  B=\u{5B58}  R=\u{63B7}  Q=\u{9000}  L=EN",
    lang_indicator: "\u{4E2D}\u{6587} [L]",
    game_over: "\u{6E38}\u{620F}\u{7ED3}\u{675F}",
    press_restart: "\u{6309} ENTER \u{91CD}\u{6765}",
    ai_think: "{n} \u{601D}\u{8003}\u{4E2D}...",
    ai_roll: "{n} \u{63B7}\u{9AB0}\u{4E2D}...",
};

pub fn lang(cn_mode: bool) -> &'static Lang<'static> {
    if cn_mode { &LANG_CN } else { &LANG_EN }
}
