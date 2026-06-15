#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

extern crate alloc;

use core::fmt::Write;
use uefi::prelude::*;
use uefi::proto::console::gop::GraphicsOutput;
use uefi::proto::console::text::Input;
use embedded_graphics::pixelcolor::Rgb888;

use frakle::framebuffer::Framebuffer;
use frakle::game::{Game, GamePhase, TurnPhase, ai_decide, AiAction};
use frakle::input::{poll_input, GameInput};
use frakle::ui::render;
use frakle::effects::Effects;
use frakle::sound::SoundQueue;
use frakle::sound;
use frakle::logger;
use frakle::FmtBuf;

const FRAME_DELAY_US: u64 = 16_000;
const PHASE_TIMEOUT_FRAMES: u32 = 600; // ~10 seconds at 60fps

struct SimpleRng { state: u64 }

impl SimpleRng {
    fn new(seed: u64) -> Self { Self { state: seed } }
    fn next(&mut self) -> u32 {
        self.state ^= self.state >> 12;
        self.state ^= self.state << 25;
        self.state ^= self.state >> 27;
        (self.state.wrapping_mul(0x2545F4914F6CDD1D) >> 32) as u32
    }
    fn next_die(&mut self) -> u8 { (self.next() % 6 + 1) as u8 }
}

/// Short name for game phase (for debug overlay).
fn phase_short(phase: &GamePhase) -> &'static str {
    match phase {
        GamePhase::Title => "Title",
        GamePhase::PlayerTurn(tp) => match tp {
            TurnPhase::ReadyToRoll => "P:Roll?",
            TurnPhase::Rolling { .. } => "P:Roll!",
            TurnPhase::Selecting => "P:Select",
            TurnPhase::Farkle { .. } => "P:Farkle",
            TurnPhase::Banking { .. } => "P:Bank",
        },
        GamePhase::AiTurn => "AI:Think",
        GamePhase::AiShowMeld { .. } => "AI:Show",
        GamePhase::AiRolling { .. } => "AI:Roll!",
        GamePhase::AiSelecting { .. } => "AI:Wait",
        GamePhase::GameOver => "GameOver",
        GamePhase::Quit => "QUIT",
    }
}

