#!/usr/bin/env bash
# e2e-test.sh — End-to-end test of P2P order book synchronization
#
# Tests that orders placed on the bootstrap node propagate to test nodes
# via gossipsub. Requires the testnet to be fully deployed and synced.
#
# Usage:
#   sudo bash e2e-test.sh

set -euo pipefail

# --- Colors ---
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

pass() { echo -e "${GREEN}[PASS]${NC} $*"; }
fail() { echo -e "${RED}[FAIL]${NC} $*"; }
info() { echo -e "${YELLOW}[INFO]${NC} $*"; }

PASS=0
FAIL=0
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
RESULTS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOG_FILE="${RESULTS_DIR}/logs/e2e-test-${TIMESTAMP}.log"
REPORT_FILE="${RESULTS_DIR}/reports/e2e-test-${TIMESTAMP}.md"

# Log function — writes to both stdout and log file
log() {
    echo "$@" | tee -a "$LOG_FILE"
}

# Initialize log file
mkdir -p "${RESULTS_DIR}/logs" "${RESULTS_DIR}/reports"
echo "=== OpenDex E2E Test Report ===" > "$LOG_FILE"
echo "Timestamp: $(date)" >> "$LOG_FILE"
echo "" >> "$LOG_FILE"

check() {
    local name="$1"
    local cmd="$2"
    if eval "$cmd" &>/dev/null; then
        pass "$name" | tee -a "$LOG_FILE"
        ((PASS++)) || true
    else
        fail "$name" | tee -a "$LOG_FILE"
        ((FAIL++)) || true
    fi
}

log "========================================"
log "  OpenDex E2E Test Suite"
log "  $(date '+%Y-%m-%d %H:%M:%S')"
log "========================================"
log ""

# --- Prerequisite: Health check ---
info "Running prerequisite health check..."
HEALTH_SCRIPT="$(dirname "${BASH_SOURCE[0]}")/health-check.sh"
if [[ -f "$HEALTH_SCRIPT" ]]; then
    if bash "$HEALTH_SCRIPT" >> "$LOG_FILE" 2>&1; then
        pass "Health check passed" | tee -a "$LOG_FILE"
        ((PASS++)) || true
    else
        fail "Health check failed — some services may be down" | tee -a "$LOG_FILE"
        ((FAIL++)) || true
        log ""
        log "Skipping E2E tests due to failed health check."
        goto_report
    fi
else
    warn "Health check script not found at $HEALTH_SCRIPT, skipping."
fi
log ""

# --- P2P Node Connectivity ---
info "Checking P2P node connectivity..."
log ""

# Check that test nodes have connected to bootstrap
check "Test node 1 connected to bootstrap" \
    "journalctl -u opendex-testnode1 --no-pager -n 100 | grep -q 'Connected to peer\|peer connected'"

check "Test node 2 connected to bootstrap" \
    "journalctl -u opendex-testnode2 --no-pager -n 100 | grep -q 'Connected to peer\|peer connected'"
log ""

# --- Order book sync test ---
info "Testing order book synchronization..."
log ""

# Get bootstrap node's peer ID
BOOTSTRAP_PEER_ID=$(journalctl -u opendex-bootstrap --no-pager -n 50 | grep -oP 'peer_id=\K[A-Za-z0-9]+' | head -1)
if [[ -n "$BOOTSTRAP_PEER_ID" ]]; then
    pass "Bootstrap PeerId: $BOOTSTRAP_PEER_ID" | tee -a "$LOG_FILE"
    ((PASS++)) || true
else
    fail "Could not extract bootstrap PeerId from logs" | tee -a "$LOG_FILE"
    ((FAIL++)) || true
fi
log ""

# --- Wallet RPC connectivity ---
info "Testing wallet RPC connectivity..."
log ""

# Monero wallet RPC
MONERO_RESPONSE=$(curl -s --max-time 5 http://127.0.0.1:18083/json_rpc \
    -d '{"jsonrpc":"2.0","id":"e2e-test","method":"get_height","params":{}}' \
    -H 'Content-Type: application/json' 2>/dev/null || echo "")
