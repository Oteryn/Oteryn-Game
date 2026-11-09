# M35: independent once claims compose one completion

The selected source-backed allocations already exist in the committed RewardClaim shards.
The quest references and exact claim membership were checked in
`content/quests/definitions/quests-*.json` and
`content/interactions/reward_claims/reward-claims-*.json`.

| Quest | Required independent once claims |
| --- | --- |
| Circle Room | `kv.circleroom1`, `kv.circleroom2` |
| Crystal Wand | `kv.crystalwand1`, `kv.crystalwand2` |
| Demona Ring | `kv.demonaring1`, `kv.demonaring2` |
| Double Hero | `kv.doublehero1`, `kv.doublehero2` |
| Griffin Shield | `kv.griffinshield1`, `kv.griffinshield2`, `kv.griffinshield3` |

Every member is required. Existing source rewards and once identities remain unchanged.
`claim-sets.json` selects `mint_only` for these groups. The canonical map binder selects
its member operation; the MINT transaction persists the obligation, and the Quest writer
requires that obligation's matching immutable MINT receipt before changing its protected
member track. The existing completion-set evaluator observes every group under the
Character root lock. The final member records completion exactly once in the same Quest
transaction. Admission recovers pending obligations after a restart.

Legacy storage-based Bear Room groups keep their previous encoding and caller policy.
A `mint_only` operation cannot be executed through a generic Command or by removing or
renaming its binding. These adapted completion rules do not certify other travel,
combat, level, or access mechanics.
