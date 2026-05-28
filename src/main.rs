#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

#[cfg(not(test))]
extern crate alloc;

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

mod framebuffer;
mod game;
mod input;
mod ui;
mod effects;
mod sound;

use uefi::prelude::*;
use uefi::proto::console::gop::GraphicsOutput;
use uefi::proto::console::text::Input;
use core::time::Duration;

use framebuffer::Framebuffer;
use game::{Game, GamePhase, TurnPhase, ai_decide, AiAction};
use input::{poll_input, GameInput};
use ui::render;
use effects::Effects;
use sound::SoundQueue;

const FRAME_DELAY_US: u64 = 16_000;

struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next(&mut self) -> u32 {
        self.state ^= self.state >> 12;
        self.state ^= self.state << 25;
        self.state ^= self.state >> 27;
        (self.state.wrapping_mul(0x2545F4914F6CDD1D) >> 32) as u32
    }

    fn next_die(&mut self) -> u8 {
        (self.next() % 6 + 1) as u8
    }
}

#[entry]
fn main() -> Status {
    uefi::helpers::init().unwrap();

    let gop_handle = match uefi::boot::get_handle_for_protocol::<GraphicsOutput>() {
        Ok(h) => h,
        Err(_) => return Status::ABORTED,
    };
    let mut gop = match uefi::boot::open_protocol_exclusive::<GraphicsOutput>(gop_handle) {
        Ok(g) => g,
        Err(_) => return Status::ABORTED,
    };
    let (width, height) = gop.current_mode_info().resolution();
    let mut fb = Framebuffer::new(width, height);

    let input_handle = match uefi::boot::get_handle_for_protocol::<Input>() {
        Ok(h) => h,
        Err(_) => return Status::ABORTED,
    };
    let mut input = match uefi::boot::open_protocol_exclusive::<Input>(input_handle) {
        Ok(inp) => inp,
        Err(_) => return Status::ABORTED,
    };

    let mut game = Game::new();
    let mut rng = SimpleRng::new(0xDEADBEEF_CAFEBABE);
    let mut ai_frame_counter: u32 = 0;
    let mut effects = Effects::new(width as i32, height as i32);
    let mut snd = SoundQueue::new();

    loop {
        let key = poll_input(&mut input);

        // Throttle: skip frames when no input and no animation
        let has_animation = match game.phase {
            GamePhase::PlayerTurn(TurnPhase::Rolling { .. }) => true,
            GamePhase::AiShowMeld { .. } => true,
            GamePhase::AiRolling { .. } => true,
            GamePhase::AiSelecting { .. } => true,
            GamePhase::GameOver => effects.particle_count > 0,
            _ => false,
        };
        if matches!(key, GameInput::None) && !has_animation && game.flash_frames == 0 {
            uefi::boot::stall(Duration::from_micros(FRAME_DELAY_US * 4));
        }

        process_game_phase(&mut game, key, &mut rng, &mut ai_frame_counter, &mut effects, &mut snd);

        if game.flash_frames > 0 {
            game.flash_frames -= 1;
            if game.flash_frames == 0 {
                game.flash_msg = "";
            }
        }

        effects.tick();

        // Non-blocking sound: plays one short segment per frame (before render)
        snd.tick();

        render(&mut fb, &game);
        effects.render(&mut fb);
        fb.present(&mut gop);
        uefi::boot::stall(Duration::from_micros(FRAME_DELAY_US));
    }
}