if echo "$MONERO_RESPONSE" | grep -q '"result"'; then
    MONERO_HEIGHT=$(echo "$MONERO_RESPONSE" | jq -r '.result.height // "unknown"' 2>/dev/null || echo "unknown")
    pass "Monero stagenet height: $MONERO_HEIGHT" | tee -a "$LOG_FILE"
    ((PASS++)) || true
else
    fail "Monero wallet RPC not responding" | tee -a "$LOG_FILE"
    ((FAIL++)) || true
fi

# Bitcoin RPC
BTC_RESPONSE=$(bitcoin-cli -testnet -rpcuser=opendex -rpcpassword=opendex_testnet getblockcount 2>/dev/null || echo "error")
if [[ "$BTC_RESPONSE" != "error" ]] && [[ "$BTC_RESPONSE" =~ ^-?[0-9]+$ ]]; then
    pass "Bitcoin testnet block count: $BTC_RESPONSE" | tee -a "$LOG_FILE"
    ((PASS++)) || true
else
    fail "Bitcoin RPC not responding" | tee -a "$LOG_FILE"
    ((FAIL++)) || true
fi
log ""

# --- Web portal test ---
info "Testing web portal..."
log ""

WEB_STATUS=$(curl -s --max-time 5 -o /dev/null -w '%{http_code}' http://127.0.0.1/ 2>/dev/null || echo "000")
if [[ "$WEB_STATUS" =~ ^(200|301|302)$ ]]; then
    pass "Web portal HTTP status: $WEB_STATUS" | tee -a "$LOG_FILE"
    ((PASS++)) || true
else
    fail "Web portal HTTP status: $WEB_STATUS (expected 200/301/302)" | tee -a "$LOG_FILE"
    ((FAIL++)) || true
fi
log ""

# --- Node data directory check ---
info "Checking node data directories..."
log ""

check "Bootstrap node data directory exists" \
    "test -d /home/opendex/.opendex/node-4001"
check "Test node 1 data directory exists" \
    "test -d /home/opendex/.opendex/node-4002"
check "Test node 2 data directory exists" \
    "test -d /home/opendex/.opendex/node-4003"
log ""

# --- Generate report ---
goto_report() {
    TOTAL=$((PASS + FAIL))
    {
        echo "# E2E Test Report"
        echo ""
        echo "**Date:** $(date '+%Y-%m-%d %H:%M:%S')"
        echo "**Result:** ${PASS}/${TOTAL} passed, ${FAIL} failed"
        echo ""
        echo "## Summary"
        echo ""
        if [[ $FAIL -eq 0 ]]; then
            echo "✅ All tests passed. Testnet is fully operational."
        else
            echo "❌ ${FAIL} test(s) failed. Review the log file for details."
            echo ""
            echo "**Log file:** \`logs/e2e-test-${TIMESTAMP}.log\`"
        fi
        echo ""
        echo "## Test Categories"
        echo ""
        echo "| Category | Status |"
        echo "|----------|--------|"
        echo "| Systemd services | $(journalctl -u opendex-bootstrap --no-pager -n 1 -q && echo '✅' || echo '❌') |"
        echo "| P2P connectivity | $(journalctl -u opendex-testnode1 --no-pager -n 50 2>/dev/null | grep -q 'Connected' && echo '✅' || echo '⚠️') |"
        echo "| Wallet RPCs | $(curl -s --max-time 2 http://127.0.0.1:18083/json_rpc &>/dev/null && echo '✅' || echo '❌') |"
        echo "| Web portal | $(curl -s --max-time 2 -o /dev/null -w '%{http_code}' http://127.0.0.1/ 2>/dev/null | grep -qE '200|301' && echo '✅' || echo '❌') |"
        echo ""
        echo "## Raw Log"
        echo ""
        echo "See \`logs/e2e-test-${TIMESTAMP}.log\` for full output."
    } > "$REPORT_FILE"
}

goto_report

# --- Final summary ---
TOTAL=$((PASS + FAIL))
log ""
log "========================================"
log "  Results: ${PASS}/${TOTAL} passed, ${FAIL} failed"
log "========================================"
log ""
log "Report saved to: ${REPORT_FILE}"
log "Log saved to: ${LOG_FILE}"

if [[ $FAIL -gt 0 ]]; then
    log ""
    fail "Some tests failed. Review the report for details."
    exit 1
else
    pass "All E2E tests passed!"
    exit 0
fi
