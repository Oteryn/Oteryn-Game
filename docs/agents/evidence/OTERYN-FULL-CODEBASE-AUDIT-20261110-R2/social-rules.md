# social-rules — REVIEWED

Base: `4f0b6c62a789489fca6c37dd36d5917d096f328e`. All 22 tracked package files, including manifests and tests, substantively reviewed. Source was not changed.

## Entrypoints and authority

This package supplies deterministic membership, guild, shared-XP, chat, Premium wire arithmetic, staff grammar and future trust-policy predicates. SQL transactions, signed evidence verification, current roles, runtime scope and lease generation remain the consumers' responsibilities. Reviewed actual dispatch and durability calls to distinguish those obligations from missing guards.

## Invariants, failures and retries

- Party membership snapshots reject duplicate identities and inconsistent solo/party observations. Their type deliberately contains no spatial or reward authority. Shared XP validates at most 50 unique principals, valid levels/nonnegative pools and exact caller-supplied scope. Eligibility requires the leader, enabled setting, no leader PZ block, level ratio, 30-tile/one-floor proximity and committed activity within 120 seconds. Separate quotient/remainder sums avoid overflowing an aggregated damage product; final equal shares use ceiling after rational bonus and summon deduction.
- The actual XP planner reads committed party data and current colocated actor bindings, derives full tagged RuntimeScopeRefV1 rather than raw Channel UUID, checks death-clock agreement, current lease generation and missing-local-member completeness. Committed heal activity requires an applied positive HP delta on another party member, correct caster/command/content/lease and original event time. Replay cannot renew activity by observation time alone.
- Guild rank changes require both old and desired rank strictly below the actor; only leader/vice exclusion is allowed. Forming/active are the ordinary command states. Exact deadline is accepted; curing an overdue formation/vice deficit is refused, using i128 differences. Actual rank writer uses transaction time, expected revision, current target account/world/lifecycle and leadership rows. Premium deficits remain separately job-owned by the accepted contract.
- Chat shares a four-line bucket across local/relay traffic, refilling one line per 2.5 seconds. Local pre-gate does not charge a successful full-path attempt twice. Under-level repeats consume lines without repeatedly taking the runtime lock. Mute state dominates cooldown and flood checks; offences/recipients are bounded. Advertising is checked again under the same mutex when claimed. Durable mute persistence is outside this pure package and was not declared fully audited here.
- Public chat role facts are distinct group/account/access values. Actual dispatch requires current scope/generation/control, then obtains role/guild facts through the durable access reader and fetches current Premium only where needed. Help intentionally has separate three-minute speech and one-hour repeat-mute deadlines. Egress policy drops room before local before private at a 64-line bound; queue implementation is outside this package.
- Premium UTC parsing is closed ASCII shape with valid Gregorian dates, years 1970..9999 and at most six fractional digits. The actual snapshot reader checks positive lease, refresh interval and a fixed one-hour maximum before product-specific authority timing. Active/not-yet-effective cutoff and whole-second refresh are recomputed, not trusted as independent input.
- Staff new-request authorization invokes valid_tools; historical records invoke valid only to retain old signatures. The executor binds world/channel to the current fence, validates typed IDs, checks live target world/lifecycle, performs role/issuer/self-target/protected-staff gates and samples an ordered bounded clock interval after awaited prerequisites. This audit did not claim the entire signature/log implementation was reviewed.
- Release/trust predicates are intentionally unwired numeric/key-membership policy, not cryptographic validation. They reject wrong roles, root reuse, revoked keys, skipped/reversed revisions, expired/regressed checkpoints and invalid clocks. Search found only tests for their public predicate names, so unsigned-predicate acceptance is not evidence of a live trust bypass.

## Hypotheses resolved

1. Low-XP multi-vocation bonus is not a confirmed defect: the exact PARTYPVP0-RL-07 registry specifically applies the >=20 threshold to one normalized vocation and independently defines two/three/four+ bonuses. The authored JSON is not wired; generic wording there is not stronger authority than the registered accepted row.
2. Shared-XP scope confusion is refuted at this consumer by full tagged world/scope input and current character/session/lease checks; equal UUID bytes do not collapse Channel/Instance or different worlds.
3. Premium timing helper arithmetic is unchecked generically, but the actual supported-wire caller bounds all timestamps through the four-digit-year parser and passes a one-hour constant; i64 endpoint overflow is not reachable through that path.
4. GM mute via historical valid() is refuted for new requests by the executor's unconditional extended::prepare -> valid_tools gate. Request::fresh alone does not verify lower<=upper; the actual executor does.

## Findings

No new confirmed finding in this package. Previously confirmed GR-SOUL-001 remains separately documented; this package does not replenish or settle soul resources.

## Evidence and remaining scope

All package tests were analyzed as code, not run. No SQL, native server, chat relay or staff integration execution is claimed. Dependency ranges are recorded separately in ledger.json and do not mark those larger packages complete. No package files remain. Source SHA256 and actual line lengths are recorded and ranges validated against the immutable checkout.
