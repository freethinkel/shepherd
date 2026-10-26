#!/bin/sh
# Shepherd's worktree hooks: runs a project's commands from hooks.conf when herdr makes or removes one of its
# worktrees. POSIX sh only (dash runs it on Debian); jq is used when it works, never required.
#
# hooks.conf, one file for all projects:
#   [name]
#   dir = ~/path/to/main/checkout
#   created = command run in the new worktree    (repeats, in order; the first failure stops the rest)
#   removed = command run from the main checkout (repeats)
# Commands run with sh -c and $PROJECT, $MAIN, $WORKTREE, $EVENT in the environment.

conf="${HERDR_PLUGIN_CONFIG_DIR:?}/hooks.conf"
state="${HERDR_PLUGIN_STATE_DIR:-${TMPDIR:-/tmp}/shepherd-worktree-hooks}"
herdr="${HERDR_BIN_PATH:-herdr}"
event="${HERDR_PLUGIN_EVENT:-}"
json="${HERDR_PLUGIN_EVENT_JSON:-}"
tab=$(printf '\t')
cr=$(printf '\r')

log() { echo "[worktree-hooks] $*" >&2; }
key() { printf %s "$1" | cksum | cut -d' ' -f1; }
real() { (cd "$1" 2>/dev/null && pwd -P) || printf '%s\n' "$1"; }

# field JSON KEY: the first value of KEY anywhere in JSON, a string without its quotes; empty when absent. herdr's
# events nest the worktree differently (workspace.worktree.checkout_path, worktree.path), so the first one found wins.
if jq -n 1 >/dev/null 2>&1; then
  field() {
    printf %s "$1" | jq -r --arg k "$2" '[.. | objects | select(has($k)) | .[$k]][0] | if . == null then empty else tostring end' 2>/dev/null
  }
else
  # ponytail: breaks on a value holding `"` or a JSON escape; jq, preferred above, has no such limit
  field() {
    printf %s "$1" | grep -oE "\"$2\" *: *(\"[^\"]*\"|[^,}\" ]*)" | head -n 1 | sed 's/^[^:]*: *//; s/^"//; s/"$//'
  }
fi

# Every `key = value` of hooks.conf as section<TAB>key<TAB>value; comments, blank lines and keys outside a section
# are skipped.
entries() {
  [ -f "$conf" ] || return 0
  s=
  while IFS= read -r line || [ -n "$line" ]; do
    line=${line%"$cr"}
    case $line in
      '#'* | '') continue ;;
      '['*']') s=${line#?}; s=${s%?}; continue ;;
      *=*) ;;
      *) continue ;;
    esac
    [ -n "$s" ] || continue
    k=${line%%=*}; k=${k%"${k##*[! ]}"}; k=${k#"${k%%[! ]*}"}
    v=${line#*=}; v=${v#"${v%%[! ]*}"}
    printf '%s\t%s\t%s\n' "$s" "$k" "$v"
  done < "$conf"
}

# The projects as name<TAB>dir, `~` expanded.
sections() {
  entries | while IFS="$tab" read -r s k v; do
    [ "$k" = dir ] || continue
    case $v in '~') v=$HOME ;; '~/'*) v=$HOME/${v#??} ;; esac
    printf '%s\t%s\n' "$s" "${v%/}"
  done
}

project_dir() {
  sections | while IFS="$tab" read -r n d; do [ "$n" = "$1" ] && { printf '%s\n' "$d"; break; }; done
}

# commands NAME KIND: project NAME's `created` or `removed` commands, one per line.
commands() {
  entries | while IFS="$tab" read -r s k v; do [ "$s" = "$1" ] && [ "$k" = "$2" ] && printf '%s\n' "$v"; done
}

# match REPO_ROOT REPO_NAME: the project whose dir is the repo's root, else whose dir's folder has the repo's name.
match() {
  list=$(sections)
  [ -n "$list" ] || return 1
  if [ -n "$1" ]; then
    r=$(real "$1")
    while IFS="$tab" read -r n d; do
      [ -n "$n" ] && [ "$(real "$d")" = "$r" ] && { printf '%s\n' "$n"; return 0; }
    done <<EOF
$list
EOF
  fi
  [ -n "$2" ] || return 1
  while IFS="$tab" read -r n d; do
    [ -n "$n" ] && [ "${d##*/}" = "$2" ] && { printf '%s\n' "$n"; return 0; }
  done <<EOF
$list
EOF
  return 1
}