#[entry]
fn main(image: Handle, mut system_table: SystemTable<Boot>) -> Status {
    uefi::helpers::init(&mut system_table).unwrap();

    let bs = system_table.boot_services();

    // Initialize logging (continue even if it fails)
    let _ = logger::init(image, bs);

    let gop_handle = bs.get_handle_for_protocol::<GraphicsOutput>().unwrap();
    let mut gop = bs.open_protocol_exclusive::<GraphicsOutput>(gop_handle).unwrap();
    let (width, height) = gop.current_mode_info().resolution();
    let mut fb = Framebuffer::new(width, height);

    let input_handle = bs.get_handle_for_protocol::<Input>().unwrap();
    let mut input = bs.open_protocol_exclusive::<Input>(input_handle).unwrap();

    // Use CPU timestamp counter as seed — different every boot
    let seed = unsafe { core::arch::x86_64::_rdtsc() };
    let mut game = Game::new();
    let mut rng = SimpleRng::new(seed);
    let mut effects = Effects::new(width as i32, height as i32);
    let mut snd = SoundQueue::new();

    logger::log(image, bs, "Game objects initialized");
    logger::log_memory_usage(image, bs, core::mem::size_of::<Game>());

    let mut frame_count: u32 = 0;
    let mut phase_frames: u32 = 0;
    let mut prev_phase: GamePhase = game.phase;
    let mut max_consecutive: u32 = 0;

    loop {
        frame_count += 1;
        phase_frames += 1;

        // Phase timeout watchdog — flash red warning
        let stuck = phase_frames > PHASE_TIMEOUT_FRAMES;
        if phase_frames > max_consecutive {
            max_consecutive = phase_frames;
        }

        let key = poll_input(&mut input);

        let has_animation = match game.phase {
            GamePhase::PlayerTurn(TurnPhase::Rolling { .. }) => true,
            GamePhase::AiShowMeld { .. } | GamePhase::AiRolling { .. } | GamePhase::AiSelecting { .. } => true,
            GamePhase::GameOver => effects.particle_count > 0,
            _ => false,
        };
        if matches!(key, GameInput::None) && !has_animation && game.flash_frames == 0 {
            bs.stall((FRAME_DELAY_US * 4) as usize);
        }

        process(&mut game, key, &mut rng, &mut effects, &mut snd);

        // Log game over
        if matches!(game.phase, GamePhase::GameOver) && !matches!(prev_phase, GamePhase::GameOver) {
            let w = game.winner.unwrap_or(0);
            let mut gb = FmtBuf::<64>::new();
            let _ = write!(gb, "GAME OVER! Winner=P{} P0:{} P1:{}",
                w, game.players[0].total_score, game.players[1].total_score);
            logger::log(image, bs, gb.as_str());
        }

        // Track phase changes for watchdog + log
        if game.phase != prev_phase {
            let mut pb = FmtBuf::<32>::new();
            let _ = write!(pb, "{}", phase_short(&game.phase));
            logger::log_game_state(image, bs, pb.as_str(),
                game.current_player,
                game.players[game.current_player].total_score,
                game.turn_score, &game.dice, &game.held_dice);
            prev_phase = game.phase;
            phase_frames = 0;
        }

        // Periodic heartbeat
        if frame_count.is_multiple_of(600) {
            let mut hb = FmtBuf::<48>::new();
            let _ = write!(hb, "Heartbeat F:{} phase={}", frame_count, phase_short(&game.phase));
            logger::log(image, bs, hb.as_str());
        }

        if game.flash_frames > 0 {
            game.flash_frames -= 1;
            if game.flash_frames == 0 { game.flash_msg = ""; }
        }

        effects.tick();
        snd.tick();
        render(&mut fb, &game);
        effects.render(&mut fb);

        // ── Debug overlay (top-left) ──
        // Frame + phase + scores + watchdog status — all stack-allocated
        let mut overlay = FmtBuf::<128>::new();
        let _ = write!(overlay, "F:{} {}", frame_count, phase_short(&game.phase));
        fb.draw_text_small(10, 10, overlay.as_str(), Rgb888::new(0, 255, 0));

        let mut score_buf = FmtBuf::<64>::new();
        let _ = write!(score_buf, "P0:{} P1:{} T:{}",
            game.players[0].total_score,
            game.players[1].total_score,
            game.turn_score
        );
        fb.draw_text_small(10, 20, score_buf.as_str(), Rgb888::new(255, 255, 0));

        // Watchdog status line
        if stuck {
            let mut wd = FmtBuf::<64>::new();
            let _ = write!(wd, "STUCK! {}f max:{}", phase_frames, max_consecutive);
            fb.draw_text_small(10, 32, wd.as_str(), Rgb888::new(255, 0, 0));
        }

        fb.present(&mut gop);
        bs.stall(FRAME_DELAY_US as usize);
    }
}

// ── Phase dispatcher ──

fn process(
    game: &mut Game, key: GameInput,
    rng: &mut SimpleRng,
    fx: &mut Effects, snd: &mut SoundQueue,
) {
    match game.phase {
        GamePhase::Title                   => handle_title(game, key),
        GamePhase::PlayerTurn(tp)          => handle_player_turn(game, key, tp, rng, fx, snd),
        GamePhase::AiTurn                  => handle_ai_decide(game),
        GamePhase::AiShowMeld { frames }   => handle_ai_show_meld(game, frames, rng),
        GamePhase::AiRolling { frames }    => handle_ai_rolling(game, frames, rng),
        GamePhase::AiSelecting { frames }  => handle_ai_selecting(game, frames, fx, snd),
        GamePhase::GameOver                => handle_game_over(game, key),
        GamePhase::Quit => {}
    }
}

// ── Title / Game Over ──

fn handle_title(game: &mut Game, key: GameInput) {
    if matches!(key, GameInput::Confirm) {
        *game = Game::new();
        game.phase = GamePhase::PlayerTurn(TurnPhase::ReadyToRoll);
    }
    if matches!(key, GameInput::LangToggle) { game.lang_cn = !game.lang_cn; }
    if matches!(key, GameInput::Quit | GameInput::Escape) { game.phase = GamePhase::Quit; }
}

fn handle_game_over(game: &mut Game, key: GameInput) {
    if matches!(key, GameInput::LangToggle) { game.lang_cn = !game.lang_cn; }
    if matches!(key, GameInput::Confirm) { *game = Game::new(); game.phase = GamePhase::Title; }
    if matches!(key, GameInput::Quit) { game.phase = GamePhase::Quit; }
}

// ── Player turn sub-phases ──

