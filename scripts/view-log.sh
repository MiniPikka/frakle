#!/bin/bash
# view-log.sh — View the Farkle debug log from USB drive
#
# Usage:
#   ./scripts/view-log.sh

set -e

echo "╔═══════════════════════════════════════════════════════════════╗"
echo "║         Farkle Debug Log Viewer                             ║"
echo "╚═══════════════════════════════════════════════════════════════╝"
echo ""

# Find USB drive
USB_DEV=$(lsblk -o NAME,SIZE,MODEL,TRAN | grep -E "usb|USB" | awk '{print $1}' | head -1)

if [ -z "$USB_DEV" ]; then
    echo "❌ No USB drive found!"
    echo ""
    echo "Please insert the USB drive and try again."
    exit 1
fi

echo "✅ Found USB drive: /dev/$USB_DEV"

# Try to find mount point
MOUNT_POINT=""

# Check common mount locations
for dir in /media /run/media /mnt; do
    if [ -d "$dir" ]; then
        # Find FARKLE volume
        FARKLE_DIR=$(find "$dir" -maxdepth 3 -type d -iname "*farkle*" 2>/dev/null | head -1)
        if [ -n "$FARKLE_DIR" ]; then
            MOUNT_POINT="$FARKLE_DIR"
            break
        fi
    fi
done

# If not found, try to mount
if [ -z "$MOUNT_POINT" ]; then
    echo ""
    echo "⚠️  USB drive not mounted. Attempting to mount..."
    echo ""

    # Try to mount without sudo (using udisks2)
    if command -v udisksctl &>/dev/null; then
        echo "Using udisks2 to mount..."
        udisksctl mount -b /dev/${USB_DEV}1 2>/dev/null || true

        # Wait a moment
        sleep 1

        # Find mount point
        MOUNT_POINT=$(findmnt -n -o TARGET /dev/${USB_DEV}1 2>/dev/null)
    fi

    # If still not found, ask user
    if [ -z "$MOUNT_POINT" ]; then
        echo "❌ Could not auto-mount USB drive"
        echo ""
        echo "Please manually mount the USB drive:"
        echo "  1. Open file manager"
        echo "  2. Click on the USB drive"
        echo "  3. Note the mount path (usually /media/username/FARKLE)"
        echo "  4. Run this script again"
        echo ""
        echo "Or mount manually:"
        echo "  sudo mount /dev/${USB_DEV}1 /mnt"
        echo "  cat /mnt/frakle_debug.log"
        exit 1
    fi
fi

echo "📁 Mount point: $MOUNT_POINT"
echo ""

# Look for log file
LOG_FILE="$MOUNT_POINT/frakle_debug.log"

if [ ! -f "$LOG_FILE" ]; then
    echo "❌ Log file not found: $LOG_FILE"
    echo ""
    echo "Possible reasons:"
    echo "  1. Game hasn't been run yet"
    echo "  2. Log file was deleted"
    echo "  3. Game crashed before creating log"
    echo ""
    echo "Try running the game first, then check again."
    exit 1
fi

echo "📄 Log file found: $LOG_FILE"
echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "                    FARKLE DEBUG LOG"
echo "═══════════════════════════════════════════════════════════════"
echo ""

# Display log contents
cat "$LOG_FILE"

echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "                    END OF LOG"
echo "═══════════════════════════════════════════════════════════════"
echo ""
echo "💡 Tips:"
echo "  - Look for PANIC messages at the end"
echo "  - Check last score values"
echo "  - Check last phase transition"
echo "  - Look for any error messages"
echo ""
echo "📊 Log file size: $(wc -c < "$LOG_FILE") bytes"
echo "📊 Total lines: $(wc -l < "$LOG_FILE")"