fn process_game_phase(
    game: &mut Game,
    key: GameInput,
    rng: &mut SimpleRng,
    ai_frame_counter: &mut u32,
    fx: &mut Effects,
    snd: &mut SoundQueue,
) {
    match game.phase {
        GamePhase::Title => {
            if matches!(key, GameInput::Confirm) {
                *game = Game::new();
                game.phase = GamePhase::PlayerTurn(TurnPhase::ReadyToRoll);
            }
            if matches!(key, GameInput::LangToggle) {
                game.lang_cn = !game.lang_cn;
            }
            if matches!(key, GameInput::Quit | GameInput::Escape) {
                game.phase = GamePhase::Quit;
            }
        }

        GamePhase::PlayerTurn(tp) => {
            // L key toggles language in any phase
            if matches!(key, GameInput::LangToggle) {
                game.lang_cn = !game.lang_cn;
            }
            match tp {
                TurnPhase::ReadyToRoll => {
                    match key {
                        GameInput::Bank => {
                            if game.turn_score > 0 {
                                game.bank_score();
                                game.phase = GamePhase::PlayerTurn(TurnPhase::Banking { frames: 0 });
                            } else {
                                flash(game, "No score to bank yet!");
                            }
                        }
                        GameInput::Roll => {
                            game.roll_dice(&mut || rng.next_die());
                            game.phase = GamePhase::PlayerTurn(TurnPhase::Rolling {
                                frames: 0,
                                frame_count: 15,
                            });
                        }
                        GameInput::Quit => {
                            game.phase = GamePhase::Title;
                        }
                        _ => {}
                    }
                }

                TurnPhase::Rolling { frames, frame_count } => {
                    if frames >= frame_count {
                        if game.is_farkle() {
                            game.turn_score = 0;
                            fx.spawn_farkle(320, 200);
                            snd.play(sound::SND_FARKLE);
                            game.phase = GamePhase::PlayerTurn(TurnPhase::Farkle { frames: 0 });
                        } else {
                            game.selected_dice = [false; 6];
                            game.cursor = 0;
                            game.phase = GamePhase::PlayerTurn(TurnPhase::Selecting);
                        }
                    } else {
                        let mut new_dice = [0u8; 6];
                        for (i, d) in new_dice.iter_mut().enumerate() {
                            if !game.held_dice[i] {
                                *d = rng.next_die();
                            } else {
                                *d = game.dice[i];
                            }
                        }
                        game.dice = new_dice;
                        game.phase = GamePhase::PlayerTurn(TurnPhase::Rolling {
                            frames: frames + 1,
                            frame_count,
                        });
                    }
                }

                TurnPhase::Selecting => {
                    match key {
                        GameInput::Left => {
                            game.cursor = if game.cursor == 0 { 5 } else { game.cursor - 1 };
                        }
                        GameInput::Right => {
                            game.cursor = if game.cursor == 5 { 0 } else { game.cursor + 1 };
                        }
                        GameInput::Select
                            if !game.held_dice[game.cursor] => {
                                game.selected_dice[game.cursor] =
                                    !game.selected_dice[game.cursor];
                            }
                        GameInput::Bank => {
                            let has_valid = game.check_selection_is_valid_meld().is_some();
                            if has_valid {
                                game.apply_selection();
                                let scored = game.turn_score;
                                game.bank_score();
                                fx.spawn_score_pop(320, 200, scored);
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
                                if game.held_dice.iter().all(|&h| h) {
                                    game.held_dice = [false; 6];
                                }
                                fx.spawn_score_pop(320, 200, game.turn_score);
                                game.roll_dice(&mut || rng.next_die());
                                snd.play(sound::SND_ROLL);
                                game.phase = GamePhase::PlayerTurn(TurnPhase::Rolling {
                                    frames: 0,
                                    frame_count: 30,
                                });
                            } else {
                                flash(game, "Select scoring dice (1s/5s), then R");
                            }
                        }
                        GameInput::Quit => {
                            game.phase = GamePhase::Title;
                        }
                        _ => {}
                    }
                }

                TurnPhase::Farkle { frames } => {
                    if frames >= 30 {
                        handle_player_turn_end(game, rng, ai_frame_counter, fx, snd);
                    } else {
                        game.phase = GamePhase::PlayerTurn(TurnPhase::Farkle { frames: frames + 1 });
                    }
                }

                TurnPhase::Banking { frames } => {
                    if frames >= 20 {
                        handle_player_turn_end(game, rng, ai_frame_counter, fx, snd);
                    } else {
                        game.phase = GamePhase::PlayerTurn(TurnPhase::Banking { frames: frames + 1 });
                    }
                }
            }
        }

        GamePhase::AiTurn => {
            let action = ai_decide(game);
            match &action {
                AiAction::Roll(meld) | AiAction::BankAfterMeld(meld) => {
                    // Store meld info for display
                    game.ai_meld_dice = [false; 6];
                    for &idx in &meld.indices[..meld.indices_len] {
                        game.ai_meld_dice[idx] = true;
                    }
                    game.ai_meld_name = meld.description;
                    game.ai_meld_points = meld.score;
                }
                AiAction::Farkle => {
                    game.ai_meld_dice = [false; 6];
                    game.ai_meld_name = "FARKLE!";
                    game.ai_meld_points = 0;
                }
            }
            // Store pending action for AiShowMeld
            game.ai_select_frame = match &action {
                AiAction::Roll(_) => 0,
                AiAction::BankAfterMeld(_) => 1,
                AiAction::Farkle => 2,
            };
            game.phase = GamePhase::AiShowMeld { frames: 0 };
        }

        GamePhase::AiShowMeld { frames } => {
            if frames >= 40 {
                // Apply the stored action after showing
                let action_type = game.ai_select_frame;
                game.ai_select_frame = 0;
                match action_type {
                    0 => {
                        // Roll: apply meld + roll remaining
                        game.held_dice = game.ai_meld_dice;
                        game.turn_score += game.ai_meld_points;
                        game.ai_meld_dice = [false; 6];
                        game.roll_dice(&mut || rng.next_die());
                        game.phase = GamePhase::AiRolling { frames: 0 };
                    }
                    1 => {
                        // Bank: apply meld + bank
                        game.held_dice = game.ai_meld_dice;
                        game.turn_score += game.ai_meld_points;
                        game.bank_score();
                        game.ai_meld_dice = [false; 6];
                        game.phase = GamePhase::AiSelecting { frames: 0 };
                    }
                    _ => {
                        // Farkle
                        game.turn_score = 0;
                        game.phase = GamePhase::AiSelecting { frames: 0 };
                    }
                }
            } else {
                game.phase = GamePhase::AiShowMeld { frames: frames + 1 };
            }
        }

        GamePhase::AiRolling { frames } => {
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
                let mut new_dice = [0u8; 6];
                for (i, d) in new_dice.iter_mut().enumerate() {
                    if !game.held_dice[i] {
                        *d = rng.next_die();
                    } else {
                        *d = game.dice[i];
                    }
                }
                game.dice = new_dice;
                game.phase = GamePhase::AiRolling { frames: frames + 1 };
            }
        }

        GamePhase::AiSelecting { frames } => {
            if frames >= 60 {
                handle_ai_turn_end(game, fx, snd);
            } else {
                game.phase = GamePhase::AiSelecting { frames: frames + 1 };
            }
        }

        GamePhase::GameOver => {
            if matches!(key, GameInput::LangToggle) {
                game.lang_cn = !game.lang_cn;
            }
            match key {
                GameInput::Confirm => {
                    *game = Game::new();
                    game.phase = GamePhase::Title;
                }
                GameInput::Quit => {
                    game.phase = GamePhase::Quit;
                }
                _ => {}
            }
        }

        GamePhase::Quit => {}
    }
}

fn handle_player_turn_end(game: &mut Game, rng: &mut SimpleRng, ai_frame_counter: &mut u32, fx: &mut Effects, snd: &mut SoundQueue) {
    game.end_turn();
    if let Some(winner) = game.check_game_over() {
        game.winner = Some(winner);
        fx.spawn_victory();
        snd.play(sound::SND_VICTORY);
        game.phase = GamePhase::GameOver;
    } else {
        game.switch_player();
        start_ai_turn(game, rng, ai_frame_counter);
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

fn start_ai_turn(game: &mut Game, rng: &mut SimpleRng, _ai_frame_counter: &mut u32) {
    game.held_dice = [false; 6];
    game.turn_score = 0;
    game.roll_dice(&mut || rng.next_die());
    game.phase = GamePhase::AiRolling { frames: 0 };
}

fn flash(game: &mut Game, msg: &'static str) {
    game.flash_msg = msg;
    game.flash_frames = 45; // ~0.75 seconds at 60fps
}
