# AGENTS.md — Farkle UEFI Dice Game

## Build & Test

```bash
cargo build --release --target x86_64-unknown-uefi   # or: bash scripts/build.sh
bash scripts/verify.sh                                 # build + clippy + game logic tests
cd game_test && cargo run                              # game logic tests only
bash scripts/run-qemu.sh                               # QEMU (auto-detect display, with audio)
bash scripts/run-qemu-gdb.sh                           # QEMU + GDB stub (port 1234)
mcopy -i esp.img ::frakle_debug.log /dev/stdout        # read QEMU debug log
```

## Deploy

```bash
bash scripts/deploy-usb.sh           # interactive device selection
sudo scripts/quick-deploy.sh         # quick deploy to /dev/sda
```

## Architecture

```
src/
├── main.rs          # UEFI entry, game loop, debug overlay, serial panic handler
├── lib.rs           # FmtBuf stack formatter, fmt_replace helper, clamp_utf8
├── game.rs          # Farkle rules engine, AI opponent, meld detection/scoring
├── framebuffer.rs   # GOP double-buffer + embedded-graphics DrawTarget + scanlines + big title
├── background.rs    # Balatro-style gradient background (dark purple→blue vertical)
├── input.rs         # Keyboard polling (UEFI SimpleTextIn)
├── effects.rs       # Particle system, screen shake, flash overlay, animated scores, combo counter
├── sound.rs         # Non-blocking PC speaker driver (PIT ch2 + 0x61 gate)
├── logger.rs        # File-based debug logging to ESP:\frakle_debug.log
└── ui/
    ├── mod.rs       # UI state machine, floating meld text, combo display, title breathing
    ├── dice.rs      # Dice face drawing with 3-layer golden glow halos
    ├── layout.rs    # Responsive layout (scales with screen height)
    ├── lang.rs      # Bilingual EN/CN string tables
    └── cn_font.rs   # Auto-generated 12×12 Chinese pixel font (sorted, binary search)
```

## Visual system

- **Background**: vertical gradient `#0A0618` → `#060A18` (dark purple → dark blue)
- **Scanlines**: odd rows darkened ~5% (CRT texture)
- **Particles**: direct `set_pixel`, bypasses DrawTarget for performance
- **Flash overlay**: additive color blend, fades over N frames (red=Farkle, green=Bank, gold=Victory)
- **Title**: 4× pixel-art font with 3-layer golden glow + breathing animation (2s period)
- **Dice**: selected dice get 3-layer gold glow halos (Balatro bloom effect)
- **Score counter**: animated — counts up from old→new value at 30pts/frame
- **Combo system**: consecutive banks without farkle → escalating particles/flash + "×N COMBO!" text
- **Milestones**: crossing 1000/2000/3000/4000 → gold burst + floating "N000!" text
- **Hot Dice**: all 6 dice score → triple particle burst + gold flash + "HOT DICE!" floating text
- **Meld floating text**: "+350" drifts up and fades after banking

## Key constraints

- **uefi crate must be 0.28** (not 0.37). API differences are breaking.
- **Rust nightly required**. `rust-toolchain.toml` pins nightly + `x86_64-unknown-uefi` target.
- **GOP resolution capped at 1024×768** — fonts are fixed-pixel-size, higher res makes text unreadable.
- **sudo + rustup**: build scripts must set `RUSTUP_HOME` and `CARGO_HOME` to original user's dirs.
- **QEMU vvfat bug**: do NOT use `fat:rw:` — QEMU crashes when guest writes files. Use mtools FAT16 image.
- **OVMF FAT32 limitation**: OVMF doesn't boot from FAT32 raw images. Use FAT16 (`mkfs.fat -F 16`).
- **pflash boot**: use `-drive if=pflash` for OVMF_CODE/VARS. `-bios` flag doesn't auto-scan for EFI boot files.
- **run-qemu.sh always rebuilds** — EFI binary is deleted before copy to force rebuild.

## Key constraints

- **uefi crate must be 0.28** (not 0.37). API differences are breaking.
- **Rust nightly required**. `rust-toolchain.toml` pins nightly + `x86_64-unknown-uefi` target.
- **sudo + rustup**: build scripts must set `RUSTUP_HOME` and `CARGO_HOME` to original user's dirs.
- **QEMU vvfat bug**: do NOT use `fat:rw:` — QEMU crashes when guest writes files. Use mtools FAT16 image.
- **OVMF FAT32 limitation**: OVMF doesn't boot from FAT32 raw images. Use FAT16 (`mkfs.fat -F 16`).
- **pflash boot**: use `-drive if=pflash` for OVMF_CODE/VARS. `-bios` flag doesn't auto-scan for EFI boot files.

## Debug logging

Game writes `\frakle_debug.log` to ESP partition. Logs startup, phase transitions, game over, heartbeat (every 600 frames). QEMU requires `cache=unsafe` on drive for timely flush.

## Known issues

- **Hardware crash**: game may crash to black screen on real hardware. Likely firmware GOP driver issue. `clflush` + `mfence` in `present()` mitigates this in QEMU.
