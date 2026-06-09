#!/bin/bash
# run-qemu.sh — Run Farkle in QEMU for testing
#
# Usage:
#   ./scripts/run-qemu.sh              # Auto-detect display
#   QEMU_DISPLAY=gtk ./scripts/run-qemu.sh
#   QEMU_DISPLAY=sdl ./scripts/run-qemu.sh
#   QEMU_DISPLAY=none ./scripts/run-qemu.sh   # Headless (check serial log)
#   GDB=1 ./scripts/run-qemu.sh               # With GDB stub on :1234

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
ESP="$PROJECT_DIR/esp"

# ── Check requirements ─────────────────────────────────────────────────────

OVMF_PATH="${OVMF_PATH:-/usr/share/edk2-ovmf/x64/OVMF.4m.fd}"
for p in "$OVMF_PATH" /usr/share/edk2/x64/OVMF.4m.fd /usr/share/OVMF/OVMF.fd; do
    [ -f "$p" ] && { OVMF_PATH="$p"; break; }
done

if [ ! -f "$OVMF_PATH" ]; then
    echo "OVMF not found. Install: sudo pacman -S edk2-ovmf"
    exit 1
fi

if [ ! -f "$ESP/EFI/BOOT/BOOTX64.EFI" ]; then
    echo "Building Farkle..."
    "$SCRIPT_DIR/build.sh"
fi

# ── Display ────────────────────────────────────────────────────────────────

if [ -n "${QEMU_DISPLAY:-}" ]; then
    DISP="$QEMU_DISPLAY"
elif [ -n "${WAYLAND_DISPLAY:-}" ]; then
    DISP="sdl"
elif [ -n "${DISPLAY:-}" ]; then
    DISP="gtk"
else
    DISP="none"
fi

echo "QEMU: OVMF=$OVMF_PATH display=$DISP"
echo "Ctrl+A then X to quit"

# ── Run QEMU ───────────────────────────────────────────────────────────────

QEMU_ARGS=(
    -bios "$OVMF_PATH"
    -drive "format=raw,file=fat:rw:$ESP"
    -display "$DISP"
    -serial file:"$PROJECT_DIR/qemu_serial.log"
    -no-reboot
    -machine pc
    -m 256M
    -cpu qemu64
)

[ "${GDB:-0}" = "1" ] && QEMU_ARGS+=(-s -S) && echo "GDB stub: :1234"

exec qemu-system-x86_64 "${QEMU_ARGS[@]}"
