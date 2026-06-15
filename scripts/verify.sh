#!/bin/bash
# verify.sh — Full verification pipeline for Farkle
#
# Runs: build → clippy → game logic tests → (optional) QEMU smoke test
#
# Usage:
#   bash scripts/verify.sh              # build + clippy + tests
#   QEMU=1 bash scripts/verify.sh       # also run QEMU smoke test (needs display)

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

export RUSTUP_TOOLCHAIN=nightly
cd "$PROJECT_DIR"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

pass() { echo -e "${GREEN}✓${NC} $*"; }
fail() { echo -e "${RED}✗${NC} $*"; exit 1; }
step() { echo -e "${YELLOW}[$1/4]${NC} $*"; }

ERRORS=0

# 1. Build
step 1 "Building release..."
if cargo build --release --target x86_64-unknown-uefi 2>&1 | tail -3; then
    pass "Release build"
else
    fail "Release build failed"
fi

# 2. Clippy
step 2 "Running clippy..."
CLIPPY_OUT=$(cargo clippy --target x86_64-unknown-uefi 2>&1)
WARNINGS=$(echo "$CLIPPY_OUT" | grep -c "warning\[" || true)
if [ "$WARNINGS" -eq 0 ]; then
    pass "Clippy: zero warnings"
else
    echo "$CLIPPY_OUT" | grep "warning\["
    fail "Clippy: $WARNINGS warning(s)"
fi

# 3. Game logic tests (has own .cargo/config.toml overriding UEFI target)
step 3 "Running game logic tests..."
if (cd game_test && cargo run 2>&1 | tail -5); then
    pass "Game logic tests"
else
    fail "Game logic tests failed"
fi

# 4. Binary size check
EFI_SIZE=$(stat -c%s "esp/EFI/BOOT/BOOTX64.EFI" 2>/dev/null || echo 0)
EFI_KB=$((EFI_SIZE / 1024))
if [ "$EFI_KB" -gt 0 ]; then
    pass "Binary: ${EFI_KB}KB (esp/EFI/BOOT/BOOTX64.EFI)"
fi

# Optional QEMU smoke test
if [ "${QEMU:-0}" = "1" ]; then
    step 4 "QEMU smoke test (5 seconds)..."
    "$SCRIPT_DIR/run-qemu.sh" &
    QEMU_PID=$!
    sleep 5
    if ps -p $QEMU_PID > /dev/null 2>&1; then
        kill $QEMU_PID 2>/dev/null || true
        pass "QEMU smoke test: ran 5s without crash"
    else
        fail "QEMU smoke test: process exited early"
    fi
else
    echo ""
    echo "Skip QEMU smoke test (set QEMU=1 to enable)"
fi

echo ""
echo -e "${GREEN}All checks passed.${NC}"
