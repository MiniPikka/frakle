# CLAUDE.md — Farkle Project Context

## Project Overview

Farkle is a bare-metal UEFI dice game written in Rust. No OS — boots directly from firmware via GOP framebuffer rendering. Single-player vs AI opponent ("Lucky"). Bilingual EN/CN UI.

## Build & Test Commands

```bash
# Build release EFI binary
cargo build --release --target x86_64-unknown-uefi
# Or: bash scripts/build.sh

# Lint (zero warnings required)
cargo clippy --target x86_64-unknown-uefi

# Type check
cargo check --target x86_64-unknown-uefi

# Game logic tests (standalone harness, not cargo test)
# Has own .cargo/config.toml that overrides parent's UEFI target
cd game_test && cargo run

# Full verification pipeline
bash scripts/verify.sh

# QEMU testing
bash scripts/run-qemu.sh           # Auto-detect display
GDB=1 bash scripts/run-qemu.sh     # With GDB stub on :1234
env QEMU_DISPLAY=none bash scripts/run-qemu.sh  # Headless

# Read QEMU debug log (after running in QEMU)
mcopy -i esp.img ::frakle_debug.log /dev/stdout

# Deploy to USB (destructive!)
sudo bash scripts/quick-deploy.sh   # /dev/sda
sudo USB_DEV=/dev/sdb bash scripts/deploy-usb.sh  # specific device
```

## Architecture

```
src/
├── main.rs          # UEFI entry, game loop, debug overlay, watchdog timer
├── lib.rs           # FmtBuf stack formatter, fmt_replace helper
├── game.rs          # Farkle rules engine, AI opponent, meld detection/scoring
├── framebuffer.rs   # GOP direct framebuffer + embedded-graphics DrawTarget
├── input.rs         # Keyboard polling (UEFI SimpleTextIn)
├── effects.rs       # Particle system, screen shake, victory confetti
├── sound.rs         # Sound stubs (no hardware access, API preserved)
├── logger.rs        # File-based debug logging to ESP:\frakle_debug.log
└── ui/
    ├── mod.rs       # UI state machine, layout rendering
    ├── dice.rs      # Dice face drawing (pips, highlights)
    ├── layout.rs    # Responsive layout (scales with screen height)
    ├── lang.rs      # Bilingual EN/CN string tables
    └── cn_font.rs   # Auto-generated 12×12 Chinese pixel font
```

## Key Technical Decisions

- **uefi crate v0.28** (not latest) — matches UHIA project, stable API
- **Zero heap allocation in hot path** — stack-based FmtBuf, pre-allocated BltPixel buffer
- **Direct framebuffer writes** — `copy_nonoverlapping` per scanline, not `gop.blt()` (firmware compat)
- **RDTSC-based RNG seed** — per-boot randomness without OS entropy
- **Rust nightly required** — UEFI target needs `#![no_std]`, `#![no_main]`

## Quality Gates

| Check | Command | Required |
|-------|---------|----------|
| Clippy zero warnings | `cargo clippy --target x86_64-unknown-uefi` | Yes |
| Game logic tests | `cd game_test && cargo run` (15 tests) | Yes |
| Release build | `cargo build --release --target x86_64-unknown-uefi` | Yes |
| QEMU smoke test | `bash scripts/run-qemu.sh` (manual visual check) | Before deploy |
| Binary size | Should be ~80KB | Monitor |

## Debugging

See `DEBUG.md` for full guide. Key points:
- **Screen overlay**: green frame counter + phase name (top-left), yellow scores
- **Red "STUCK!"** appears if phase hangs >10 seconds
- **File log**: `\frakle_debug.log` — phase changes, key events, heartbeat (every 600 frames)
- **QEMU log read**: `mcopy -i esp.img ::frakle_debug.log /dev/stdout` (needs `cache=unsafe`)
- **Real hardware log**: USB root `\frakle_debug.log`
- **GDB**: `bash scripts/run-qemu-gdb.sh` then `gdb -ex "target remote :1234"`

## QEMU Gotchas (learned the hard way)

- **FAT16 required for raw images**: OVMF doesn't boot from FAT32 raw disk images. Use FAT16 (`mkfs.fat -F 16`). FAT16 supports guest file writes (debug log) without QEMU crashes.
- **pflash boot method**: Use `-drive if=pflash` for OVMF_CODE and OVMF_VARS. The `-bios` flag with raw images doesn't auto-scan for EFI boot files.
- **OVMF_VARS caching**: Fresh copy of OVMF_VARS each run (`cp` to `/tmp/`) — stale boot entries cause "Not Found" errors.
- **vvfat driver bug**: QEMU's `fat:rw:` vvfat driver crashes when guest writes files. Scripts now use mtools-created FAT16 images instead.
- **Disk cache**: QEMU's IDE driver caches guest writes. Use `cache=unsafe` on the drive to get faster log visibility. Guest writes flush periodically — not immediately. Long-running sessions accumulate more log entries.
- **Display backend**: Wayland → use `sdl`; X11 → use `gtk`; headless → `none`. Auto-detected in `run-qemu.sh`.
- **sudo + rustup**: Running build scripts as root loses the nightly toolchain. `build.sh` sets `RUSTUP_HOME` and `CARGO_HOME` to the original user's dirs.
- **GDB + QEMU**: `run-qemu-gdb.sh` starts QEMU with `-s -S` (GDB stub, paused). Connect with `gdb -ex "target remote :1234"`. The `-d int,cpu_reset -D crash.log` flags capture CPU exceptions.

## Known Issue: Hardware Crash

Game runs in QEMU but crashes to black screen on some real hardware. Not a CPU exception — likely firmware GOP driver incompatibility with direct framebuffer writes. See `DEBUG.md` for investigation history.

## Cross-Project References

- **UHIA** (`../UHIA/`) — sibling UEFI project, same tech stack. `deploy-usb.sh` pattern was borrowed from there. Uses uefi 0.28, same build scripts structure.
- **UEFI-frakle** (`../UEFI-frakle/`) — earlier version of this project (CLAUDE.md created there first)
