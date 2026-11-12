# Party effects v1

Rebased onto `c97858c3f440fd26676d3bc9be6d6a35b2c711ff`. Existing native
routes and capabilities 286/287 are retained; the five new shortcuts use
Ctrl+Alt+Shift plus C/R/B/K/P respectively to avoid the base bindings.

GAP-SV7 S13c: capability 280, command 496, with no capability dependencies.
This is an own-actor observation, offered in bundle Worlds. No migration or
new gameplay rule is introduced; the existing party and condition owners remain authoritative.

Current-main gap: `gameplay_transport/party_command.rs:51` copies only the shared-XP
preference into `protocol-oteryn/src/party.rs:46`; the native party panel consumes
that preference at `client-ui/src/features/0042-party/mod.rs:139-145`. Eligibility
and numerical/duration party effects are absent there.

## Request

Empty. Invalid bytes, unselected capabilities, unavailable party authority, or a
stale actor/session/connection/Character lease return a rejected empty result.
There is no target Character parameter. The normal session command ingress owns
same-command replay; a new query observes current state.

## Reply

Exactly 41 bytes. First byte flags: bit 0 party membership, bit 1 shared-XP
preference enabled, bit 2 shared XP active, bit 3 own member eligible. Remaining
bits are zero. An active state requires membership, enabled, and eligible;
a nonmember has no shared-XP flags. The eligibility bit is meaningful only while
sharing is enabled; the client labels disabled eligibility as not evaluated.
Then ten little-endian u32 values:

- Four numerical percentages: damage reduction, auto attack, spell damage,
  spell healing; then additive Guiding Presence Mantra points (not a percentage).
  Existing `spell/cast/state.rs:610-628` multiplies these points by Ascetic stage.
- Five remaining durations in milliseconds: Train, Protect, Enchant, Heal,
  Enlighten Party. Zero means inactive. A still-active finite condition has at
  least one millisecond in this projection, including its final partial millisecond.
  The client rounds display seconds up.

The decoder rejects truncation, trailing bytes and impossible flags. Client
values change only after an accepted, decoded reply; refusals retain confirmed
values with an error, and logout clears the panel.

## Parity and accepted differences

Donor evidence class is OTS_HYPOTHESIS_ONLY, pinned Crystal
`201189f871d23597b82caf88fd611454a2ddefb8` and Canary
`47dfd51f45280a59a1d3e50ba7edd573d7234446` (repository audit revisions).
Crystal `src/creatures/players/player.cpp:7694-7730` distinguishes enabled,
active and individually eligible shared XP using party shields. This projection
reuses the existing strong `party_skull::observe_party_skull` owner. That owner
accepts selected capability 280 internally without requiring client support for
unrelated party-skull rendering (134); command 155 remains gated by 134. This
allows native clients to negotiate the new panel alone. The projection never
turns the preference into eligibility or authorizes XP settlement from a cache.

Crystal `data/scripts/spells/party/protect_party.lua:6-10,26-38` defines a timed
party buff. In the same directory, `train_party.lua:6-9`,
`enchant_party.lua:6-9`, `heal_party.lua:6-9`, and `enlighten_party.lua:5-8`
define their own timed conditions. Oteryn reads the already-installed source-qualified conditions
through exact effect identities from `spell/actor_conditions.rs:407-425`; it does
not reapply spell duration, recipient range, or numerical rules in the UI.
Canary `src/creatures/players/grouping/party.cpp:308-350` shares Guiding Presence
Mantra, independently of shared XP. Existing accepted Oteryn party rules and
`party_spell_owner.rs:510-514` govern the numerical buff owner, including its
one-second observation lifetime (`spell/cast/state.rs:636-642`).
Inherited accepted-profile difference: current Oteryn's captured Enlighten Party
profile and existing `postgres_tests/party_buffs.rs:478-487` use 120 seconds,
where pinned Crystal `enlighten_party.lua:8` has 300 seconds. This package reads
the already-installed owner's duration and does not change that gameplay rule;
the captured profile provenance is in
`docs/reference/spells/r21-local-candidate/bundles/instant-enlighten_party/manifest.json`.
Both donor pins are revisions on their repositories' `main` branch.

Accepted wire difference for this package: the native Oteryn panel presents this
server-owned state together in a bounded query reply. The server's existing
Channel-local eligibility policy remains unchanged; this display is never an XP
grant or a replacement for the settlement owner's proof.
