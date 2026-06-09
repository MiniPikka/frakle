# Farkle — A UEFI Native Dice Game in Rust

**Farkle** is a press-your-luck dice game (also known as *Zonk*, *Hot Dice*, *Ten Thousand*) running **natively on UEFI firmware** — no operating system required. Built entirely in Rust with a custom pixel-based graphical UI rendered through the UEFI Graphics Output Protocol (GOP).

---

## Features

- **Bare-metal UEFI application** — boots directly from firmware, no OS needed
- **Pixel-perfect graphical UI** — dice, scoreboard, buttons, and particle effects
- **Single-player vs computer** — smart rule-based opponent ("Lucky")
- **Bilingual UI** — English / Chinese, toggle with `L` key
- **Particle effects** — score explosions, farkle screen shake, victory confetti
- **Direct framebuffer rendering** — writes BGRx pixels directly to GOP framebuffer
- **Zero heap allocation in hot path** — stack-buffer formatting, no per-frame allocs
- **Zero Clippy warnings** (main crate), 15 game-logic tests passing

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
| Language | Rust (nightly) |
| Target | `x86_64-unknown-uefi` |
| UEFI bindings | [`uefi`](https://crates.io/crates/uefi) 0.28 |
| 2D Graphics | [`embedded-graphics`](https://crates.io/crates/embedded-graphics) 0.8 |
| Chinese Font | Generated from Sarasa Gothic SC (12×12 pixel glyphs) |
| RNG | Xorshift64* + RDTSC seed (per-boot random) |
| Emulator | QEMU + OVMF |

Full details: [`tech-stack.md`](tech-stack.md)

---

## Project Structure

```
frakle/
├── src/
│   ├── main.rs            # UEFI entry point, game loop, debug overlay, watchdog
│   ├── lib.rs             # Shared FmtBuf (stack formatter), fmt_replace helper
│   ├── game.rs            # Farkle rules engine, AI opponent, meld detection
│   ├── framebuffer.rs     # GOP direct framebuffer, embedded-graphics DrawTarget
│   ├── input.rs           # Keyboard polling (UEFI SimpleTextIn)
│   ├── effects.rs         # Particle system, screen shake, victory effects
│   ├── sound.rs           # Sound effect stubs (no audio hardware access)
│   ├── logger.rs          # File-based debug logging (ESP:\frakle_debug.log)
│   └── ui/
│       ├── mod.rs         # UI state machine, layout rendering (zero-alloc)
│       ├── dice.rs        # Dice face drawing (pips, highlights)
│       ├── layout.rs      # Responsive layout (scales with screen height)
│       ├── lang.rs        # Bilingual EN/CN string tables
│       └── cn_font.rs     # Auto-generated 12×12 Chinese pixel font
├── scripts/
│   ├── build.sh           # Build release + stage to esp/
│   ├── run-qemu.sh        # One-click QEMU launcher
│   ├── run-qemu-gdb.sh    # QEMU with GDB stub (:1234) for remote debugging
│   ├── deploy-usb.sh      # USB deployment (interactive device selection)
│   └── quick-deploy.sh    # Deploy to /dev/sda with confirmation
├── esp/                   # Staged EFI boot files
│   └── EFI/BOOT/BOOTX64.EFI
├── game_test/             # Standalone game logic test harness (15 tests)
├── game-design-doc.md     # Full game design document
├── tech-stack.md          # Technology choices and architecture
├── rules.md               # Original Farkle rule reference
└── DEBUG.md               # Debugging guide
```

---

## Build & Run

### Prerequisites

```bash
rustup toolchain install nightly
rustup target add x86_64-unknown-uefi
# Arch: sudo pacman -S edk2-ovmf qemu-desktop
```

### Build

```bash
cargo build --release --target x86_64-unknown-uefi
# Or: bash scripts/build.sh
# Output: esp/EFI/BOOT/BOOTX64.EFI
```

### Run in QEMU

```bash
bash scripts/run-qemu.sh
```

### GDB Debugging

```bash
bash scripts/run-qemu-gdb.sh     # Starts QEMU with GDB stub on :1234
# In another terminal:
gdb -ex "target remote :1234" target/x86_64-unknown-uefi/debug/frakle.efi
```

### Run on Real Hardware

```bash
sudo scripts/quick-deploy.sh     # Deploy to /dev/sda (USB drive)
```

---

## Known Issue: Hardware-Specific Crash

The game runs fine in QEMU (OVMF) but may crash to black screen on some real hardware. Investigation (GDB + QEMU exception logging) shows:

- **No CPU exception occurs** — the crash is not a page fault, GP fault, or undefined opcode
- QEMU timer interrupts continue normally — game logic keeps running
- Likely a firmware GOP driver issue with direct framebuffer writes on specific GPUs

Debug overlay (green text top-left) shows frame counter + current game phase — useful for pinpointing the crash moment.

---

## Design Documents

- [`game-design-doc.md`](game-design-doc.md) — Complete game design with state machine, UI layout, scoring algorithm
- [`tech-stack.md`](tech-stack.md) — Technology choices, architecture decisions, build pipeline
- [`rules.md`](rules.md) — Original Farkle rule reference
- [`DEBUG.md`](DEBUG.md) — Debugging guide and QEMU setup

---

## Acknowledgements

- Chinese font glyphs generated from [Sarasa Gothic](https://github.com/be5invis/Sarasa-Gothic) (SIL Open Font License)
- UEFI development powered by the [uefi-rs](https://github.com/rust-osdev/uefi-rs) project
- Graphics rendering via [embedded-graphics](https://github.com/embedded-graphics/embedded-graphics)

---

## License

MIT
