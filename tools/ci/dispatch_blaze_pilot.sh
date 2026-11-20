#!/usr/bin/env bash
set -euo pipefail
metrics="$RUNNER_TEMP/metrics"
mkdir -p "$metrics"
test "$(git rev-parse HEAD)" = "$SOURCE_SHA"

if [[ $1 == setup ]]; then
  test ! -e target
  sudo rm -rf /usr/share/dotnet /usr/local/lib/android /opt/ghc /opt/hostedtoolcache/CodeQL
  sudo fallocate -l 16G /mnt/dispatch-blaze.swap
  sudo chmod 600 /mnt/dispatch-blaze.swap
  sudo mkswap /mnt/dispatch-blaze.swap
  sudo swapon /mnt/dispatch-blaze.swap
  mkdir -p "$HOME/.cargo" "$RUNNER_TEMP/pilot-bin"
  printf '[profile.dev.package.oteryn-gameplay-dispatch]\ndebug = 0\n' >> "$HOME/.cargo/config.toml"
  rustup toolchain install 1.100.0 --profile minimal --no-self-update
  rustc +1.100.0 -vV > "$metrics/rustc.txt"
  cargo +1.100.0 fetch --locked
  fetch() {
    curl --proto '=https' --tlsv1.2 -fsSL --retry 5 --retry-all-errors --connect-timeout 15 -o "$2" "$1"
    echo "$3  $2" | sha256sum -c -
  }
  fetch https://github.com/rui314/mold/releases/download/v2.42.1/mold-2.42.1-x86_64-linux.tar.gz \
    "$RUNNER_TEMP/mold.tgz" 6ff270c9bf07d2bec5c98aa324eb7c4daf6a1a4d815c05ff1708049616047855
  tar -xzf "$RUNNER_TEMP/mold.tgz" -C "$RUNNER_TEMP"
  echo "MOLD_PATH=$RUNNER_TEMP/mold-2.42.1-x86_64-linux/bin/mold" >> "$GITHUB_ENV"
  echo "MOLD_PRELOAD=$RUNNER_TEMP/mold-2.42.1-x86_64-linux/lib/mold/mold-wrapper.so" >> "$GITHUB_ENV"
  if [[ $OPERATION == archive ]]; then
    fetch https://github.com/nextest-rs/nextest/releases/download/cargo-nextest-0.9.146/cargo-nextest-0.9.146-x86_64-unknown-linux-gnu.tar.gz \
      "$RUNNER_TEMP/nextest.tgz" 682c21b777c333e96fd532e114d3a5a894e0729ab88d94c0a9f20f8419695428
    tar -xzf "$RUNNER_TEMP/nextest.tgz" -C "$RUNNER_TEMP/pilot-bin" cargo-nextest
  fi
  echo "$RUNNER_TEMP/pilot-bin" >> "$GITHUB_PATH"
  if [[ $ENGINE != cargo ]]; then
    version=v0.20261027.0
    asset="blaze-$version-x86_64-unknown-linux-gnu"
    fetch "https://github.com/indexable-inc/blaze/releases/download/$version/$asset.tar.gz" \
      "$RUNNER_TEMP/blaze.tgz" df61ce6d8216087a40e986d4f84cf2a230638bde014e38519e08b50282e90e5d
    tar -xzf "$RUNNER_TEMP/blaze.tgz" -C "$RUNNER_TEMP"
    cd "$RUNNER_TEMP/$asset"
    for exe in ./*; do
      if [[ -f $exe && -x $exe ]]; then install -m 0755 "$exe" "$RUNNER_TEMP/pilot-bin/"; fi
    done
    cd "$GITHUB_WORKSPACE/source"
    export PATH="$RUNNER_TEMP/pilot-bin:$PATH"
    # The binary verifies its own pinned rust toolchain; prefetch outside measured phases.
    blaze toolchain | tee "$metrics/blaze-toolchain.txt"
    if [[ $ENGINE == enabled ]]; then
      blaze enable 2>&1 | tee "$metrics/enable.txt"
      test -n "$(grep -F rustc-workspace-wrapper "$HOME/.cargo/config.toml")"
    fi
  fi
  exit
fi

test "$1" = measure
export LD_PRELOAD="$MOLD_PRELOAD"
export RUSTUP_TOOLCHAIN=1.100.0
# Each phase keeps diagnostics in the Actions log and a small ordered result file,
# including if the compiler rejects the candidate.
measure() {
  local phase=$1
  shift
  local rc=0
  /usr/bin/time -f 'wall_seconds=%e\nmax_process_rss_kib=%M' -o "$metrics/$phase.time" \
    timeout --signal=TERM --kill-after=10s 20m "$@" > "$RUNNER_TEMP/$phase.log" 2>&1 || rc=$?
  printf '%s\n' "$rc" > "$metrics/$phase.exit"
  tail -n 28 "$RUNNER_TEMP/$phase.log" | cut -c1-1600 | tee "$metrics/$phase.tail"
  cat "$metrics/$phase.time"
  return "$rc"
}
if [[ $OPERATION == edit ]]; then
  example=crates/gameplay/dispatch/examples/dispatch_blaze_probe.rs
  test ! -e "$example"
  mkdir -p "$(dirname "$example")"
  cat > "$example" <<'RS'
use oteryn_gameplay_dispatch::observability::record_channel_work_item;
use std::time::Duration;
fn main() {
    let lines = oteryn_error_codes::capture(|| {
        record_channel_work_item("cast", Duration::from_millis(51), Duration::from_millis(50));
    });
    let events: Vec<_> = lines.iter().flat_map(|line| line.split_whitespace())
        .filter_map(|field| field.strip_prefix("event=")).collect();
    assert_eq!(events.len(), 1);
    println!("{}", events[0]);
}
RS
  compiler=(cargo)
  if [[ $ENGINE == blaze ]]; then compiler=(blaze); fi
  build=("${compiler[@]}" build --locked -p oteryn-gameplay-dispatch --example dispatch_blaze_probe)
  exe=target/debug/examples/dispatch_blaze_probe
  verify() {
    local observed
    observed=$("$exe")
    printf '%s\n' "$observed" > "$metrics/$1.observed"
    test "$observed" = "$2"
    sha256sum "$exe" | cut -d' ' -f1 > "$metrics/$1.binary.sha256"
  }
  measure cold "${build[@]}"
  verify cold work_item_overrun
  measure warm "${build[@]}"
  verify warm work_item_overrun
  src=crates/gameplay/dispatch/src/observability.rs
  test "$(grep -Fc '"work_item_overrun"' "$src")" -eq 1
  sed -i 's/"work_item_overrun"/"work_item_overrun_probe"/' "$src"
  git diff -- "$src" > "$metrics/body-edit.diff"
  measure edit "${build[@]}"
  verify edit work_item_overrun_probe
  if [[ $ENGINE != cargo ]]; then
    blaze status > "$metrics/pre-restart-status.txt" 2>&1 || true
    timeout 20 blaze stop
    measure restart "${build[@]}"
    verify restart work_item_overrun_probe
  fi
else
  lib="$RUNNER_TEMP/lib3"
  mkdir -p "$lib"
  if [[ $ENGINE == blaze ]]; then
    # cargo-nextest calls CARGO for metadata and test --no-run. Stable Cargo
    # supplies metadata only. The build command must be serviced by Blaze.
    cat > "$RUNNER_TEMP/nextest-cargo" <<'WRAP'
#!/usr/bin/env bash
set -eu
printf '%s\n' "$*" >> "$RUNNER_TEMP/metrics/nextest-invocations.txt"
args=("$@")
for ((i=0; i<${#args[@]}; i++)); do
  case "${args[i]}" in
    metadata|locate-project|-V|--version) exec cargo "${args[@]}" ;;
    test|build|rustc|check) exec blaze "${args[@]:i}" ;;
  esac
done
echo "Unhandled nextest/Cargo command" >&2
exit 2
WRAP
    chmod +x "$RUNNER_TEMP/nextest-cargo"
    export CARGO="$RUNNER_TEMP/nextest-cargo"
  fi
  measure archive cargo-nextest nextest archive --locked -p oteryn-gameplay-dispatch --lib --archive-file "$lib/pg-lib-3.tar.zst"
  cargo-nextest nextest list --archive-file "$lib/pg-lib-3.tar.zst" --workspace-remap "$PWD" --message-format json > "$lib/pg-lib-3.json"
  python3 tools/ci/plan_pg_partitions.py "$lib" 0 1 tools/ci/pg_test_durations.json --optional-archives
  sha256sum "$lib/pg-lib-3.filters" | cut -d' ' -f1 | tee "$metrics/inventory.sha256"
  wc -l < "$lib/pg-lib-3.filters" | tee "$metrics/inventory.count"
  test "$(cat "$metrics/inventory.sha256")" = 1863be3bc98bc6fbdb0a9f9b27ae47cc1503156e95c03dbdf399ca679976ad9b
  test "$(cat "$metrics/inventory.count")" -eq 784
fi
