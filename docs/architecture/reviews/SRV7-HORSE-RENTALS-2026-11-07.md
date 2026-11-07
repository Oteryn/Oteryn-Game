# Closed S06 horse rental and horse-station source plan

Implementation and authoring inputs are mounted; central native verification and provider generation remain pending.

## Source evidence
Pinned Crystal201189f data-global/npc/appaloosa.lua and palomino.lua183-221:
- rent/horses asks500gold for one day; yes under source topic2;
- shared Timer>=os.time refuses an existing horse;
- removeMoneyBank500, uniform {22,25,26}, Timer=now+86400;
- Natural Born Cowboy, exact success phrase;
- no quest/premium/level gate on buying;
- both NPCs refuse while World RAID_STORAGE120226>now (184-187);
- successful rental tries3/10 escape (149-164), once while inactive.
Crystal mounts.xml51/54/55: clientlooks421/437/438, speed10,
premium false/true/true. Preserve acquisition-versus-wear distinction.
Crystal check_mount.lua13-26,32: every15seconds, Timer>0 && Timer<now
removes all3 rented mounts, dismounts, and sends exact Advance contract text.
Thus at the exact expiry second the source contract still exists; no fresh
rental until now>expiry. Preserve this inclusive seconds boundary.
Crystal world-npc.xml1843-1844 Appaloosa32852,32124,7;
1960-1961 Palomino32450,32226,7. WikiPalomino map32449,32228 is a
walk-radius observation, not permission to rewrite source spawn.

Pinned Canary Appaloosa65-101 agrees price/duration/3uniform/achievement,
but omits escape gate/trigger. Current retainedWiki Appaloosa txt191,
395,458-460 and Palomino txt181,374 confirm24realhours and both-town
escape suspension, so choose Crystal's complete state rule (approved root).
Wiki source: Lavovskii/grimorio@acc9994e2ad8868696bc10ef08070108e2baba92
quests/data/wiki/appaloosa-4870b.json SHA256
0ea6f1e88d2b13c22c8e025d2b0ac737966299ed043fddbb42f634263fb2844f
palomino-f39fe.json SHA256
77e108a15843d141a0fa88014491aeb67b38da7e97b2a10a616cedfedb4b94fe.
Directwiki API remains wiki-pending; retainedHTML is actual full transcript.

## Personal owner reuse
Existing appearance earned_unlocks.rs resolves ACCOUNT permanentfacts only.
NEVER insert these rentals into game_account_appearance_unlocks/receipts.
Add closed character-scoped immutable rental receipts and one active pointer,
with source NPC/CommandRef/binding, world, character, mount22|25|26,
issued/expiresseconds, fee occurrence, sourceRNG, exact source provenance.
All require current Node-secret context, character/session/lease/scope,
current NPC native talk fence; personalcharacter row locked before check.
Mature item_fee_burn::burn_fee_in_transaction supports coins+bank/change
inside same caller Type2TX. Add exact closed HorseRental fee cause, binding
500gold and receipt; do not debit bank with unaudited SQL. Next cause/RNG
numbers require central reservation (no IDs allocated in this plan).
Fee+expiryreceipt+achievement+immutableNPCreply commit atomically.
Stable occurrence draws select exactly one of3 and the independent3/10
Worldtrigger. Retained exact command returns receipt, no new charge/draw.
Another command during active contract refuses even via otherNPC/session.
After expiry, a new rental makes a new immutable receipt/pointer.

Add the3 source-qualified cosmetic metadata/grant mappings through owning
cosmetics generator. Temporary projection returns a matching character-only
earned key only while live; no permanentfacts/account sharing. Route both
read and write validation through the same earned_unlocks owner (existing
read callsites current_appearance115, character_appearance230/337).
Premium wear follows metadata25/26,22free; rental purchase itself staysopen.
Use existing premium owner/coordinator rather than raw evidence inference.
Expired selected rental must project mount=None while preserving outfit,
colours/addons. Existing catalogue::base falls back entireoutfit forinvalid
mount, so add closed rental normalization rather than lose playeroutfit.
Fresh appearance read/revalidate rejects expired rental even before15s tick.
Existing observation/current_mount then naturally publishes dismount/novoice.
15s cadence and first current reconnect/load emit exact contract Advanceonce,
using a persisted expiration notification receipt, no repeated historicalcue.
No new client protocol: existing outfit catalogue/bindings +ServerMessage.

## World horse-station slice, same source-owned frame
No existing horse rule found in world-change daily/scheduled/content owners;
those own daily reset rolls, not transaction-triggered persistent changes.
Add a CLOSED horse_station leaf, not a new general event engine.
Durable per-World state stores generation/start/end(now+86400), sourceRental,
next3h wave boundary, current revision. World row serializes rentaltrigger;
all-townrent/transport reads it in sameTX. Exactcatch termination andexpiry
increment revision. Per-channel physical claims bind live Node/scope and
World generation, like existing Cake claimed source slots. Foreignchannel
claims never create authority without its own current owner.

