#!/bin/sh
# Runs event.sh against a stub herdr, a temp hooks.conf and fake events. `SH=dash sh test.sh` runs it under dash.
# Every case runs twice: with jq (when installed) and with a broken jq first on PATH, so the sed path is covered.
SH=$(command -v "${SH:-sh}")
PLUGIN=$(cd "$(dirname "$0")" && pwd)
FAILED=0

ok() { echo "ok $MODE $CASE"; }
fail() { echo "FAIL $MODE $CASE: $*"; FAILED=1; }

# A fresh world: $MAIN (main checkout), $WT (a linked worktree), a stub herdr and login shell, an empty hooks.conf.
setup() {
  T=$(mktemp -d)
  MAIN="$T/main"; WT="$T/wt"
  [ -n "$SPACES" ] && { MAIN="$T/main repo"; WT="$T/the wt"; }
  mkdir -p "$MAIN" "$WT" "$T/conf" "$T/state" "$T/bin"
  : > "$WT/.git"
  export LOG="$T/ran.log" STUB_DIR="$T"
  : > "$LOG"; : > "$T/calls.log"
  cat > "$T/herdr" <<'EOF'
#!/bin/sh
echo "$*" >> "$STUB_DIR/calls.log"
case "$1 $2" in "workspace get") cat "$STUB_DIR/ws.json" ;; esac
EOF
  # an interactive login shell prints junk before the marker; $STUB_PATH is the PATH it reports
  cat > "$T/login" <<'EOF'
#!/bin/sh
echo "Last login: today"
printf '__SHEPHERD_PATH__=%s\n' "$STUB_PATH"
EOF
  chmod +x "$T/herdr" "$T/login"
  export STUB_PATH="$TEST_PATH"
  ws_json 1 1 true
}

conf() { printf '%s\n' "$@" > "$T/conf/hooks.conf"; }

project() { conf "[main]" "dir = $MAIN" "$@"; }

# What `herdr workspace get` answers: tabs, panes, linked.
ws_json() {
  printf '{"result":{"type":"workspace_info","workspace":{"workspace_id":"w1","tab_count":%s,"pane_count":%s,"worktree":{"checkout_path":"%s","is_linked_worktree":%s,"repo_key":"%s/.git","repo_name":"main","repo_root":"%s"}}}}' \
    "$1" "$2" "$WT" "$3" "$MAIN" "$MAIN" > "$T/ws.json"
}

created_json() { # [linked] [repo_root]
  linked=${1:-true}; root=${2-$MAIN}
  printf '{"type":"worktree_created","workspace":{"workspace_id":"w1","tab_count":1,"pane_count":1,"worktree":{"checkout_path":"%s","is_linked_worktree":%s,"repo_name":"main"%s}},"worktree":{"path":"%s","is_linked_worktree":%s,"branch":"b"}}' \
    "$WT" "$linked" "${root:+,\"repo_root\":\"$root\"}" "$WT" "$linked"
}
focused_json() { printf '{"type":"workspace_focused","workspace_id":"%s"}' "${1:-w1}"; }
removed_json() { printf '{"type":"worktree_removed","workspace_id":"w1","forced":false,"worktree":{"path":"%s","is_linked_worktree":true,"branch":"b"}}' "$WT"; }

# event <name> <json> [socket]
event() {
  (cd "$PLUGIN" && env HERDR_PLUGIN_EVENT="$1" HERDR_PLUGIN_EVENT_JSON="$2" HERDR_SOCKET_PATH="${3:-/tmp/a.sock}" \
    HERDR_PLUGIN_CONFIG_DIR="$T/conf" HERDR_PLUGIN_STATE_DIR="$T/state" HERDR_BIN_PATH="$T/herdr" \
    SHELL="$T/login" PATH="$EVENT_PATH" "$SH" event.sh 2>>"$T/stderr.log")
}

lines() { grep -c "$1" "$2" 2>/dev/null || true; }

