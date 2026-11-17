# Definition-ready reward claims

The shared reward-claim route is already implemented on main. Reuse
`content::world_reward_claims::bind_embedded_reward_claims` and the ordinary
`ComposedFreshAdmission::use_bundle_chest` dispatch path. Do not add another
quest-specific claim resolver or accept a client-supplied claim identity.

The source placement supplies position, Crystalserver unique ID and appearance.
The binder requires every placement of a shared claim to bind. Item admission
uses the active native generation. The existing DUR03 owner commits the complete
reward and once-per-character claim together; command replay reuses its receipt.

The definition-ready subset of the completion matrix selects coverage in
`definition_ready_claims.rs`: 42 quests, 59 distinct claims and 60 placements.
The matrix remains a derived work queue and is never loaded by production
dispatch. Coverage uses embedded claim definitions, source-coordinate fixture
geometry and actual native item facts. It does not prove those cells are present
in every deployed World bundle, or qualify other lanes' complete quest stories.

The PostgreSQL case uses the canonical Adorned UH Rune claim through literal
USE, with source UID6025 and appearance2469 at (33136,31601,-15). Its reward is
Item11603, the decorative silver rune emblem. The case verifies stale-session
refusal without a MINT receipt, exact reward quantity, refusal of another live
USE, reconnect command replay, and exactly one durable claim/MINT receipt.
Server-version and elapsed-time markers distinguish actual PostgreSQL execution
from the repository's route-less early return.

This is qualification of reused engine behavior, so no artificial failing
baseline or duplicate production implementation is introduced. W15b's broader
cooldown, random, timed and written reward variants remain separate work.

Main c77d436d8 supplies the eleven missing native item admissions through
QREADY1+2. This lane reuses those owners and retains its existing profiles; it
does not create a duplicate item producer. The premerge coverage failure for
Blood Herb is retained as evidence of the original admission gap.
