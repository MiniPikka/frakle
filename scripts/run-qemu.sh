#!/bin/bash
# run-qemu.sh — Run Farkle in QEMU for testing
#
# Usage:
#   ./scripts/run-qemu.sh              # Auto-detect display
#   QEMU_DISPLAY=gtk ./scripts/run-qemu.sh
#   QEMU_DISPLAY=sdl ./scripts/run-qemu.sh
#   QEMU_DISPLAY=none ./scripts/run-qemu.sh   # Headless (check serial log)
#   GDB=1 ./scripts/run-qemu.sh               # With GDB stub on :1234
#
# Uses mtools-created FAT16 image + pflash for proper UEFI boot.
# Supports guest file writes (debug log) without QEMU vvfat crash.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
ESP="$PROJECT_DIR/esp"
IMG="$PROJECT_DIR/esp.img"
OVMF_VARS="/tmp/frakle_OVMF_VARS.fd"

# ── Check requirements ─────────────────────────────────────────────────────

OVMF_CODE="${OVMF_CODE:-/usr/share/edk2/x64/OVMF_CODE.4m.fd}"
OVMF_VARS_TEMPLATE="${OVMF_VARS_TEMPLATE:-/usr/share/edk2/x64/OVMF_VARS.4m.fd}"
for p in "$OVMF_CODE" /usr/share/edk2-ovmf/x64/OVMF_CODE.4m.fd; do
    [ -f "$p" ] && { OVMF_CODE="$p"; break; }
done
for p in "$OVMF_VARS_TEMPLATE" /usr/share/edk2-ovmf/x64/OVMF_VARS.4m.fd; do
    [ -f "$p" ] && { OVMF_VARS_TEMPLATE="$p"; break; }
done

if [ ! -f "$OVMF_CODE" ]; then
    echo "OVMF not found. Install: sudo pacman -S edk2-ovmf"
    exit 1
fi

if [ ! -f "$ESP/EFI/BOOT/BOOTX64.EFI" ]; then
    echo "Building Farkle..."
    "$SCRIPT_DIR/build.sh"
fi

for cmd in mkfs.fat mcopy; do
    if ! command -v "$cmd" &>/dev/null; then
        echo "Missing $cmd. Install: sudo pacman -S dosfstools mtools"
        exit 1
    fi
done

# ── Build FAT16 image ─────────────────────────────────────────────────────
# FAT16 is required — OVMF doesn't boot from FAT32 raw images.
# FAT16 works fine and supports guest file writes (debug log).

echo "Creating FAT16 image..."
dd if=/dev/zero of="$IMG" bs=1M count=16 status=none
mkfs.fat -F 16 -n "FARKLE" "$IMG" >/dev/null 2>&1
mcopy -s -i "$IMG" "$ESP/EFI" ::EFI

# Fresh OVMF_VARS (avoids stale boot entries)
cp "$OVMF_VARS_TEMPLATE" "$OVMF_VARS"

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

echo "QEMU: OVMF=$OVMF_CODE display=$DISP"
echo "Ctrl+A then X to quit"
echo "Debug log: $IMG -> \\frakle_debug.log (read with: mcopy -i $IMG ::frakle_debug.log /dev/stdout)"

# ── Run QEMU ───────────────────────────────────────────────────────────────

QEMU_ARGS=(
    -drive "if=pflash,format=raw,readonly=on,file=$OVMF_CODE"
    -drive "if=pflash,format=raw,file=$OVMF_VARS"
    -drive "format=raw,file=$IMG,cache=unsafe"
    -display "$DISP"
    -serial file:"$PROJECT_DIR/qemu_serial.log"
    -d int,cpu_reset
    -D "$PROJECT_DIR/qemu_crash.log"
    -no-reboot
    -no-shutdown
    -machine pc,pcspk-audiodev=snd0
    -audiodev pa,id=snd0
    -device virtio-vga
    -vga none
    -m 256M
    -cpu qemu64
)

[ "${GDB:-0}" = "1" ] && QEMU_ARGS+=(-s -S) && echo "GDB stub: :1234"

exec qemu-system-x86_64 "${QEMU_ARGS[@]}"
