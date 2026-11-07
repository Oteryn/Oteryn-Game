#!/bin/sh
# Register the oteryn-regen and oteryn-additive merge drivers in this clone's local git config (run once per clone).
set -eu
cd "$(git rev-parse --show-toplevel)"
git config merge.oteryn-regen.name "Oteryn derived content: take incoming, regenerate afterwards"
git config merge.oteryn-regen.driver "python3 tools/merge-driver/oteryn_regen_driver.py %O %A %B %P"
git config merge.oteryn-additive.name "Oteryn hand-written sources: combine insert-only conflicts"
git config merge.oteryn-additive.driver "python3 tools/merge-driver/oteryn_additive_driver.py %O %A %B %L %P"
echo "oteryn-regen and oteryn-additive merge drivers registered. After a merge run cargo fmt, then tools/merge-driver/regen.sh."
