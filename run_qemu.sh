#!/bin/bash
# Run Farkle game in QEMU with OVMF firmware
set -e

EFI_FILE="target/x86_64-unknown-uefi/release/frakle.efi"
FAT_IMG="test_fat.img"
OVMF_CODE="/usr/share/OVMF/x64/OVMF.4m.fd"

if [ ! -f "$EFI_FILE" ]; then
    echo "Building game..."
    cargo build --release
fi

echo "Creating FAT image..."
mkdir -p test_image
cp "$EFI_FILE" test_image/

cat > test_image/startup.nsh << 'EOF'
@echo -off
echo ====================
echo   F A R K L E
echo   UEFI Dice Game
echo ====================
echo.
echo Starting game...
frakle.efi
echo.
echo Game exited.
pause
EOF

dd if=/dev/zero of="$FAT_IMG" bs=1M count=64 status=none
mkfs.fat -F 32 "$FAT_IMG" > /dev/null 2>&1

# Use mmd/mcopy to create EFI boot structure
mmd -i "$FAT_IMG" ::/EFI 2>/dev/null || true
mmd -i "$FAT_IMG" ::/EFI/BOOT 2>/dev/null || true
mcopy -i "$FAT_IMG" test_image/* ::/ > /dev/null 2>&1
mcopy -i "$FAT_IMG" "$EFI_FILE" ::/EFI/BOOT/BOOTX64.EFI > /dev/null 2>&1

echo "Starting QEMU..."
qemu-system-x86_64 \
    -bios "$OVMF_CODE" \
    -drive file="$FAT_IMG",format=raw,if=ide \
    -m 512M \
    -vga std \
    -display gtk \
    -net none \
    -machine pcspk-audiodev=snd0 \
    -audiodev pa,id=snd0 \
    -name "Farkle UEFI Game" \
    "$@"

rm -rf test_image
echo "Done."
