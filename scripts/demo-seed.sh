#!/usr/bin/env bash
# Rebuild a throwaway demo DB from fixtures/demo-seed.sql.
# Used by every VHS tape in assets/tapes/ so recordings start from an
# identical, rich snapshot of the app.
#
# Tapes export HOME=<repo>/.demo-home so the binary writes its
# ~/.rondo-rs/ tree under the throwaway dir and the user's real data is
# never touched.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEMO_HOME="${RONDO_DEMO_HOME:-$ROOT/.demo-home}"
RONDO_DIR="$DEMO_HOME/.rondo-rs"
DEMO_DB="$RONDO_DIR/todo.db"
SEED="$ROOT/fixtures/demo-seed.sql"

rm -rf "$DEMO_HOME"
mkdir -p "$RONDO_DIR/backups" "$RONDO_DIR/logs" "$RONDO_DIR/plugins" "$RONDO_DIR/lang"

sqlite3 "$DEMO_DB" < "$SEED"

cat > "$RONDO_DIR/config.toml" <<'TOML'
[ui]
theme = "dark"
sidebar = true
animations = true
language = "en"

[pomodoro]
work_min = 25
short_break_min = 5
long_break_min = 15
cycles_per_long = 4

[plugins]
enabled = []

[plugins.permissions]
TOML

echo "Demo HOME ready: $DEMO_HOME"
