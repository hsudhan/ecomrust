#!/usr/bin/env bash
# start_apis.sh — start all eight ecomrust REST APIs in the background.
#
# Builds the binaries once, then launches each service with nohup, logging
# to $LOG_DIR/<service>.log. Ports that are already listening are skipped,
# so the script is safe to re-run. Finishes with a health-checked summary.
#
# Usage:   ./start_apis.sh
# Logs:    tail -f "${TMPDIR:-/tmp}/ecomrust-apis/<service>.log"
# Stop:    pkill -f 'target/debug/.*_api'

set -euo pipefail
cd "$(dirname "$0")"

LOG_DIR="${LOG_DIR:-${TMPDIR:-/tmp}/ecomrust-apis}"
mkdir -p "$LOG_DIR"

SERVICES=(
  "orders_api:4001"
  "shipments_api:4002"
  "users_api:4003"
  "logins_api:4004"
  "shopping_carts_api:4005"
  "payment_infos_api:4006"
  "payments_api:4007"
  "shipment_trackings_api:4008"
)

echo "Building ecomrust binaries..."
cargo build --bins --quiet

echo "Starting APIs (logs: $LOG_DIR)"
: > "$LOG_DIR/apis.pids"
for entry in "${SERVICES[@]}"; do
  bin="${entry%%:*}"
  port="${entry##*:}"
  if lsof -iTCP:"$port" -sTCP:LISTEN >/dev/null 2>&1; then
    echo "  $bin (:$port) already running - skipped"
    continue
  fi
  nohup "./target/debug/$bin" > "$LOG_DIR/$bin.log" 2>&1 &
  echo "$! $bin" >> "$LOG_DIR/apis.pids"
done

# Give the servers a moment to bind, then summarize with a health check.
sleep 1
echo
printf "%-26s %-6s %-9s %s\n" "SERVICE" "PORT" "HEALTH" "LOG"
for entry in "${SERVICES[@]}"; do
  bin="${entry%%:*}"
  port="${entry##*:}"
  health="starting"
  for _ in 1 2 3 4 5 6 7 8 9 10; do
    if curl -sf "localhost:$port/health" >/dev/null 2>&1; then
      health="ok"
      break
    fi
    sleep 0.5
  done
  printf "%-26s %-6s %-9s %s\n" "$bin" "$port" "$health" "$LOG_DIR/$bin.log"
  if [ "$health" != "ok" ]; then
    echo "    last log: $(tail -1 "$LOG_DIR/$bin.log" 2>/dev/null || echo '(no log)')"
  fi
done

echo
echo "PIDs:   $LOG_DIR/apis.pids"
echo "Logs:   tail -f $LOG_DIR/<service>.log"
echo "Stop:   pkill -f 'target/debug/.*_api'"
