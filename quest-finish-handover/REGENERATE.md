# Excluded generated/minified files

All21 excluded paths and exact stopped-tree output hashes are in generated-sha256.txt.
The handwritten patch excludes every .json; none of its remaining changed source files is larger than200KB.
No generated JSON is printed in the handwritten diff.

Run from repository root after applying source.diff onto f52f71f7857b399dc8f6d70ff51c23f5b779f89d.
These commands are in dependency order. Public donor caches are included in sources.zip.
Replace SOURCE_ROOT with the extracted sources directory. No tool installation is required.

```bash
python3 tools/content-migration/quest_reward_item_semantics.py --output-root .
python3 tools/content-schema/reward-claim-authoring/reward_claim_authoring.py content
python3 tools/content-schema/quest-authoring/amend_monk_source_receipt.py
python3 tools/content-schema/quest-authoring/quest_tree_authoring.py content
python3 tools/content-migration/register_spell_families.py
python3 tools/content-schema/quest-authoring/quest_callback_coverage.py --canary "$SOURCE_ROOT/quest-finish-donors/canary" --crystal "$SOURCE_ROOT/quest-finish-donors/crystal"
python3 tools/content-schema/quest-authoring/check_quest_claim_qualification.py
python3 tools/content-migration/quest_finish_newhaven_items.py --crystal "$SOURCE_ROOT/quest-newhaven-source"
```

Command1 owns the three changed canonical items-* shards.
Command2 owns reward_claims index and three changed claim shards.
Command3 owns completion242 source-fix approval/receipt, validating28immutable cores.
Command4 owns quest definition index and three changed quests-* shards.
Command5 owns native profiles, Item index, spells manifest and content manifest/lock.
Command6 owns QUEST_CALLBACK_COVERAGE.json.
Command7 owns quest-claim-qualification.json.
Command8 owns quest-finish-newhaven-items.json ONLY; its current staff51119 capsule is blocked for integration by currentWiki51166 conflict. Do NOT call its apply_admissions/admit hooks in the shared generators; they were deliberately never mounted.

KNOWN STALE SNAPSHOT EXCEPTION:
The stopped-tree content/quests/definitions/index.json hash is
162c0541e603d41397f25687bd66abdfbe46223b802c1640946ae14da5b55618.
Running current generators refreshes two pre-Medusa authoring_sources digests and produces
c16f9777cc4ee30ee96937533ed4c83536f6d7be6aeeabbf5322242b16489969.
Thus the exact frozen snapshot CANNOT honestly be described as fully reproducible from final sources with ordinary regeneration alone. This is a real known stale artifact left at STOP, not a missing command. Do not hand-edit either pin.
An idempotence check in the scratch worktree reproduced the other20/21 stopped hashes; no source-tree repair was made.
For byte-exact reconstruction of the stopped snapshot if needed, apply_json.py restores all21JSON files from f52 base versions using JSON-path replacements and verifies every input/output SHA; proved in /tmp/quest-finish-transfer-proof before the regeneration check (json-proof.log). Its existence is transfer fallback, not a claim that the stale snapshot passes authoring checks.

The next agent should keep the regenerated index and requalify the repaired tree. Any other output SHA mismatch is unexplained drift and must be investigated.