fn handle_player_turn(
    game: &mut Game, key: GameInput, tp: TurnPhase,
    rng: &mut SimpleRng, fx: &mut Effects, snd: &mut SoundQueue,
) {
    if matches!(key, GameInput::LangToggle) { game.lang_cn = !game.lang_cn; }
    match tp {
        TurnPhase::ReadyToRoll           => handle_ready_to_roll(game, key, rng),
        TurnPhase::Rolling { frames, frame_count } => handle_rolling(game, frames, frame_count, rng, fx, snd),
        TurnPhase::Selecting             => handle_selecting(game, key, rng, fx, snd),
        TurnPhase::Farkle { frames } => {
            if frames >= 30 {
                handle_player_turn_end(game, rng, fx, snd);
            } else {
                game.phase = GamePhase::PlayerTurn(TurnPhase::Farkle { frames: frames + 1 });
            }
        }
        TurnPhase::Banking { frames } => {
            if frames >= 20 {
                handle_player_turn_end(game, rng, fx, snd);
            } else {
                game.phase = GamePhase::PlayerTurn(TurnPhase::Banking { frames: frames + 1 });
            }
        }
    }
}

fn handle_ready_to_roll(game: &mut Game, key: GameInput, rng: &mut SimpleRng) {
    match key {
        GameInput::Bank => {
            if game.turn_score > 0 {
                game.bank_score();
                game.phase = GamePhase::PlayerTurn(TurnPhase::Banking { frames: 0 });
            } else { flash(game, "No score to bank yet!"); }
        }
        GameInput::Roll => {
            game.roll_dice(&mut || rng.next_die());
            game.phase = GamePhase::PlayerTurn(TurnPhase::Rolling { frames: 0, frame_count: 15 });
        }
        GameInput::Quit => game.phase = GamePhase::Title,
        _ => {}
    }
}

fn handle_rolling(
    game: &mut Game, frames: u32, frame_count: u32,
    rng: &mut SimpleRng, fx: &mut Effects, snd: &mut SoundQueue,
) {
    if frames >= frame_count {
        if game.is_farkle() {
            game.turn_score = 0;
            fx.spawn_farkle(fx.center_x(), fx.center_y());
            snd.play(sound::SND_FARKLE);
            game.phase = GamePhase::PlayerTurn(TurnPhase::Farkle { frames: 0 });
        } else {
            game.selected_dice = [false; 6];
            game.cursor = 0;
            game.phase = GamePhase::PlayerTurn(TurnPhase::Selecting);
        }
    } else {
        roll_animate_dice(game, rng);
        game.phase = GamePhase::PlayerTurn(TurnPhase::Rolling { frames: frames + 1, frame_count });
    }
}

fn handle_selecting(
    game: &mut Game, key: GameInput,
    rng: &mut SimpleRng, fx: &mut Effects, snd: &mut SoundQueue,
) {
    match key {
        GameInput::Left  => game.cursor = if game.cursor == 0 { 5 } else { game.cursor - 1 },
        GameInput::Right => game.cursor = if game.cursor == 5 { 0 } else { game.cursor + 1 },
        GameInput::Select if !game.held_dice[game.cursor] => {
            game.selected_dice[game.cursor] = !game.selected_dice[game.cursor];
        }
        GameInput::Bank => {
            let has_valid = game.check_selection_is_valid_meld().is_some();
            if has_valid {
                game.apply_selection();
                let scored = game.turn_score;
                game.bank_score();
                fx.spawn_score_pop(fx.center_x(), fx.center_y(), scored);
                snd.play(sound::SND_BANK);
                game.phase = GamePhase::PlayerTurn(TurnPhase::Banking { frames: 0 });
            } else if game.turn_score > 0 {
                game.bank_score();
                snd.play(sound::SND_BANK);
                game.phase = GamePhase::PlayerTurn(TurnPhase::Banking { frames: 0 });
            } else {
                flash(game, "Select scoring dice (1s/5s), then B or R");
            }
        }
        GameInput::Roll => {
            if game.check_selection_is_valid_meld().is_some() {
                game.apply_selection();
                if game.held_dice.iter().all(|&h| h) { game.held_dice = [false; 6]; }
                fx.spawn_score_pop(fx.center_x(), fx.center_y(), game.turn_score);
                game.roll_dice(&mut || rng.next_die());
                snd.play(sound::SND_ROLL);
                game.phase = GamePhase::PlayerTurn(TurnPhase::Rolling { frames: 0, frame_count: 30 });
            } else {
                flash(game, "Select scoring dice (1s/5s), then R");
            }
        }
        GameInput::Quit => game.phase = GamePhase::Title,
        _ => {}
    }
}

