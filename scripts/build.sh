#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# Preserve original user's rustup config when running as root
if [ "$(id -u)" -eq 0 ] && [ -n "${SUDO_USER:-}" ]; then
    REAL_USER="$SUDO_USER"
    REAL_HOME=$(eval echo "~$REAL_USER")
    export RUSTUP_HOME="$REAL_HOME/.rustup"
    export CARGO_HOME="$REAL_HOME/.cargo"
fi

echo "[*] Building Farkle..."
cd "$PROJECT_DIR"

# Ensure we use nightly toolchain for UEFI
export RUSTUP_TOOLCHAIN=nightly
cargo build --release --target x86_64-unknown-uefi

mkdir -p esp/EFI/BOOT esp/EFI/Farkle

# Copy the UEFI application
cp target/x86_64-unknown-uefi/release/frakle.efi esp/EFI/BOOT/BOOTX64.EFI
cp target/x86_64-unknown-uefi/release/frakle.efi esp/EFI/Farkle/Farkle.efi

echo "[+] Built: esp/EFI/BOOT/BOOTX64.EFI"
echo "[+] Built: esp/EFI/Farkle/Farkle.efi"

# Optional: deploy to USB
if [ "${1:-}" = "--deploy-usb" ]; then
    echo ""
    exec "$SCRIPT_DIR/deploy-usb.sh"
fi