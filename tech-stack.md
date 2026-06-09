# Farkle - Technology Stack

## Overview

A Farkle dice game running natively as a UEFI application, written in Rust, with a pixel-based graphical interface rendered directly to the GOP framebuffer.

## Build Toolchain

| Component | Choice | Version |
|-----------|--------|---------|
| Language | Rust (nightly) | latest |
| Target Triple | `x86_64-unknown-uefi` | - |
| Linker | `rust-lld` (LLVM) | bundled |

## Core Crates

| Crate | Version | Purpose |
|-------|---------|---------|
| [`uefi`](https://crates.io/crates/uefi) | 0.28 | UEFI API bindings (GOP, Input, Boot Services, allocator) |
| `uefi-services` | 0.25 | UEFI global allocator, panic handler, entry macro |
| [`embedded-graphics`](https://crates.io/crates/embedded-graphics) | 0.8 | 2D drawing primitives (text, shapes, framebuffer); `no_std` compatible |
| `spin` | 0.9 | `Mutex` for logger state |

## Graphics Stack

```
┌──────────────────────────────────────┐
│           Game UI Layer              │  ← Dice faces, scoreboard, buttons, animations
├──────────────────────────────────────┤
│        embedded-graphics             │  ← Primitives (Rectangle, Text, Circle)
├──────────────────────────────────────┤
│     Framebuffer (double buffer)      │  ← Vec<u32> BGRx buffer, impl DrawTarget
├──────────────────────────────────────┤
│   Direct GOP framebuffer write       │  ← copy_nonoverlapping per row (no blt())
├──────────────────────────────────────┤
│         UEFI Firmware GOP            │  ← Hardware framebuffer
└──────────────────────────────────────┘
```

### Rendering Pipeline

1. **Draw to back buffer** — All rendering goes into `Vec<u32>` in BGRx format (matches GOP layout)
2. **Flush directly to GOP** — `copy_nonoverlapping` each row from back buffer to GOP framebuffer. No `gop.blt()` call — avoids potential firmware bugs
3. **Double buffering** — Eliminates tearing

### Key Details

- **Color format**: BGRx (Blue in lowest byte), matching standard UEFI GOP format
- **Resolution**: Auto-detect from `GOP::current_mode_info()`
- **Font**: embedded-graphics `MonoFont` for score text; custom 12×12 Chinese pixel font; built-in 5×7 ASCII font for debug overlay

## Input

| Protocol | Usage |
|----------|-------|
| Simple Text Input | Poll keyboard (arrow keys, Space, R, B, L, Q) |

## Memory & Allocator

- **Global allocator**: `uefi-services` provides UEFI-backed allocator (`AllocatePool`/`FreePool`)
- **Hot path**: Zero heap allocations — all formatting uses `FmtBuf<N>` stack buffers
- **Framebuffer**: Single `Vec<u32>` double buffer, BGRx format

## Project Structure

```
frakle/
├── src/
│   ├── main.rs            # UEFI entry, game loop, debug overlay, phase watchdog
│   ├── lib.rs             # FmtBuf stack formatter, fmt_replace template helper
│   ├── game.rs            # Farkle rules engine, AI (optimal non-overlapping melds)
│   ├── framebuffer.rs     # Double buffer, DrawTarget impl, direct GOP write
│   ├── input.rs           # Keyboard polling
│   ├── effects.rs         # Particle system, screen shake, victory effects
│   ├── sound.rs           # Sound stubs (no audio hardware)
│   ├── logger.rs          # File-based debug log (ESP:\frakle_debug.log)
│   └── ui/
│       ├── mod.rs         # UI state machine (zero-alloc rendering)
│       ├── dice.rs        # Dice face drawing
│       ├── layout.rs      # Responsive layout
│       ├── lang.rs        # Bilingual EN/CN strings
│       └── cn_font.rs     # 12×12 Chinese pixel font
├── scripts/
│   ├── build.sh           # Build release + stage to esp/
│   ├── run-qemu.sh        # QEMU launcher
│   ├── run-qemu-gdb.sh    # QEMU with GDB stub
│   ├── deploy-usb.sh      # USB deployment
│   └── quick-deploy.sh    # Quick deploy to /dev/sda
└── game_test/             # Standalone 15-test harness (host target)
```

## Build & Run

```bash
# Build
cargo build --release --target x86_64-unknown-uefi

# QEMU
bash scripts/run-qemu.sh

# USB deploy
sudo scripts/quick-deploy.sh
```

## Why This Stack?

| Design Choice | Rationale |
|---------------|-----------|
| **Direct FB write vs blt()** | Avoids firmware GOP blt() bugs (suspected crash source on some hardware) |
| **BGRx internal format** | Matches GOP layout — memcpy per row, zero conversion |
| **FmtBuf stack formatting** | Zero heap allocs in rendering — prevents UEFI allocator fragmentation |
| **RDTSC seed** | Per-boot random dice rolls, no UEFI protocol dependency |
| **No asset files** | Everything compiled into `.efi`; single file deployment |
