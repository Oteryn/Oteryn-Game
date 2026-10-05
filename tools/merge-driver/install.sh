#!/bin/sh
# Register the oteryn-regen merge driver in this clone's local git config (run once per clone).
set -eu
cd "$(git rev-parse --show-toplevel)"
git config merge.oteryn-regen.name "Oteryn derived content: take incoming, regenerate afterwards"
git config merge.oteryn-regen.driver "python3 tools/merge-driver/oteryn_regen_driver.py %O %A %B %P"
echo "oteryn-regen merge driver registered. After a merge run tools/merge-driver/regen.sh."
