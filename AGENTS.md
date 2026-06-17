# AGENTS.md — Farkle UEFI Dice Game

## Build

```bash
cargo build --release --target x86_64-unknown-uefi
# Or use the script (handles staging to esp/):
bash scripts/build.sh
```

Output: `esp/EFI/BOOT/BOOTX64.EFI`

## Test

```bash
# Full verification pipeline (build + clippy + game logic tests):
bash scripts/verify.sh

# Game logic tests only (standalone harness, not cargo test):
cd game_test && cargo run

# QEMU (no sudo required, uses mtools FAT16 image):
bash scripts/run-qemu.sh

# QEMU with GDB stub (port 1234):
bash scripts/run-qemu-gdb.sh

# Read QEMU debug log (after running in QEMU):
mcopy -i esp.img ::frakle_debug.log /dev/stdout
```

## Deploy to real hardware

```bash
# Interactive device selection:
bash scripts/deploy-usb.sh

# Quick deploy to /dev/sda:
sudo scripts/quick-deploy.sh
```

## Key constraints

- **uefi crate must be 0.28** (not 0.37). Matches UHIA project at `../UHIA`. API differences are breaking (SystemTable parameter style, `bs.stall()` vs `uefi::boot::stall()`).
- **Rust nightly required**. `rust-toolchain.toml` pins nightly + `x86_64-unknown-uefi` target.
- **sudo + rustup**: build scripts must set `RUSTUP_HOME` and `CARGO_HOME` to original user's dirs, otherwise root's toolchain config is used and nightly target is missing.
- **QEMU vvfat bug**: do NOT use `fat:rw:` drive format — QEMU crashes when guest writes files. Use mtools-created FAT16 image instead (handled by `run-qemu.sh`).
- **QEMU disk cache**: guest writes flush periodically to host file. Use `cache=unsafe` for faster log visibility. Long-running sessions accumulate more log entries.
- **OVMF FAT32 limitation**: OVMF doesn't boot from FAT32 raw images. Use FAT16 (`mkfs.fat -F 16`).
- **pflash boot**: use `-drive if=pflash` for OVMF_CODE/VARS. The `-bios` flag with raw images doesn't auto-scan for EFI boot files.

## Known issues

- **Hardware crash**: game runs in QEMU but may crash to black screen on real hardware. No CPU exception — likely firmware GOP driver issue. Debug overlay (green frame counter `F:XXXX` top-left) helps pinpoint crash moment.

## Debug logging

Game writes `\frakle_debug.log` to ESP partition. Logs:
- Startup info (Game struct size)
- Phase transitions (Title → P:Roll? → P:Select → AI:Think → …)
- Key events
- Game over (winner, final scores)
- Heartbeat every 600 frames (~10s)

Works on real hardware and QEMU. QEMU requires `cache=unsafe` on drive for timely flush.

## Cross-project reference

`../UHIA` is the reference UEFI project. Useful for:
- Deployment script patterns
- File logging implementation (uefi 0.28 API)
- uefi crate version and feature flags

## Project structure

```
src/
├── main.rs          # UEFI entry, game loop, debug overlay, watchdog
├── lib.rs           # FmtBuf stack formatter
├── game.rs          # Farkle rules, AI opponent, meld detection
├── framebuffer.rs   # GOP framebuffer, embedded-graphics DrawTarget
├── input.rs         # Keyboard polling (SimpleTextIn)
├── effects.rs       # Particles, screen shake, victory effects
├── sound.rs         # Non-blocking PC speaker driver (PIT ch2 + 0x61 gate)
├── logger.rs        # File-based debug logging (phase changes, keys, heartbeat)
└── ui/
    ├── mod.rs       # UI state machine, layout rendering
    ├── dice.rs      # Dice face drawing
    ├── layout.rs    # Responsive layout (scales with screen height)
    ├── lang.rs      # Bilingual EN/CN string tables
    └── cn_font.rs   # 12×12 Chinese pixel font
```
