#!/bin/bash
# run-qemu-gdb.sh — Start QEMU with GDB stub for remote debugging
#
# 1. Run this script in your desktop terminal (opens QEMU window)
# 2. Claude connects GDB to :1234 to catch crashes
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
ESP="$PROJECT_DIR/esp"
IMG="$PROJECT_DIR/esp.img"
OVMF_VARS="/tmp/frakle_OVMF_VARS.fd"

OVMF_CODE="/usr/share/edk2/x64/OVMF_CODE.4m.fd"
OVMF_VARS_TEMPLATE="/usr/share/edk2/x64/OVMF_VARS.4m.fd"
for p in "$OVMF_CODE" /usr/share/edk2-ovmf/x64/OVMF_CODE.4m.fd; do
    [ -f "$p" ] && { OVMF_CODE="$p"; break; }
done
for p in "$OVMF_VARS_TEMPLATE" /usr/share/edk2-ovmf/x64/OVMF_VARS.4m.fd; do
    [ -f "$p" ] && { OVMF_VARS_TEMPLATE="$p"; break; }
done

if [ ! -f "$ESP/EFI/BOOT/BOOTX64.EFI" ]; then
    echo "Building..."
    "$SCRIPT_DIR/build.sh"
fi

# Build FAT16 image (supports guest file writes without vvfat crash)
echo "Creating FAT16 image..."
dd if=/dev/zero of="$IMG" bs=1M count=16 status=none
mkfs.fat -F 16 -n "FARKLE" "$IMG" >/dev/null 2>&1
mcopy -s -i "$IMG" "$ESP/EFI" ::EFI

cp "$OVMF_VARS_TEMPLATE" "$OVMF_VARS"

echo "=== Farkle QEMU (GDB mode) ==="
echo "Game window opening..."
echo "GDB port: :1234 (for Claude to connect)"
echo ""

qemu-system-x86_64 \
    -drive "if=pflash,format=raw,readonly=on,file=$OVMF_CODE" \
    -drive "if=pflash,format=raw,file=$OVMF_VARS" \
    -drive "format=raw,file=$IMG,cache=unsafe" \
    -no-reboot -no-shutdown \
    -machine pc,pcspk-audiodev=snd0 -audiodev pa,id=snd0 \
    -device virtio-vga -vga none \
    -m 256M -cpu qemu64 \
    -d int,cpu_reset -D "$PROJECT_DIR/qemu_crash.log" \
    -s \
    "$@"
