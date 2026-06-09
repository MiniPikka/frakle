#!/bin/bash
# quick-deploy.sh — One-command deployment to /dev/sda
#
# Usage:
#   sudo ./scripts/quick-deploy.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "╔═══════════════════════════════════════════════════════════════╗"
echo "║         Farkle UEFI Game - Quick Deploy to /dev/sda         ║"
echo "╚═══════════════════════════════════════════════════════════════╝"
echo ""

# Check if running as root
if [ "$(id -u)" -ne 0 ]; then
    echo "❌ This script must be run as root"
    echo ""
    echo "Run with: sudo $0"
    exit 1
fi

# Check if /dev/sda exists and is USB
if [ ! -b /dev/sda ]; then
    echo "❌ /dev/sda not found"
    echo ""
    echo "Available devices:"
    lsblk -o NAME,SIZE,MODEL,TRAN | grep -v "loop\|sr\|zram"
    exit 1
fi

# Show device info
echo "Target device:"
lsblk -o NAME,SIZE,MODEL /dev/sda
echo ""

# Confirm
read -rp "⚠️  This will DESTROY all data on /dev/sda. Continue? (yes/no): " CONFIRM
if [ "$CONFIRM" != "yes" ]; then
    echo "Aborted."
    exit 0
fi

echo ""
echo "🚀 Deploying Farkle to /dev/sda..."
echo ""

# Deploy
USB_DEV=/dev/sda "$SCRIPT_DIR/deploy-usb.sh"

echo ""
echo "✅ Deployment complete!"
echo ""
echo "📋 Next steps:"
echo "  1. Safely eject: eject /dev/sda"
echo "  2. Insert into target machine"
echo "  3. Enter BIOS/UEFI setup"
echo "  4. Set USB as first boot device"
echo "  5. Disable Secure Boot (if needed)"
echo "  6. Boot and test!"
echo ""
echo "🎯 Test focus:"
echo "  - Frame counter (green, top-left)"
echo "  - Score display (yellow, top-left)"
echo "  - Black screen near 5000 points"
echo ""
echo "💡 If black screen occurs:"
echo "  - Restart computer"
echo "  - Remove USB drive"
echo "  - Check \frakle_debug.log on USB"
echo ""