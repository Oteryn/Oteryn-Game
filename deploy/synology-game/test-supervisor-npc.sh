#!/usr/bin/env bash
# Tests supervisor.sh start against a recording stand-in for oteryn-game-server: the NPC catalogue flags
# (--npc-data-project/--npc-data-sha256) are passed exactly when the deploy staged BASE/run/npc-data, and a
# damaged staging directory or pin refuses the start instead of booting without NPCs.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' EXIT
export BASE="$tmp/base" OTERYN_NATIVE_GAMEPLAY_MANIFEST=/nonexistent
mkdir -p "$BASE/bin" "$BASE/node" "$BASE/run" "$BASE/log"
cat > "$BASE/bin/oteryn-game-server" <<'S'
#!/usr/bin/env bash
printf '%s' "${OTERYN_STEP_WAIT_RECEIPTS-unset}" > "$BASE/run/env"
printf '%s\n' "$@" > "$BASE/run/args.tmp" && mv "$BASE/run/args.tmp" "$BASE/run/args"
S
chmod 755 "$BASE/bin/oteryn-game-server"
digest=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
npc="$BASE/run/npc-data"
fail() { echo "FAIL $1" >&2; exit 1; }
launch() { # launch <name> <expected-rc> [expected args, one per word]: start, wait for the server's argv
  local name="$1" want="$2" rc=0 i; shift 2
  rm -f "$BASE/run/args" "$BASE/run/node.pid"
  "$here/supervisor.sh" start 2>"$tmp/err" || rc=$?
  [[ "$rc" = "$want" ]] || fail "$name: rc=$rc want=$want ($(cat "$tmp/err"))"
  if [[ "$want" = 0 ]]; then
    for i in $(seq 1 50); do [[ -f "$BASE/run/args" ]] && break; sleep 0.1; done
    [[ "$(tr '\n' ' ' < "$BASE/run/args")" = "$* " ]] || fail "$name: argv=$(tr '\n' ' ' < "$BASE/run/args")"
  else
    [[ ! -e "$BASE/run/args" && ! -e "$BASE/run/node.pid" ]] || fail "$name: a server was started"
  fi
}
base_args=(serve --config "$BASE/node/node.toml")
stage() { rm -rf "$npc"; mkdir -p "$npc/content/world"; printf '%s\n' "$digest" > "$npc/PROJECT_SHA256"; }

launch "no staging: no flags" 0 "${base_args[@]}"
stage
launch "staged: flags with the pinned digest" 0 "${base_args[@]}" --npc-data-project "$npc/content/world" --npc-data-sha256 "$digest"
printf '%s' "$digest" > "$npc/PROJECT_SHA256"
launch "pin without trailing newline" 0 "${base_args[@]}" --npc-data-project "$npc/content/world" --npc-data-sha256 "$digest"
printf '%s\n' "${digest^^}" > "$npc/PROJECT_SHA256"; launch "uppercase pin refused" 1
printf '%s\n' "${digest:0:63}" > "$npc/PROJECT_SHA256"; launch "short pin refused" 1
printf '%s\n%s\n' "$digest" "$digest" > "$npc/PROJECT_SHA256"; launch "two-line pin refused" 1
printf '\n' > "$npc/PROJECT_SHA256"; launch "empty pin refused" 1
rm "$npc/PROJECT_SHA256"; launch "missing pin refused" 1
stage; rmdir "$npc/content/world"; launch "missing project root refused" 1
stage; rm -rf "$npc/content/world"; ln -s "$tmp" "$npc/content/world"; launch "symlinked project root refused" 1
stage; mv "$npc/PROJECT_SHA256" "$tmp/pin"; ln -s "$tmp/pin" "$npc/PROJECT_SHA256"; launch "symlinked pin refused" 1
rm -rf "$npc"; mkdir "$tmp/elsewhere"; ln -s "$tmp/elsewhere" "$npc"; launch "symlinked npc-data refused" 1
rm -f "$npc"; : > "$npc"; launch "npc-data file refused" 1
rm -f "$npc"; launch "removed staging: no flags again" 0 "${base_args[@]}"
receipts="$BASE/run/step-wait-receipts"
launch "no marker" 0 "${base_args[@]}"; [[ "$(cat "$BASE/run/env")" = unset ]] || fail "receipts env set without marker"
printf '1\n' > "$receipts"; launch "marker 1" 0 "${base_args[@]}"; [[ "$(cat "$BASE/run/env")" = 1 ]] || fail "marker did not enable receipts"
printf '0\n' > "$receipts"; launch "marker 0" 0 "${base_args[@]}"; [[ "$(cat "$BASE/run/env")" = unset ]] || fail "marker 0 enabled receipts"
rm -f "$receipts"
echo "supervisor npc-data flags: ok"
