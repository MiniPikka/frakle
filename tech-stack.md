# Farkle UEFI Game - Technology Stack

## Overview

A Farkle dice game running natively as a UEFI application, written in Rust, with a pixel-based graphical interface rendered via the UEFI Graphics Output Protocol (GOP).

## Build Toolchain

| Component | Choice | Version |
|-----------|--------|---------|
| Language | Rust (stable) | 1.95.0 |
| Build System | Cargo + [cargo-make](https://github.com/sagiegurari/cargo-make) | latest |
| Target Triple | `x86_64-unknown-uefi` | - |
| Linker | `lld-link` (LLVM) | bundled with Rust |

## Core Crates

| Crate | Purpose | Justification |
|-------|---------|---------------|
| [`uefi`](https://crates.io/crates/uefi) | v0.37.0 | UEFI API bindings (GOP, Input, Boot Services, allocator, logger, entry macro) |
| [`embedded-graphics`](https://crates.io/crates/embedded-graphics) | v0.8.2 | 2D drawing primitives (text, shapes, framebuffer); `no_std` compatible |
| SimpleRng (xorshift64*) | built-in | PRNG for dice rolling; no external dependency needed |

## Graphics Stack

```
┌──────────────────────────────────────┐
│           Game UI Layer              │  ← Dice faces, scoreboard, buttons, animations
├──────────────────────────────────────┤
│        embedded-graphics             │  ← Primitives (Rectangle, Text, Circle, Line)
├──────────────────────────────────────┤
│     Custom Framebuffer Adapter       │  ← Adapts UEFI GOP to embedded-graphics DrawTarget
├──────────────────────────────────────┤
│   uefi::proto::console::gop          │  ← Blt() to screen via UEFI GOP protocol
├──────────────────────────────────────┤
│         UEFI Firmware GOP            │  ← Hardware framebuffer (EmulatorPkg / real UEFI)
└──────────────────────────────────────┘
```

### Rendering Pipeline

1. **Draw to back buffer** — All rendering goes into a `Vec<u32>` in BGRx format (matches GOP `Blt` pixel layout)
2. **Flush via `Blt`** — Call `GOP::blt()` with `EfiBltBufferToVideo` to copy back buffer to screen
3. **Double buffering** — Eliminates tearing; essential since GOP framebuffer may be uncached

### Key Rendering Details

- **Color format**: BGR (Blue-Green-Red-Alpha-Reserved), as used by `EFI_GRAPHICS_OUTPUT_BLT_PIXEL`
- **Resolution**: Auto-detect from `GOP::current_mode_info()` → `HorizontalResolution` / `VerticalResolution`
- **Fallback**: 640×480 minimum expected; layout must be responsive or fixed at minimum resolution
- **Font**: `embedded-graphics` monospace fonts (`MonoFont`) for score text; pixel-art for decorative elements

## Input

| Protocol | Crate Path | Usage |
|----------|-----------|-------|
| Simple Text Input | `uefi::proto::console::text::input` | Poll keyboard for arrow keys, Enter, Space |
| WaitForEvent | `uefi::table::boot::BootServices::wait_for_event` | Non-blocking input loop with timed wait |

### Key Mapping

| Key | Action |
|-----|--------|
| Arrow Left/Right | Navigate between dice / menu items |
| Space | Select / deselect a die (hold for scoring) |
| Enter | Confirm roll / end turn |
| Esc | Bank points and end turn |

## Memory & Allocator

- **Global allocator**: `uefi-services` provides a UEFI-backed global allocator (`AllocatePool` / `FreePool`)
- **Heap**: Standard Rust `Vec`, `String`, `Box` available
- **Stack**: Minimal; game state fits in a few KB
- **No external dependencies**: No filesystem access needed; all assets compiled into binary via `include_bytes!`

## Project Structure

```
frakle/
├── Cargo.toml                # Package manifest
├── .cargo/
│   └── config.toml            # Target and linker configuration
├── src/
│   ├── main.rs                # UEFI entry point, init GOP & input
│   ├── framebuffer.rs         # Framebuffer adapter (impl DrawTarget for GOP buffer)
│   ├── game.rs                # Game state machine (Farkle rules engine)
│   ├── ui/
│   │   ├── mod.rs             # UI module
│   │   ├── dice.rs            # Dice face rendering (1-6 pips)
│   │   ├── scoreboard.rs      # Score display, turn indicator
│   │   ├── menu.rs            # Main menu / game over screen
│   │   └── layout.rs          # Responsive layout calculations
│   └── input.rs               # Keyboard input handler
├── assets/
│   └── (optional .bmp files embedded at compile time)
├── build.rs                   # Build script (if needed for env vars)
├── Makefile.toml              # cargo-make tasks (build, run-emulator)
├── tech-stack.md
├── game-design-doc.md
└── rules.md
```

## Build & Run

### Prerequisites

```bash
rustup target add x86_64-unknown-uefi
rustup component add llvm-tools-preview
cargo install cargo-make
```

### Cargo Config (`.cargo/config.toml`)

```toml
[build]
target = "x86_64-unknown-uefi"

[target.x86_64-unknown-uefi]
linker = "rust-lld"
```

### Build Commands

```bash
# Build UEFI binary
cargo build --release

# Output: target/x86_64-unknown-uefi/release/frakle.efi

# Verify binary type
file target/x86_64-unknown-uefi/release/frakle.efi
# PE32+ executable for EFI (application), x86-64
```

### Running in EmulatorPkg

Copy `frakle.efi` to the EmulatorPkg build output directory and either:
- Run from UEFI Shell: `Shell> frakle.efi`
- Create `startup.nsh` for auto-launch

## Why This Stack?

| Design Choice | Rationale |
|---------------|-----------|
| **Rust + UEFI** | Memory safety in a bare-metal context; no OS dependency; the `uefi` crate is mature and well-maintained |
| **embedded-graphics** | Battle-tested `no_std` 2D library; avoids writing a full software renderer from scratch; supports text rendering |
| **No dynamic allocation for rendering** | GOP back buffer is a single pre-allocated `Vec<u32>`; no per-frame allocations |
| **No asset files** | `include_bytes!` embeds everything; single `.efi` file deployment |
| **cargo-make** | Simplifies complex build steps (compile + copy to emulator + launch emulator) into one command |