run_cases() {
  CASE=created_runs_once; setup
  project 'created = echo "$EVENT $PROJECT $PWD" >> "$LOG"'
  event worktree.created "$(created_json)"; event workspace.created "$(created_json)"
  [ "$(cat "$LOG")" = "created main $WT" ] && ok || fail "ran.log: $(cat "$LOG")"

  CASE=focus_path_queries_herdr; setup
  project 'created = echo "$EVENT $PWD" >> "$LOG"'
  event workspace.focused "$(focused_json)"
  [ "$(cat "$LOG")" = "created $WT" ] && [ "$(lines 'workspace get' "$T/calls.log")" = 1 ] && ok || fail "ran: $(cat "$LOG") calls: $(cat "$T/calls.log")"

  CASE=focus_on_arranged_worktree_skips; setup
  project 'created = echo ran >> "$LOG"'
  ws_json 1 2 true
  event workspace.focused "$(focused_json)"; event workspace.focused "$(focused_json)"
  [ ! -s "$LOG" ] && [ "$(lines 'workspace get' "$T/calls.log")" = 1 ] && ok || fail "ran: $(cat "$LOG") calls: $(cat "$T/calls.log")"

  CASE=main_checkout_skips; setup
  project 'created = echo ran >> "$LOG"'
  event worktree.created "$(created_json false)"
  [ ! -s "$LOG" ] && ok || fail "ran for the main checkout"

  CASE=same_id_other_session_runs; setup
  project 'created = echo ran >> "$LOG"'
  ws_json 1 2 true
  event workspace.focused "$(focused_json)" /tmp/a.sock
  ws_json 1 1 true
  event workspace.focused "$(focused_json)" /tmp/b.sock
  [ "$(lines ran "$LOG")" = 1 ] && ok || fail "ran $(lines ran "$LOG") times"

  CASE=remade_at_same_path_runs; setup
  project 'created = echo ran >> "$LOG"'
  event worktree.created "$(created_json)"
  sleep 1; rm "$WT/.git"; : > "$WT/.git"
  event worktree.created "$(created_json)"
  [ "$(lines ran "$LOG")" = 2 ] && ok || fail "ran $(lines ran "$LOG") times"

  CASE=failure_releases_claim; setup
  project 'created = echo try >> "$LOG"' 'created = false'
  event worktree.created "$(created_json)"; event worktree.created "$(created_json)"
  [ "$(lines try "$LOG")" = 2 ] && ok || fail "tried $(lines try "$LOG") times"

  CASE=stops_at_first_failure; setup
  project 'created = false' 'created = echo after >> "$LOG"'
  event worktree.created "$(created_json)"
  [ ! -s "$LOG" ] && ok || fail "went on after a failure"

  CASE=removed_runs_from_main; setup
  project 'created = :' 'removed = echo "$EVENT $PROJECT $PWD $WORKTREE" >> "$LOG"'
  event worktree.created "$(created_json)"; event worktree.removed "$(removed_json)"
  [ "$(cat "$LOG")" = "removed main $MAIN $WT" ] && [ -z "$(ls -A "$T/state/applied")$(ls -A "$T/state/map")" ] && ok || fail "ran: $(cat "$LOG") state: $(ls -R "$T/state")"

  CASE=match_by_basename; setup
  project 'created = echo ran >> "$LOG"'
  event worktree.created "$(created_json true "")"
  [ "$(lines ran "$LOG")" = 1 ] && ok || fail "no match by repo name"

  CASE=command_verbatim; setup
  project "created = [ \"\$(printf '%s' \"a b\")\" = \"a b\" ] && echo q >> \"\$LOG\" || true"
  event worktree.created "$(created_json)"
  [ "$(cat "$LOG")" = q ] && ok || fail "ran.log: $(cat "$LOG")"

  CASE=paths_with_spaces; SPACES=1; setup; SPACES=
  project 'created = echo "$PWD" >> "$LOG"'
  event worktree.created "$(created_json)"
  [ "$(cat "$LOG")" = "$WT" ] && ok || fail "ran.log: $(cat "$LOG")"

  CASE=path_from_login_shell; setup
  printf '#!/bin/sh\n' > "$T/bin/fakepnpm"; chmod +x "$T/bin/fakepnpm"
  export STUB_PATH="$T/bin:$TEST_PATH"
  project 'created = command -v fakepnpm >> "$LOG"'
  event worktree.created "$(created_json)"
  [ "$(cat "$LOG")" = "$T/bin/fakepnpm" ] && ok || fail "ran.log: $(cat "$LOG")"

  CASE=preexisting_worktree_skips; setup
  project 'created = echo ran >> "$LOG"'
  sleep 1; : > "$T/conf/.installed"
  event workspace.created "$(created_json)"
  [ ! -s "$LOG" ] && ok || fail "ran for a worktree made before the install"

  CASE=worktree_after_install_runs; setup
  project 'created = echo ran >> "$LOG"'
  : > "$T/conf/.installed"; sleep 1; rm "$WT/.git"; : > "$WT/.git"
  event worktree.created "$(created_json)"
  [ "$(lines ran "$LOG")" = 1 ] && ok || fail "ran $(lines ran "$LOG") times"

  CASE=failed_query_retries; setup
  project 'created = echo ran >> "$LOG"'
  : > "$T/ws.json"
  event workspace.focused "$(focused_json)"
  ws_json 1 1 true
  event workspace.focused "$(focused_json)"
  [ "$(lines ran "$LOG")" = 1 ] && ok || fail "ran $(lines ran "$LOG") times"

  CASE=session_project_runs_in_its_session; setup
  conf "[demo/main]" "dir = $MAIN" 'created = echo "$PROJECT" >> "$LOG"'
  event worktree.created "$(created_json)" /x/sessions/demo/herdr.sock
  [ "$(cat "$LOG")" = main ] && ok || fail "ran.log: $(cat "$LOG")"

  CASE=session_project_skips_other_sessions; setup
  conf "[demo/main]" "dir = $MAIN" 'created = echo ran >> "$LOG"'
  event worktree.created "$(created_json)" /x/herdr.sock
  event worktree.created "$(created_json)" /x/sessions/work/herdr.sock
  [ ! -s "$LOG" ] && ok || fail "ran outside its session"

  CASE=default_session_project_runs; setup
  conf "[default/main]" "dir = $MAIN" 'created = echo ran >> "$LOG"'
  event worktree.created "$(created_json)" /x/herdr.sock
  [ "$(lines ran "$LOG")" = 1 ] && ok || fail "ran $(lines ran "$LOG") times"

  CASE=no_projects_exits_quietly; setup
  conf "# nothing yet"
  event workspace.focused "$(focused_json)" && [ ! -s "$T/calls.log" ] && ok || fail "calls: $(cat "$T/calls.log")"
}

TEST_PATH=/usr/bin:/bin
if command -v jq >/dev/null 2>&1; then
  MODE=jq; EVENT_PATH="$(dirname "$(command -v jq)"):$TEST_PATH"; run_cases
fi
NOJQ=$(mktemp -d); printf '#!/bin/sh\nexit 2\n' > "$NOJQ/jq"; chmod +x "$NOJQ/jq"
MODE=sed; EVENT_PATH="$NOJQ:$TEST_PATH"; run_cases
exit $FAILED
