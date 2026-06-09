#!/bin/bash
# run-qemu-gdb.sh — Start QEMU with GDB stub for remote debugging
#
# 1. Run this script in your desktop terminal (opens QEMU window)
# 2. Claude connects GDB to :1234 to catch crashes
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
ESP="$PROJECT_DIR/esp"
OVMF="/usr/share/edk2-ovmf/x64/OVMF.4m.fd"

# OVMF fallback
for p in "$OVMF" /usr/share/edk2/x64/OVMF.4m.fd; do
    [ -f "$p" ] && { OVMF="$p"; break; }
done

if [ ! -f "$ESP/EFI/BOOT/BOOTX64.EFI" ]; then
    echo "Building..."
    "$SCRIPT_DIR/build.sh"
fi

echo "=== Farkle QEMU (GDB mode) ==="
echo "Game window opening..."
echo "GDB port: :1234 (for Claude to connect)"
echo ""

qemu-system-x86_64 \
    -bios "$OVMF" \
    -drive "format=raw,file=fat:rw:$ESP" \
    -no-reboot -no-shutdown \
    -machine pc -m 256M -cpu qemu64 \
    -d int,cpu_reset -D "$PROJECT_DIR/qemu_crash.log" \
    -s \
    "$@"