# herdr's server runs with launchd's bare PATH; hook commands want the user's (Homebrew, nvm, pnpm). An interactive
# login shell may print other things first, hence the marker.
login_path() {
  p=$("${SHELL:-/bin/sh}" -lic 'printf "\n__SHEPHERD_PATH__=%s\n" "$PATH"' </dev/null 2>/dev/null | sed -n 's/^__SHEPHERD_PATH__=//p' | tail -n 1)
  if [ -n "$p" ]; then PATH=$p; else PATH="$PATH:/opt/homebrew/bin:/usr/local/bin:$HOME/.local/bin"; fi
  export PATH
}

# run NAME KIND CWD WORKTREE: the project's commands of KIND in CWD, stopping at the first failure.
run() {
  cmds=$(commands "$1" "$2")
  [ -n "$cmds" ] || return 0
  main=$(project_dir "$1")
  login_path
  PROJECT=$1 MAIN=$main WORKTREE=$4 EVENT=$2
  export PROJECT MAIN WORKTREE EVENT
  while IFS= read -r cmd; do
    [ -n "$cmd" ] || continue
    log "\$ $cmd"
    (cd "$3" && sh -c "$cmd") </dev/null || { log "exited $?: $cmd"; return 1; }
  done <<EOF
$cmds
EOF
}

created() {
  [ -n "$(sections)" ] || exit 0
  ws=$(field "$json" workspace_id)
  decided=
  case $event in
    *focus*)
      # workspace.focused fires on every focus: once a workspace is settled, skip it without asking herdr. Ids are
      # short counters shared by every session, so the session is part of the key.
      [ -n "$ws" ] || exit 0
      decided="$state/decided/$(key "${HERDR_SOCKET_PATH:-}|$ws")"
      [ -e "$decided" ] && exit 0
      ;;
  esac
  checkout=$(field "$json" checkout_path)
  [ -n "$checkout" ] || checkout=$(field "$json" path)
  root=$(field "$json" repo_root)
  name=$(field "$json" repo_name)
  linked=$(field "$json" is_linked_worktree)
  fresh=true
  if [ -z "$checkout" ] || { [ -z "$root" ] && [ -z "$name" ]; }; then
    # the focus event carries only an id; a brand-new worktree's workspace has 1 tab with 1 pane
    [ -n "$ws" ] || exit 0
    info=$("$herdr" workspace get "$ws" 2>/dev/null)
    checkout=$(field "$info" checkout_path)
    root=$(field "$info" repo_root)
    name=$(field "$info" repo_name)
    linked=$(field "$info" is_linked_worktree)
    [ "$(field "$info" tab_count)" = 1 ] && [ "$(field "$info" pane_count)" = 1 ] || fresh=false
  fi

  if [ -n "$checkout" ] && [ "$linked" != false ] && [ "$fresh" = true ]; then
    if project=$(match "$root" "$name"); then
      mkdir -p "$state/applied" "$state/map"
      k=$(key "$checkout")
      claim="$state/applied/$k"
      # a .git newer than the claim: removed outside herdr and made again at the same path
      [ -d "$claim" ] && [ "$checkout/.git" -nt "$claim" ] && rmdir "$claim"
      if mkdir "$claim" 2>/dev/null; then
        printf '%s\n' "$project" > "$state/map/$k"
        log "created [$project] -> $checkout"
        run "$project" created "$checkout" "$checkout" || rmdir "$claim" # failed: the next event retries
      fi
    else
      log "no project matches repo ${root:-$name}"
    fi
  fi
  [ -n "$decided" ] && mkdir -p "$state/decided" && : > "$decided"
  exit 0
}

removed() {
  checkout=$(field "$json" checkout_path)
  [ -n "$checkout" ] || checkout=$(field "$json" path)
  [ -n "$checkout" ] || { log "removed: no worktree path in the event"; exit 0; }
  k=$(key "$checkout")
  project=$(cat "$state/map/$k" 2>/dev/null)
  rm -rf "$state/applied/$k" "$state/map/$k"
  # the worktree and its git data are gone: the project comes from the map made when it was created
  if [ -z "$project" ] || [ -z "$(project_dir "$project")" ]; then
    project=$(match "$(field "$json" repo_root)" "$(field "$json" repo_name)") || { log "removed: no project for $checkout"; exit 0; }
  fi
  main=$(project_dir "$project")
  log "removed [$project] <- $checkout"
  run "$project" removed "$main" "$checkout"
  exit 0
}

case $event in
  *remove*) removed ;;
  *) created ;;
esac