Crystal appaloosa57-60/126-145: two source rectangles,
32456..32491,32193..32261,z7 and32431..32464,32240..32280,z7;
each3iterations×{WildHorse,Horse,GreyHorse,BrownHorse}=12actors; bothareas total24,
then source3h schedule (104-112). Source-uniform positions, qualify actual
B3 walkable cells/entryframes before admission; no invented map/stable.
Clear stable Horse/Grey/Brown in source rectangles62-70/72-99 ontrigger.
Startup spawn source horse_station_world_change.lua1-58: stablePalomino
32438..32442,32231..32238,z7 countsHorse3/Grey1/Brown2; stableAppaloosa
32846..32850,32114..32120,z7 countsHorse1/Grey1/Brown1; skipwhileescape.
Nativeprofiles already exist for all4 exactcreaturekeys (no aliases/captures).
Reuse actual native spawn_arena_opponent/currentmonster cycle/profile;
source spawn owner must preserve ordinary creature behavior/taming ownership.

Crystal horse_station.lua40-80: actual monster StepIn on sourceitem31159;
only exact nameHorse removed, no player/grey/brown/wild alias. Whileactive,
independent1/30 catch ends Worldevent, stopswaves, restores source9stable;
otherwise merely removesHorse+POFF. Hook actual committed monsterstep before
nextAI like Cake machine arrivals; immutable creature/movement occurrence
makes retries no extra draw/despawn. Currentactualmap31159 required, not
client coordinate/topic. Worldstate revision projects current per-channel
physicalactors; catch from one current channel resumesrent everywhere.
Restart resumes original event/wave boundaries, never rerolls originalrental.

## Focused execution tests, red before green
1. RealNPC hi/rent/yes +500coins/bank mix -> source chosenlook; no permanent
accountfacts, one fee+receipt+achievement; replay no second debit/draw;
insufficientmoney and staleNode/session/lease refuse atomically.
2. OtherNPC/sharedcharacter activecontract refuses; sameaccountothercharacter
has no rentedmount; no premium NPCgate; grey/brown wear checks premium.
3. Native catalogue/select/observer/reconnect before/equal/after expiry;
selectedrental dismount retains exact outfit/addons/colours; writerrefuses
expiredselection; one sourceAdvance, reconnect/replay never renews.
4. SourceRNG boundary trigger3/10 and catch1/30; actualrenttrigger disables
both actualNPCs, source24wave/currentprofiles; 3h/resume no duplicatewaves.
5. RealHorse monsterstep actual31159 -> exact1/30 termination/resume;
failedcatch removesonlyHorse and bodycount; nonHorse/player neverterminates.
6. ActualsourceB3qualification for2NPCs,2stableareas,2raidareas/catchtiles;
actualnative client outfit+contract expiry play from server captures.

0993 SQL append remains S06 sole writer; tables/context/reciprocalfee checks
must be handed over as one reviewed source template before append. No0994.

## Implementation and verification ledger

The native `hi / rent / yes` regression executed before implementation and
failed `NoConversation` in100.618seconds. This is the whole missing-entry RED;
it does not separately prove a fee, expiry, or catch failure. The new writer
uses the existing audited BURN owner and creates a personal Character receipt,
never a permanent Account mount. Its exact source arm and reciprocal deferred
fee receipt must share the same transaction/current Node/session/scope.

Cause473 is `HorseRental`;474 is the independent closed cosmetic NPC purchase.
They reuse the existing typed purchase payload. RNG217selects the horse,
218the source escape trigger,219the catch outcome,220/221the source coordinate;
222is reserved but unused. All numbers are provisional for integration renumbering.

Recursive NPC cash shares only the existing bounded508-leaf physical collector,
denomination/stack-worth planner and custody mutations with Atelier. It has its
own typed proof, source context and reciprocal service receipt. It never decodes
or manufactures a Gem mutation. Mixed material/equipment costs retain their
original cost transaction and one equipment epoch. Source item cost and fee
transaction identities are distinct. The accepted BANK-FEE junior-bank policy,
existing output capacity and refusal when change cannot fit are inherited.

Source waves attempt24creatures (two areas times three loops times four names),
not36. Source positions are tested against the shipped B3. The accepted native
fixed-cell/no-search policy can refuse a blocked or unqualified selected cell;
this is not evidence that all24actors successfully spawned. Startup/return
stable roster attempts9actors. Native actors surviving catch/event expiry are
preserved, as in the donor; only matching stable residents are cleared on escape.
Wave receipts bind current Node incarnation and scope generation. A retry after
an ambiguous commit can reconstruct the source slot; the current native runtime
installs that slot once, at deterministic source positions. An owner restart does
not silently lose the remaining current wave.

Isolated PostgreSQL validation applied all303current migrations and the actual
horse, NPCsource-context and cash overlays. DDL plus11negative source/receipt
cases and256bounded draws passed in4.304seconds. This is parse/security proof,
not native rental/fee/expiry/World gameplay proof. Native focused results are to
be recorded after the coherent implementation is mounted and centrally tested.

The cap25 test records actual server envelopes/client artifact. It retains the
raw historical mount predecessor after expiration while the current catalogue
removes temporary eligibility and effective riding becomes zero. A subsequent
actual appearance command clears the expired mount and changes outfit colours;
no synthetic predecessor or new activation protocol is introduced.
