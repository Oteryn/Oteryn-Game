#!/usr/bin/env bash
# Build the reusable node-boot artifacts from a clean source checkout.
set -Eeuo pipefail
[[ $# == 1 ]] || { echo 'usage: prebuild.sh <out_dir>' >&2; exit 2; }
GAME_SOURCE="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
cd "$GAME_SOURCE"
[[ -z "$(git status --porcelain)" ]] || { echo 'prebuild requires a clean tree' >&2; exit 2; }
SOURCE_SHA="$(git rev-parse HEAD)"
# Read the single literal declaration without executing the harness.
eval "$(sed -n '/^readonly NODE_BOOT_WORLD_REGIONS=/p' tools/qualification/node_boot/run.sh)"
OUT="$(realpath -m -- "$1")"
mkdir -p "$OUT/world"
rm -f "$OUT/manifest.json"
for variant in default qualification-clock; do
  features=()
  [[ "$variant" == default ]] || features=(--features qualification-clock)
  mkdir -p "$OUT/$variant"
  cargo +1.100.0 build --locked -p oteryn-game-server --bins "${features[@]}"
  cargo +1.100.0 test --locked -p oteryn-game-server --lib --no-run "${features[@]}" --message-format=json > "$OUT/$variant/artifacts.jsonl"
  python3 - "$OUT/$variant" <<'PY_ARTIFACT'
import json, pathlib, shutil, sys
out = pathlib.Path(sys.argv[1])
artifacts = [json.loads(line) for line in (out / 'artifacts.jsonl').read_text().splitlines()]
tests = [a['executable'] for a in artifacts if a.get('reason') == 'compiler-artifact'
         and a.get('target', {}).get('name') == 'oteryn_game_server'
         and 'lib' in a['target']['kind'] and a.get('profile', {}).get('test') and a.get('executable')]
if len(tests) != 1:
    raise SystemExit(f'expected one game-server lib test artifact, got {len(tests)}')
shutil.copy2(tests[0], out / 'oteryn-game-server-lib-test')
for name in ('oteryn-game-server', 'oteryn-game-ops', 'oteryn-game-migrate'):
    # The lib-only test invocation does not emit bin artifacts.
    shutil.copy2(pathlib.Path(tests[0]).parent.parent / name, out / name)
(out / 'artifacts.jsonl').unlink()
PY_ARTIFACT
done
cargo +1.100.0 run --locked --release -p oteryn-world-bundle-compiler -- compile-regions "$GAME_SOURCE" "$GAME_SOURCE/content/world/pins/oteryn.identity.json" "$OUT/world/world.otwb" "$NODE_BOOT_WORLD_REGIONS" > "$OUT/world/world-compile.json"
python3 - "$OUT/manifest.json" "$SOURCE_SHA" "$NODE_BOOT_WORLD_REGIONS" <<'PY_MANIFEST'
import json, sys
with open(sys.argv[1], 'w') as out:
    json.dump(dict(source_sha=sys.argv[2], regions=sys.argv[3],
                   variants=['default', 'qualification-clock'], toolchain='1.100.0'), out, indent=2)
    out.write('\n')
PY_MANIFEST
