/// Collect dice values matching a predicate into a fixed-size array.
/// Returns (array, count). At most 6 elements.
fn collect_dice<F: Fn(usize) -> bool>(dice: &[u8; 6], pred: F) -> ([u8; 6], usize) {
    let mut out = [0u8; 6];
    let mut n = 0;
    for (i, &die) in dice.iter().enumerate() {
        if pred(i) { out[n] = die; n += 1; }
    }
    (out, n)
}

/// Safe index into the 2-player array — clamps to [0, 1].
/// Prevents out-of-bounds access if state is ever corrupted.
#[inline]
fn player_idx(idx: usize) -> usize {
    idx & 1
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GamePhase {
    Title,
    PlayerTurn(TurnPhase),
    AiTurn,
    AiShowMeld { frames: u32 },
    AiRolling { frames: u32 },
    AiSelecting { frames: u32 },
    GameOver,
    Quit,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TurnPhase {
    ReadyToRoll,
    Rolling { frames: u32, frame_count: u32 },
    Selecting,
    Farkle { frames: u32 },
    Banking { frames: u32 },
}

#[derive(Clone)]
pub struct Player {
    pub name: &'static str,
    pub total_score: u32,
}

#[derive(Clone)]
pub struct Game {
    pub phase: GamePhase,
    pub players: [Player; 2],
    pub current_player: usize,
    pub dice: [u8; 6],
    pub held_dice: [bool; 6],
    pub turn_score: u32,
    pub selected_dice: [bool; 6],
    pub cursor: usize,
    pub roll_count_this_turn: u32,
    pub ai_select_frame: u32,
    pub final_turn: Option<usize>,
    pub winner: Option<usize>,
    pub flash_msg: &'static str,
    pub flash_frames: u32,
    pub ai_meld_dice: [bool; 6],
    pub ai_meld_name: &'static str,
    pub ai_meld_points: u32,
    pub lang_cn: bool,
    /// Animated display scores — smoothly count toward actual totals.
    pub display_scores: [u32; 2],
    /// Title text breathing brightness (0.6..1.0), driven by Effects.
    pub title_breathe: f32,

    // ── Gameplay-driven visual state ──
    /// Consecutive successful banks without farkling (combo streak).
    pub combo_streak: u32,
    /// Last meld description for floating text display.
    pub last_meld_name: &'static str,
    /// Last meld points.
    pub last_meld_points: u32,
    /// Frames remaining to show the meld floating text.
    pub meld_display_frames: u32,
    /// Score milestones already triggered (1000, 2000, 3000, 4000).
    pub milestones_hit: [bool; 4],
}

#[derive(Clone)]
pub struct MeldInfo {
    pub indices: [usize; 6],
    pub indices_len: usize,
    pub score: u32,
    pub description: &'static str,
}

pub struct MeldList {
    pub items: [MeldInfo; 16],
    pub len: usize,
}

impl MeldList {
    fn new() -> Self {
        Self { items: [MELD_EMPTY; 16], len: 0 }
    }
    fn push(&mut self, m: MeldInfo) {
        if self.len < 16 {
            self.items[self.len] = m;
            self.len += 1;
        }
    }
    fn iter(&self) -> impl Iterator<Item = &MeldInfo> {
        self.items[..self.len].iter()
    }
    fn is_empty(&self) -> bool { self.len == 0 }
}

const MELD_EMPTY: MeldInfo = MeldInfo {
    indices: [0; 6], indices_len: 0, score: 0, description: "",
};

impl Game {
    pub fn new() -> Self {
        Self {
            phase: GamePhase::Title,
            players: [
                Player { name: "You", total_score: 0 },
                Player { name: "Lucky", total_score: 0 },
            ],
            current_player: 0,
            dice: [0; 6], held_dice: [false; 6], turn_score: 0,
            selected_dice: [false; 6], cursor: 0, roll_count_this_turn: 0,
            ai_select_frame: 0, final_turn: None, winner: None,
            flash_msg: "", flash_frames: 0,
            ai_meld_dice: [false; 6], ai_meld_name: "", ai_meld_points: 0,
            lang_cn: false,
            display_scores: [0, 0],
            title_breathe: 1.0,
            combo_streak: 0,
            last_meld_name: "",
            last_meld_points: 0,
            meld_display_frames: 0,
            milestones_hit: [false; 4],
        }
    }

    /// Safe access to the current player — uses `player_idx` to prevent OOB.
    pub fn current_player(&self) -> &Player {
        &self.players[player_idx(self.current_player)]
    }

    pub fn current_player_mut(&mut self) -> &mut Player {
        &mut self.players[player_idx(self.current_player)]
    }

    pub fn roll_dice(&mut self, rng: &mut impl FnMut() -> u8) {
        for i in 0..6 {
            if !self.held_dice[i] { self.dice[i] = rng(); }
        }
        self.roll_count_this_turn += 1;
    }

    pub fn reset_held_if_all_scored(&mut self) {
        if self.held_dice.iter().all(|&h| h) { self.held_dice = [false; 6]; }
    }

    pub fn is_farkle(&self) -> bool {
        let (unheld, n) = collect_dice(&self.dice, |i| !self.held_dice[i]);
        find_all_melds(&unheld[..n]).is_empty()
    }

    pub fn check_selection_is_valid_meld(&self) -> Option<u32> {
        let (selected_vals, n) = collect_dice(&self.dice, |i| self.selected_dice[i] && !self.held_dice[i]);
        if n == 0 { return None; }
        find_meld_score(&selected_vals[..n]).or_else(|| {
            let (all_unheld, m) = collect_dice(&self.dice, |i| !self.held_dice[i]);
            find_meld_score(&all_unheld[..m]).filter(|_| (0..6).all(|i| self.held_dice[i] || self.selected_dice[i]))
        })
    }

    pub fn apply_selection(&mut self) {
        let score = self.check_selection_is_valid_meld();
        for i in 0..6 {
            if self.selected_dice[i] && !self.held_dice[i] { self.held_dice[i] = true; }
        }
        if let Some(s) = score { self.turn_score += s; }
        self.selected_dice = [false; 6];
        self.reset_held_if_all_scored();
    }

    pub fn bank_score(&mut self) -> u32 {
        let scored = self.turn_score;
        self.current_player_mut().total_score += scored;
        scored
    }

    pub fn end_turn(&mut self) {
        self.turn_score = 0; self.held_dice = [false; 6];
        self.selected_dice = [false; 6]; self.cursor = 0;
        self.roll_count_this_turn = 0; self.dice = [0; 6];
    }

    pub fn switch_player(&mut self) {
        self.end_turn();
        self.current_player = 1 - self.current_player;
    }

    pub fn check_game_over(&mut self) -> Option<usize> {
        let cp = player_idx(self.current_player);
        let other_player = 1 - cp;
        let this_score = self.players[cp].total_score;
        let other_score = self.players[other_player].total_score;
        if this_score >= 5000 {
            match self.final_turn {
                None => { self.final_turn = Some(self.current_player); None }
                Some(trigger) => {
                    if trigger != self.current_player {
                        Some(if this_score > other_score { self.current_player } else { other_player })
                    } else { Some(self.current_player) }
                }
            }
        } else {
            self.final_turn.map(|trigger| {
                if other_score > this_score { other_player } else { trigger }
            })
        }
    }
}

impl Default for Game {
    fn default() -> Self { Self::new() }
}

pub fn ai_decide(game: &Game) -> AiAction {
    // Collect unheld dice values AND their original positions in game.dice
    let mut unheld_vals = [0u8; 6];
    let mut orig_pos = [0usize; 6];
    let mut n = 0;
    for i in 0..6 {
        if !game.held_dice[i] {
            unheld_vals[n] = game.dice[i];
            orig_pos[n] = i;
            n += 1;
        }
    }
    if n == 0 { return AiAction::Farkle; }

    let all_melds = find_all_melds(&unheld_vals[..n]);
    if all_melds.is_empty() { return AiAction::Farkle; }

    // Find the best non-overlapping combination of melds
    let (best_set, best_len, total_score) = find_best_meld_combo(&all_melds);

    // Remap indices from collected-array positions to original game.dice positions
    let mut combined_indices = [0usize; 6];
    let mut combined_len = 0;
    for &meld_idx in &best_set[..best_len] {
        let meld = &all_melds.items[meld_idx];
        for &local_idx in &meld.indices[..meld.indices_len] {
            combined_indices[combined_len] = orig_pos[local_idx];
            combined_len += 1;
        }
    }

    let score_after = game.turn_score + total_score;
    let remaining = n - combined_len;
    let should_bank = game.turn_score > 0
        && (score_after >= 500 || remaining <= 1
            || game.current_player().total_score + score_after >= 5000
            || (remaining == 0 && game.turn_score > 300));

    let meld_info = MeldInfo {
        indices: combined_indices,
        indices_len: combined_len,
        score: total_score,
        description: "Meld",
    };

    if should_bank { AiAction::BankAfterMeld(meld_info) }
    else { AiAction::Roll(meld_info) }
}

pub enum AiAction {
    Roll(MeldInfo), BankAfterMeld(MeldInfo), Farkle,
}

pub fn find_all_melds(dice: &[u8]) -> MeldList {
    let n = dice.len();
    if n == 0 { return MeldList::new(); }
    let mut melds = MeldList::new();
    let mut counts = [0usize; 7];
    for &d in dice { if (1..=6).contains(&d) { counts[d as usize] += 1; } }

    if counts[1..=6].iter().all(|&c| c == 1) {
        let mut indices = [0usize; 6];
        for (i, v) in indices.iter_mut().enumerate() { *v = i; }
        melds.push(MeldInfo { indices, indices_len: n, score: 1500, description: "1-6 Straight" });
    }
    if counts.iter().filter(|&&c| c == 2).count() == 3 {
        let mut indices = [0usize; 6];
        for (i, v) in indices.iter_mut().enumerate() { *v = i; }
        melds.push(MeldInfo { indices, indices_len: n, score: 1500, description: "Three Pairs" });
    }

    let mut triplet_vals = [0usize; 2];
    let mut triplet_count = 0;
    for (v, &count) in counts.iter().enumerate().skip(1) {
        if triplet_count < 2 && count >= 3 {
            triplet_vals[triplet_count] = v;
            triplet_count += 1;
        }
    }
    if triplet_count >= 2 {
        let (v1, v2) = (triplet_vals[0], triplet_vals[1]);
        let mut indices = [0usize; 6];
        let (mut idx, mut c1, mut c2) = (0, 0, 0);
        for (i, &d) in dice.iter().enumerate() {
            if d as usize == v1 && c1 < 3 { indices[idx] = i; idx += 1; c1 += 1; }
            else if d as usize == v2 && c2 < 3 { indices[idx] = i; idx += 1; c2 += 1; }
        }
        melds.push(MeldInfo { indices, indices_len: 6, score: 2500, description: "Two Triplets" });
    }

    for (v, &cnt) in counts.iter().enumerate().skip(1) {
        let make_indices = |dice: &[u8], v: usize, cnt: usize| -> ([usize; 6], usize) {
            let mut indices = [0usize; 6];
            let mut idx = 0;
            for (i, &d) in dice.iter().enumerate() {
                if d as usize == v && idx < cnt { indices[idx] = i; idx += 1; }
            }
            (indices, cnt)
        };
        match cnt {
            6 => {
                let mut indices = [0usize; 6];
                for (i, v) in indices.iter_mut().enumerate() { *v = i; }
                melds.push(MeldInfo { indices, indices_len: 6, score: 3000, description: "Six of a kind" });
            }
            5 => {
                let (indices, len) = make_indices(dice, v, 5);
                melds.push(MeldInfo { indices, indices_len: len, score: 2000, description: "Five of a kind" });
            }
            4 => {
                let (indices, len) = make_indices(dice, v, 4);
                melds.push(MeldInfo { indices, indices_len: len, score: 1000, description: "Four of a kind" });
            }
            3 => {
                let (indices, len) = make_indices(dice, v, 3);
                let score = if v == 1 { 1000 } else { v as u32 * 100 };
                let desc = match v {
                    1 => "Three 1s", 2 => "Three 2s", 3 => "Three 3s",
                    4 => "Three 4s", 5 => "Three 5s", _ => "Three 6s",
                };
                melds.push(MeldInfo { indices, indices_len: len, score, description: desc });
            }
            _ => {}
        }
    }

    // Build a bitmask of dice positions already consumed by higher melds.
    // Each bit represents one dice index (0–5), avoiding the O(n²) linear
    // scan that previously called `melds.iter().any(|m| ...contains(&i))`.
    let mut used_mask: u8 = 0;
    for m in melds.iter() {
        for &idx in &m.indices[..m.indices_len] {
            used_mask |= 1 << idx;
        }
    }

    for v in [1, 5] {
        if counts[v] > 0 && counts[v] < 3 {
            let score = if v == 1 { 100 } else { 50 };
            for (i, &d) in dice.iter().enumerate() {
                if d as usize == v && (used_mask & (1 << i)) == 0 {
                    let mut indices = [0usize; 6]; indices[0] = i;
                    melds.push(MeldInfo { indices, indices_len: 1, score,
                        description: if v == 1 { "Single 1" } else { "Single 5" } });
                }
            }
        }
    }
    melds
}

/// State for the recursive meld-search.
struct MeldSearch {
    best_set: [usize; 16],
    best_len: usize,
    best_score: u32,
    cur_set: [usize; 16],
}

/// Find the best (highest-scoring) non-overlapping combination of melds.
/// Returns (meld_indices, count, total_score).
fn find_best_meld_combo(melds: &MeldList) -> ([usize; 16], usize, u32) {
    let mut state = MeldSearch {
        best_set: [0; 16],
        best_len: 0,
        best_score: 0,
        cur_set: [0; 16],
    };
    search_melds(&melds.items[..melds.len], 0, 0u8, 0, 0, &mut state);
    (state.best_set, state.best_len, state.best_score)
}

/// Recursive branch-and-bound search for the best non-overlapping meld set.
fn search_melds(
    melds: &[MeldInfo],
    start: usize,
    used: u8,
    cur_len: usize,
    cur_score: u32,
    state: &mut MeldSearch,
) {
    if cur_score > state.best_score {
        state.best_score = cur_score;
        state.best_set[..cur_len].copy_from_slice(&state.cur_set[..cur_len]);
        state.best_len = cur_len;
    }
    for i in start..melds.len() {
        let meld = &melds[i];
        let mut mask = 0u8;
        for &idx in &meld.indices[..meld.indices_len] {
            mask |= 1 << idx;
        }
        if used & mask == 0 {
            state.cur_set[cur_len] = i;
            search_melds(melds, i + 1, used | mask, cur_len + 1, cur_score + meld.score, state);
        }
    }
}

pub fn find_meld_score(dice: &[u8]) -> Option<u32> {
    let melds = find_all_melds(dice);
    if melds.is_empty() { None } else {
        let (_, _, score) = find_best_meld_combo(&melds);
        Some(score)
    }
}
