#!/usr/bin/env bash
set -euo pipefail
DB_PATH="${DB_PATH:-fyi_system.db}"
FEED_URL="${FEED_URL:-https://fyi.org.nz/search/all/feed}"

fyi-system init-db --db "$DB_PATH"
fyi-system ingest-feed "$FEED_URL" --db "$DB_PATH"
fyi-system reconcile-events --db "$DB_PATH"
fyi-system attention-report --db "$DB_PATH" --output outputs/attention-report.json
fyi-system handover --db "$DB_PATH" --output outputs/handover.md
