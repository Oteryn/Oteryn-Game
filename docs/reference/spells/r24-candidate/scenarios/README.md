# Reusable player spell qualification plans

`player-spell-scenarios.json` covers every entry of the exact 246-definition catalog,
including five inactive source aliases. Its 1-based indices follow the Rust catalog's
canonical identity order. Verify the loaded book and pinned catalog before live use;
these indices are not upstream spell IDs.

There are 241 selected definitions: 101 have an executable path requiring suitable
fixtures, and 140 require missing owner/connection integration for a positive run.
These are planning categories, not successful gameplay counts. Existing Premium,
parameter transport, house and known missing native dispatcher prerequisites remain
explicit. A generator cannot qualify those prerequisites or issue activation authority.

Each entry preserves requirements, mana/soul, cooldown groups, target rules, rune
metadata, dependency references and source identities. Positive runs must use an
actual eligible actor, actual items/targets and current owner facts. Negative cases
must isolate the indicated failing precondition with earlier gates satisfied. A
refusal must leave costs, cooldowns and world state unchanged; an observed `Cast`
requires checking its real target/world effect. An empty damage footprint is not
damage evidence. An inactive alias has only a refusal case.

Regenerate to a new output file (exclusive creation):

```sh
python tools/qualification/spells/build_scenarios.py \
  docs/reference/spells/r21-local-candidate/active-artifact/catalog.json \
  docs/reference/spells/r21-local-candidate/active-artifact/source-selection.json \
  /tmp/player-spell-scenarios.json
python -m unittest discover -s tools/qualification/spells -p test_qualification_tools.py
```

Use the existing `tools/qualification/spells/run.sh` and Thalom/S3-B environment;
this adds no second server or topology. The existing dev client supports no-parameter
`none`, `attack` and `position` wire requests. Listed transport modes are candidates
to test against each spell's header, not permission to cast with any target. Text or
player-name parameters require the separately negotiated consumer/client route.

The input retains dated Canary/Crystal and wiki source evidence. This lane does not
claim fresh wiki retrieval, all spells passing, long-lived deployment, complete
monster spell execution or production activation.
