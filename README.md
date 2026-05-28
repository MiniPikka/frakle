# Farkle — A UEFI Native Dice Game in Rust

<p align="center">
  <img src="gameplay.mp4" width="640" alt="Farkle gameplay demo" />
</p>

**Farkle** is a press-your-luck dice game (also known as *Zonk*, *Hot Dice*, *Ten Thousand*) running **natively on UEFI firmware** — no operating system required. Built entirely in Rust with a custom pixel-based graphical UI rendered through the UEFI Graphics Output Protocol (GOP).

---

## Features

- **Bare-metal UEFI application** — boots directly from the UEFI Shell, no OS needed
- **Pixel-perfect graphical UI** — dice, scoreboard, buttons, and particle effects
- **Single-player vs computer** — smart rule-based opponent ("Lucky")
- **Bilingual UI** — English / Chinese (pinyin), toggle with `L` key
- **Particle effects** — score explosions, farkle screen shake, victory confetti
- **PC Speaker sound** — non-blocking cooperative beeps (requires QEMU with `-machine pcspk-audiodev`)
- **Clean code** — zero Clippy warnings, 15 game-logic tests passing

---

## Gameplay

| Action | Key |
|--------|-----|
| Move cursor | `←` `→` |
| Select / deselect die | `Space` |
| Score meld + roll again | `R` |
| Score meld + bank | `B` |
| Toggle language | `L` |
| Quit | `Q` |

### Rules (Farkle)

Roll six dice and set aside **at least one scoring combination** each roll. Re-roll remaining dice. Bank your score at any time. If a roll produces no scoring dice, you **Farkle** and lose all unbanked points for the turn. First to **5,000** wins.

| Meld | Points |
|------|--------|
| Each 1 | 100 |
| Each 5 | 50 |
| Three 1s | 1,000 |
| Three 2s–6s | 200–600 |
| Four of a kind | 1,000 |
| Five of a kind | 2,000 |
| Six of a kind | 3,000 |
| 1–6 Straight | 1,500 |
| Three pairs | 1,500 |
| Two triplets | 2,500 |

---

## Technology Stack

| Layer | Technology |
|-------|-----------|
| Language | Rust 1.95 (stable) |
| Target | `x86_64-unknown-uefi` |
| UEFI bindings | [`uefi`](https://crates.io/crates/uefi) 0.37 |
| 2D Graphics | [`embedded-graphics`](https://crates.io/crates/embedded-graphics) 0.8 |
| Chinese Font | Generated from Sarasa Gothic SC via ImageMagick (`gen_cn_font.sh`) |
| Sound | x86 `in`/`out` port I/O (PC Speaker, PIT) |
| RNG | Xorshift64* (no external dependency) |
| Emulator | QEMU + OVMF |

Full details: [`tech-stack.md`](tech-stack.md)

---

## Project Structure

```
frakle/
├── src/
│   ├── main.rs            # UEFI entry point, game loop
│   ├── game.rs            # Farkle rules engine, computer opponent, meld detection
│   ├── framebuffer.rs     # GOP double-buffer, embedded-graphics DrawTarget
│   ├── input.rs           # Keyboard polling (UEFI SimpleTextIn)
│   ├── effects.rs         # Particle system, screen shake, victory effects
│   ├── sound.rs           # Non-blocking PC speaker sound queue
│   └── ui/
│       ├── mod.rs         # UI state machine, layout rendering
│       ├── dice.rs        # Dice face drawing (pips, highlights)
│       ├── layout.rs      # Responsive layout (scales with screen height)
│       ├── lang.rs        # Bilingual EN/CN string tables
│       └── cn_font.rs     # Auto-generated 12×12 Chinese pixel font
├── game_test/             # Standalone game logic test harness (15 tests)
├── gen_cn_font.sh         # Chinese font bitmap generator
├── run_qemu.sh            # One-click QEMU launcher
├── game-design-doc.md     # Full game design document
├── tech-stack.md          # Technology choices and architecture
└── rules.md               # Original Farkle rule reference
```

---

## Build & Run

### Prerequisites

```bash
rustup target add x86_64-unknown-uefi
# For Chinese font generation (one-time):
# Requires ImageMagick + Sarasa Gothic SC font
```

### Build

```bash
cargo build --release
# Output: target/x86_64-unknown-uefi/release/frakle.efi
```

### Run in QEMU

```bash
./run_qemu.sh
```

This script automatically creates a FAT disk image with the game binary and boots QEMU with OVMF firmware, PC speaker audio support, and GTK display.

### Run on Real Hardware

Copy `frakle.efi` to an EFI System Partition and launch from the UEFI Shell:

```
Shell> fs0:
FS0:\> frakle.efi
```

---

## Design Documents

- [`game-design-doc.md`](game-design-doc.md) — Complete game design with state machine, UI layout, scoring algorithm
- [`tech-stack.md`](tech-stack.md) — Technology choices, architecture decisions, build pipeline
- [`rules.md`](rules.md) — Original Farkle rule reference

---

## Acknowledgements

- Chinese font glyphs generated from [Sarasa Gothic](https://github.com/be5invis/Sarasa-Gothic) (SIL Open Font License)
- UEFI development powered by the [uefi-rs](https://github.com/rust-osdev/uefi-rs) project
- Graphics rendering via [embedded-graphics](https://github.com/embedded-graphics/embedded-graphics)

---

## License

MIT
