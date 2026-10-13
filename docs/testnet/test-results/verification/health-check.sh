#!/usr/bin/env bash
# health-check.sh — Quick verification of all testnet services
#
# Checks that all systemd services are running and responsive.
# Run as root or with sudo after deployment.
#
# Usage:
#   sudo bash health-check.sh

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

check() {
    local name="$1"
    local cmd="$2"
    if eval "$cmd" &>/dev/null; then
        pass "$name"
        ((PASS++)) || true
    else
        fail "$name"
        ((FAIL++)) || true
    fi
}

echo "========================================"
echo "  OpenDex Testnet Health Check"
echo "  $(date '+%Y-%m-%d %H:%M:%S')"
echo "========================================"
echo ""

# --- Systemd services ---
info "Checking systemd services..."
check "Bootstrap node running" "systemctl is-active opendex-bootstrap"
check "Test node 1 running" "systemctl is-active opendex-testnode1"
check "Test node 2 running" "systemctl is-active opendex-testnode2"
check "Monero stagenet running" "systemctl is-active monero-stagenet"
check "Bitcoin testnet running" "systemctl is-active bitcoin-testnet"
check "Nginx running" "systemctl is-active nginx"
echo ""

# --- Port checks ---
info "Checking listening ports..."
check "Bootstrap node port 4001" "ss -tlnp | grep -q ':4001 '"
check "Test node 1 port 4002" "ss -tlnp | grep -q ':4002 '"
check "Test node 2 port 4003" "ss -tlnp | grep -q ':4003 '"
check "Monero RPC port 18082" "ss -tlnp | grep -q ':18082 '"
check "Bitcoin RPC port 18332" "ss -tlnp | grep -q ':18332 '"
check "Nginx HTTP port 80" "ss -tlnp | grep -q ':80 '"
echo ""

# --- Wallet RPC checks ---
info "Checking wallet RPC endpoints..."
check "Monero wallet RPC responds" "curl -s --max-time 5 http://127.0.0.1:18082/json_rpc -d '{\"jsonrpc\":\"2.0\",\"id\":\"test\",\"method\":\"getbalance\",\"params\":{\"wallet_file\":\"test\"}}' -H 'Content-Type: application/json' 2>/dev/null | grep -q 'result\|error'"
check "Bitcoin RPC responds" "bitcoin-cli -testnet -rpcuser=opendex -rpcpassword=opendex_testnet getblockcount 2>/dev/null"
echo ""

# --- Web portal check ---
info "Checking web portal..."
check "Web portal HTTP responds" "curl -s --max-time 5 -o /dev/null -w '%{http_code}' http://127.0.0.1/ | grep -qE '200|301|302'"
echo ""

# --- Node logs check ---
info "Checking recent node logs..."
check "Bootstrap node has recent logs" "journalctl -u opendex-bootstrap --no-pager -n 50 | grep -q 'Listening\|Connected\|gossipsub'"
echo ""

# --- Summary ---
TOTAL=$((PASS + FAIL))
echo "========================================"
echo "  Results: ${PASS}/${TOTAL} passed, ${FAIL} failed"
echo "========================================"

if [[ $FAIL -gt 0 ]]; then
    echo ""
    fail "Some checks failed. Check service status with:"
    echo "  systemctl status <service-name>"
    echo "  journalctl -u <service-name> -f"
    exit 1
else
    pass "All health checks passed!"
    exit 0
fi