// ── AI phases ──

fn handle_ai_decide(game: &mut Game) {
    let action = ai_decide(game);
    match &action {
        AiAction::Roll(meld) | AiAction::BankAfterMeld(meld) => {
            game.ai_meld_dice = [false; 6];
            for &idx in &meld.indices[..meld.indices_len] { game.ai_meld_dice[idx] = true; }
            game.ai_meld_name = meld.description;
            game.ai_meld_points = meld.score;
        }
        AiAction::Farkle => {
            game.ai_meld_dice = [false; 6];
            game.ai_meld_name = "FARKLE!";
            game.ai_meld_points = 0;
        }
    }
    game.ai_select_frame = match &action {
        AiAction::Roll(_) => 0, AiAction::BankAfterMeld(_) => 1, AiAction::Farkle => 2,
    };
    game.phase = GamePhase::AiShowMeld { frames: 0 };
}

fn handle_ai_show_meld(game: &mut Game, frames: u32, rng: &mut SimpleRng) {
    if frames >= 40 {
        match game.ai_select_frame {
            0 => {
                game.held_dice = game.ai_meld_dice;
                game.turn_score += game.ai_meld_points;
                game.ai_meld_dice = [false; 6];
                game.roll_dice(&mut || rng.next_die());
                game.phase = GamePhase::AiRolling { frames: 0 };
            }
            1 => {
                game.held_dice = game.ai_meld_dice;
                game.turn_score += game.ai_meld_points;
                game.bank_score();
                game.ai_meld_dice = [false; 6];
                game.phase = GamePhase::AiSelecting { frames: 0 };
            }
            _ => {
                game.turn_score = 0;
                game.phase = GamePhase::AiSelecting { frames: 0 };
            }
        }
    } else {
        game.phase = GamePhase::AiShowMeld { frames: frames + 1 };
    }
}

fn handle_ai_rolling(game: &mut Game, frames: u32, rng: &mut SimpleRng) {
    if frames >= 30 {
        if game.is_farkle() {
            game.turn_score = 0;
            game.ai_meld_name = "FARKLE!";
            game.ai_meld_points = 0;
            game.phase = GamePhase::AiSelecting { frames: 0 };
        } else {
            game.phase = GamePhase::AiTurn;
        }
    } else {
        roll_animate_dice(game, rng);
        game.phase = GamePhase::AiRolling { frames: frames + 1 };
    }
}

fn handle_ai_selecting(game: &mut Game, frames: u32, fx: &mut Effects, snd: &mut SoundQueue) {
    if frames >= 60 {
        handle_ai_turn_end(game, fx, snd);
    } else {
        game.phase = GamePhase::AiSelecting { frames: frames + 1 };
    }
}

// ── Turn end / helpers ──

fn handle_player_turn_end(game: &mut Game, rng: &mut SimpleRng, fx: &mut Effects, snd: &mut SoundQueue) {
    game.end_turn();
    if let Some(winner) = game.check_game_over() {
        game.winner = Some(winner);
        fx.spawn_victory();
        snd.play(sound::SND_VICTORY);
        game.phase = GamePhase::GameOver;
    } else {
        game.switch_player();
        start_ai_turn(game, rng);
    }
}

fn handle_ai_turn_end(game: &mut Game, fx: &mut Effects, snd: &mut SoundQueue) {
    game.end_turn();
    if let Some(winner) = game.check_game_over() {
        game.winner = Some(winner);
        fx.spawn_victory();
        snd.play(sound::SND_VICTORY);
        game.phase = GamePhase::GameOver;
    } else {
        game.switch_player();
        game.phase = GamePhase::PlayerTurn(TurnPhase::ReadyToRoll);
    }
}

fn start_ai_turn(game: &mut Game, rng: &mut SimpleRng) {
    game.held_dice = [false; 6];
    game.turn_score = 0;
    game.roll_dice(&mut || rng.next_die());
    game.phase = GamePhase::AiRolling { frames: 0 };
}

/// Advance dice roll animation (new random values for non-held dice each frame).
fn roll_animate_dice(game: &mut Game, rng: &mut SimpleRng) {
    let mut new_dice = [0u8; 6];
    for (i, d) in new_dice.iter_mut().enumerate() {
        if !game.held_dice[i] { *d = rng.next_die(); }
        else { *d = game.dice[i]; }
    }
    game.dice = new_dice;
}

fn flash(game: &mut Game, msg: &'static str) {
    game.flash_msg = msg;
    game.flash_frames = 45;
}
