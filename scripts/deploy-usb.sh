#!/bin/bash
# deploy-usb.sh — Deploy Farkle to a USB drive for real hardware testing
#
# Usage:
#   ./scripts/deploy-usb.sh              # interactive device selection
#   USB_DEV=/dev/sdX ./scripts/deploy-usb.sh  # specify device directly
#
# WARNING: This script will DESTROY all data on the selected USB device.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
ESP_PATH="$PROJECT_DIR/esp"

# ── Colors ───────────────────────────────────────────────────────────────────

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

info()  { echo -e "${GREEN}[+]${NC} $*"; }
warn()  { echo -e "${YELLOW}[!]${NC} $*"; }
error() { echo -e "${RED}[-]${NC} $*" >&2; }
step()  { echo -e "${CYAN}[*]${NC} $*"; }

# ── Pre-flight checks ───────────────────────────────────────────────────────

if [ "$(id -u)" -ne 0 ]; then
    error "This script must be run as root (need block device access)."
    error "Re-run with: sudo $0 $*"
    exit 1
fi

for cmd in sgdisk mkfs.fat mount umount; do
    if ! command -v "$cmd" &>/dev/null; then
        error "Required command not found: $cmd"
        error "Install with: pacman -S gptfdisk dosfstools (Arch)"
        error "             apt install gdisk dosfstools (Debian/Ubuntu)"
        exit 1
    fi
done

# ── Build first ──────────────────────────────────────────────────────────────

step "Building Farkle..."
"$SCRIPT_DIR/build.sh"
echo ""

# ── Select target device ─────────────────────────────────────────────────────

if [ -z "${USB_DEV:-}" ]; then
    step "Available block devices:"
    echo ""
    lsblk -o NAME,SIZE,MODEL,TRAN,MOUNTPOINT | grep -E "usb|USB" --color=never || true
    echo ""
    lsblk -o NAME,SIZE,MODEL,TRAN | grep -v "loop\|sr\|zram" --color=never
    echo ""
    read -rp "Enter target device (e.g., /dev/sdb): " USB_DEV
fi

# Validate device exists
if [ ! -b "$USB_DEV" ]; then
    error "Block device not found: $USB_DEV"
    exit 1
fi

# Refuse to operate on the system disk
ROOT_PART=$(findmnt -n -o SOURCE /)
if [[ "$ROOT_PART" =~ ^/dev/nvme ]]; then
    ROOT_DEV=$(echo "$ROOT_PART" | sed 's/p[0-9]*$//')
else
    ROOT_DEV=$(echo "$ROOT_PART" | sed 's/[0-9]*$//')
fi
if [ "$USB_DEV" = "$ROOT_DEV" ]; then
    error "Refusing to operate on $USB_DEV — this is your system disk!"
    error "Double-check with: lsblk"
    exit 1
fi

# ── Confirm ──────────────────────────────────────────────────────────────────

echo ""
warn "═══════════════════════════════════════════════════════════════"
warn "  ALL DATA ON $USB_DEV WILL BE DESTROYED!"
warn "═══════════════════════════════════════════════════════════════"
echo ""
lsblk -o NAME,SIZE,MODEL "$USB_DEV" 2>/dev/null || true
echo ""
read -rp "Type 'YES' to confirm: " CONFIRM
if [ "$CONFIRM" != "YES" ]; then
    info "Aborted."
    exit 0
fi

# ── Unmount any mounted partitions ───────────────────────────────────────────

step "Unmounting any mounted partitions on $USB_DEV..."
for part in "${USB_DEV}"*; do
    if mountpoint -q "$part" 2>/dev/null; then
        umount "$part"
        info "Unmounted $part"
    fi
done

# Also try to unmount using findmnt
if findmnt -r "$USB_DEV" >/dev/null 2>&1; then
    umount "$USB_DEV" 2>/dev/null || true
fi

# ── Partition ────────────────────────────────────────────────────────────────

step "Creating GPT partition table on $USB_DEV..."
# Wipe existing partition signatures
wipefs -a "$USB_DEV" >/dev/null 2>&1 || true
dd if=/dev/zero of="$USB_DEV" bs=1M count=16 status=none 2>/dev/null || true
dd if=/dev/zero of="$USB_DEV" bs=512 count=2048 seek=$(( $(blockdev --getsz "$USB_DEV") - 2048 )) status=none 2>/dev/null || true
sync
partprobe "$USB_DEV" 2>/dev/null || true
sleep 1
sgdisk --zap-all "$USB_DEV" 2>/dev/null || true
sgdisk --new=1:0:0 --typecode=1:ef00 --change-name=1:"Farkle ESP" "$USB_DEV"
info "Partition created: ${USB_DEV}1"

# ── Format ───────────────────────────────────────────────────────────────────

step "Formatting ${USB_DEV}1 as FAT32..."
sync
partprobe "$USB_DEV" 2>/dev/null || true
for i in $(seq 1 10); do
    [ -b "${USB_DEV}1" ] && break
    sleep 0.5
done
if [ ! -b "${USB_DEV}1" ]; then
    error "Partition ${USB_DEV}1 did not appear. Try: partprobe $USB_DEV"
    exit 1
fi

# Force unmount if still mounted
if mountpoint -q "${USB_DEV}1" 2>/dev/null; then
    umount -f "${USB_DEV}1" 2>/dev/null || true
fi

wipefs -a "${USB_DEV}1" >/dev/null 2>&1 || true
mkfs.fat -F 32 -n "FARKLE" "${USB_DEV}1"
info "Formatted as FAT32 (label: FARKLE)"

# ── Copy files ───────────────────────────────────────────────────────────────

step "Copying EFI files..."
MNT_POINT=$(mktemp -d)

mount "${USB_DEV}1" "$MNT_POINT"
cp -rv "$ESP_PATH"/EFI "$MNT_POINT/"
umount "$MNT_POINT"
rmdir "$MNT_POINT"

info "EFI files copied to USB drive"

# ── Summary ──────────────────────────────────────────────────────────────────

echo ""
echo "═══════════════════════════════════════════════════════════════"
info "Deployed: Farkle dice game"
info "Files on USB:"
info "  EFI/BOOT/BOOTX64.EFI    (auto-loaded by firmware)"
info "  EFI/Farkle/Farkle.efi   (backup)"
echo "═══════════════════════════════════════════════════════════════"
echo ""
info "USB drive $USB_DEV is ready!"
info ""
info "Next steps:"
info "  1. Safely eject: eject $USB_DEV"
info "  2. Insert into target machine"
info "  3. Enter BIOS/UEFI setup → set USB as first boot device"
info "  4. Disable Secure Boot (if not already)"
info "  5. Boot and play Farkle!"