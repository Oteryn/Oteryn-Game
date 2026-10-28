# DONOR-F26-1 — wire count bounds

- Task: DONOR-F26-1; issue #710; owner-approved test-only lane.
- Donor locator: `zimbadev/crystalserver@75e9c72e33ce2c3f193e4f2d2ff17ebae4bbfaac`, “fix(protocol): prevent bestiary request dispatcher stall (#832)”. Locator and failure summary supplied by the task; donor source was not copied or independently executed.
- Failure mode: a primitive-valid declared count monopolizes a dispatcher by allocating/iterating before checking remaining bytes/catalogue size, or by wrapping a wire-width loop index.
- Owning contract: [FND-02](../../architecture/FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md), fail-closed parsing and hard resource limits; mechanic-specific maxima remain unchanged.
- Oteryn invariant: check the declaration before body access, iteration or copying. Byte fields borrow a checked slice; counted collections allocate at most their existing supported maximum. Loop counters use `usize`, independent of the wire width.
- Classification: PROVEN by synthetic unit/property fixtures and source-level composition. No gameplay integration, PostgreSQL, production CPU benchmark, donor execution or catalogue equivalence is claimed.
- Decoder fixes: none. All added executable code is test-only, including work counters at the three counted-collection allocation/iteration sites. No format, public production API, dependency or resource limit changes.

## Fixture records

Every source site in the inventory inherits the donor locator, failure mode, FND-02 owner and synthetic classification above. Its PASS is a **compositional prefix-safety result**, not a claim that arbitrary bytes satisfy that field's semantic schema. Each `read_bytes` consumer uses one of the seven independently exercised borrowing readers. Nested messages, strings, packed fields and unknown fields cannot traverse a missing advertised body because the shared reader rejects first.

| Fixture | Sites / invariant | Test locator (in `crates/protocol-oteryn/src/`) | Status |
|---|---|---|---|
| Borrowed protobuf lengths | `charm_wire`, `actor_spell`, `actor_spell_v2`, `actor_spell_item_v2`, `world_object`, `world_spatial`, `account_achievements`; remaining bytes are the structural MAX_SUPPORTED | Each module's `count_bounds_tests::f26_borrowed_length_bounds`; shared `count_bounds_tests::borrowed_length_bounds` | PASS |
| Foundation lengths / skip | policy maximum precedes slice access; unknown fields use checked addition; borrow without allocation | `count_bounds_tests::f26_foundation_bounded_lengths_and_unknown_fields` | PASS |
| Frame / envelope payload | big-endian u32 frame limit; direct envelope payload varint length cannot exceed remaining input | `count_bounds_tests::f26_frame_length_and_envelope_payload` | PASS |
| Cyclopedia visible points | u8 count <=8 and exact `29 + count*5` size before allocation/loop | `count_bounds_tests::f26_cyclopedia_count_bound_precedes_allocation_and_iteration` | PASS |
| Presentation events / cooldowns | u16 count <=1024 / <=4096 before allocation/loop; admitted capacity and iteration count bounded | `count_bounds_tests::f26_spell_collection_counts_are_bounded_before_work` | PASS |
| Presentation event byte length | u8 length; body borrowed before fixed event decode; event length including its prefix <=128 | `spell_presentation_candidate::count_bounds_tests::f26_event_length_and_checked_reader_arithmetic` | PASS |
| Condition appearance keys | u16 BE length; 1..=256; shared by outfit, mount, item and revision keys; copy after range check | `actor_condition_snapshot_candidate::count_bounds_tests::f26_key_length_bounds_before_copy` | PASS |
| Mail text | u16 BE length <=7996 and exact backing size before UTF-8 copy | `count_bounds_tests::f26_mail_text_length_bound_before_copy` | PASS |
| Bounded copied text | every maximum passed by chat (including private/guild), highscores, NPC service and quest log | Those modules' `count_bounds_tests::f26_owned_text_bounds` | PASS |
| Achievement text | key/name/description byte maxima before owned string creation | `account_achievements::string_count_bounds_tests::f26_each_string_limit_before_copy` | PASS |
| Foundation repeated entries | capabilities <=128; revisions <=64 / <=256; checked counter increment, guard before next element decode | `count_bounds_tests::f26_capability_and_revision_repetition_limits` | PASS |
| Snapshot declarations | chunk count <=256 / assembled bytes <=16 MiB; scalar metadata does not drive an inline loop | `count_bounds_tests::f26_snapshot_declarations_do_not_drive_decode_work` | PASS |

The borrowed-reader matrix covers 0, 1, remaining-payload MAX_SUPPORTED, MAX_SUPPORTED+1, u8/u16/u32/u64 maxima, a 16-byte multiplication overflow candidate, and the wire maximum wrap candidate, both backed and truncated. Pointer equality proves borrowing; failed cursors stop at the prefix (<=10 bytes). It also runs the repository's existing deterministic property runner (default 256 cases per reader), with no new dependency. The crate does not use proptest.

The binary count matrices cover 0, 1, MAX_SUPPORTED, MAX_SUPPORTED+1 and wire maximum with full supported fixtures and absent bodies. Thread-local counters record allocation size and loop entry; over-limit declarations produce `(0, 0)` work even with missing/invalid element bytes. In-limit truncated presentation counts may reserve up to MAX_SUPPORTED before rejecting the first missing element; that is the existing accepted bounded behavior. Cyclopedia additionally requires exact backing size before allocation.

Arithmetic qualification: u8/u16 counts are widened to `usize`; even their wire maxima times the largest actual element size fit 32-bit `usize`. No representable count can overflow that multiplication, and no wire-width loop index exists. The maximum input tests exercise these wrap/overflow candidates without manufacturing an unrepresentable wire value. Varint lengths use checked addition (including `u64::MAX` with a nonzero cursor); the presentation reader also directly tests `1 + usize::MAX`. Foundation implicit counters test `usize::MAX` overflow. Protobuf repeated fields carry an element length rather than an aggregate count, so they cannot advertise an unbacked number of entries.

## Search and gameplay decoder scope

Enumerated `read_u8/read_u16/read_u32`, `usize` conversions, `with_capacity`, `for _ in 0..`, varint-to-length conversions, shared readers and their call sites under `crates/protocol-oteryn` and gameplay crates. False positives (quantity scalars, encoder capacity based on owned vectors, test fixtures, and content-artifact/storage decoders) are outside wire command length prefixes.

Gameplay command wrappers delegate to these protocol codecs. `crates/gameplay/dispatch/src/gameplay_transport/character_info.rs:39` accepts a fixed three-byte page request with a response limit; it does not declare an inline collection length. `spell_parameter_cast.rs:49` delegates to `actor_spell_v2`. `tcp_tls.rs:40` and `tcp_tls.rs:87` use `FrameLength::from_prefix` before allocating a frame buffer, so they inherit the tested u32 bound; they are framing consumers rather than additional command-prefix formats. Relay/storage/content formats are not gameplay command decoders.

## Source-site inventory

Locations refer to the authored worktree. Rows are prefix readers and each concrete length-consuming call site, including wrappers (`bytes`, text, singular fields), plus explicit binary count sites. Each row has status PASS through the fixture records above. This inventory intentionally distinguishes shared structural prefix qualification from a separate integration fixture for every semantic message field.

| Site | Prefix-consuming expression | Status / fixture kind |
|---|---|---|
| `crates/protocol-oteryn/src/account_achievements.rs:198` | `let bytes = read_bytes(input, cursor)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/account_achievements.rs:289` | `let value = read_string(input, &mut cursor, MAX_ACHIEVEMENT_KEY_BYTES)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/account_achievements.rs:293` | `let value = read_string(input, &mut cursor, MAX_ACHIEVEMENT_NAME_BYTES)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/account_achievements.rs:297` | `let value = read_string(input, &mut cursor, MAX_ACHIEVEMENT_DESCRIPTION_BYTES)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/account_achievements.rs:363` | `rows.push(decode_row(read_bytes(payload, &mut cursor)?)?);` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/achievement_notices.rs:110` | `(1, 2) => set_once(&mut key, read_text(payload, &mut cursor)?)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/achievement_notices.rs:111` | `(2, 2) => set_once(&mut name, read_text(payload, &mut cursor)?)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/achievement_notices.rs:130` | `let bytes = read_bytes(input, cursor)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/actor_condition_snapshot_candidate.rs:144` | `let n = usize::from(u16::from_be_bytes(self.bytes()?));` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/actor_spell.rs:307` | `let value = decode_position(read_bytes(payload, &mut cursor)?)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/actor_spell_item_v2.rs:101` | `let intent = decode_spell_cast_intent(bytes(input, &mut cursor)?)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/actor_spell_item_v2.rs:105` | `let rune_item_instance = bytes(input, &mut cursor)?` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/actor_spell_item_v2.rs:118` | `let item_instance = bytes(input, &mut cursor)?` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/actor_spell_v2.rs:88` | `intent = Some(decode_spell_cast_intent(bytes(input, &mut cursor)?)?)` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/actor_spell_v2.rs:92` | `let text = std::str::from_utf8(bytes(input, &mut cursor)?)` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/actor_spell_v2.rs:250` | `std::str::from_utf8(bytes(input, &mut cursor)?)` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/actor_spell_v2.rs:310` | `bytes(input, &mut cursor)?` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/actor_spell_v2.rs:317` | `std::str::from_utf8(bytes(input, &mut cursor)?)` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/actor_spell_v2.rs:341` | `std::str::from_utf8(bytes(input, &mut cursor)?)` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/actor_spell_v2.rs:408` | `feedback = Some(decode_feedback(bytes(input, &mut cursor)?)?)` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/actor_spell_v2.rs:410` | `34 if editor.is_none() => editor = Some(decode_editor(bytes(input, &mut cursor)?)?),` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/analyser.rs:213` | `let inner = read_bytes(payload, &mut cursor)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/analyser.rs:233` | `items.push(decode_item(read_bytes(inner, &mut cursor)?)?);` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/analyser.rs:326` | `facts.push(decode_analyser_fact(read_bytes(payload, &mut cursor)?)?);` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/appearance/wire.rs:41` | `2 => Field::Bytes(read_bytes(input, &mut cursor)?),` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/attack.rs:159` | `let bytes: [u8; ENTITY_IDENTITY_BYTES] = read_bytes(input, cursor)?` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/attack.rs:268` | `(1, 2) => set_once(&mut target, decode_target(read_bytes(input, cursor)?)?),` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/attack.rs:324` | `(1, 2) => set_once(&mut target, decode_target(read_bytes(input, cursor)?)?),` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/bed.rs:29` | `decode_item_target(read_bytes(payload, &mut cursor)?)` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/bestiary.rs:143` | `read_uint32_fields::<5>(read_bytes(payload, &mut cursor)?)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/boss_encounter.rs:32` | `let key = std::str::from_utf8(read_bytes(payload, &mut at)?).map_err(\|_\| Error::Malformed)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/charm.rs:225` | `charms.push(decode_charm_state(read_bytes(payload, &mut cursor)?)?);` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/charm_progression.rs:219` | `0x22 => set_once(&mut bestiary, read_bytes(payload, &mut cursor)?)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/charm_progression.rs:220` | `0x2a => set_once(&mut charms, read_bytes(payload, &mut cursor)?)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/chat.rs:239` | `let bytes = read_bytes(input, cursor)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/chat.rs:326` | `let inner = read_bytes(input, cursor)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/chat.rs:344` | `FirstField::Name(read_text(input, cursor, MAX_CHAT_NAME_BYTES)?),` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/chat.rs:346` | `(2, 2) if field <= 3 => set_once(&mut text, read_text(input, cursor, MAX_CHAT_TEXT_BYTES)?),` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/chat.rs:511` | `let identity = read_bytes(input, cursor)?` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/chat.rs:517` | `(4, 2) => set_once(&mut f.name, read_text(input, cursor, MAX_CHAT_NAME_BYTES)?),` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/chat.rs:520` | `(7, 2) => set_once(&mut f.text, read_text(input, cursor, MAX_CHAT_TEXT_BYTES)?),` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/chat.rs:522` | `let position = decode_position(read_bytes(input, cursor)?)` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/chat/guild_room.rs:54` | `(1, 2) => set_once(&mut text, read_text(input, cursor, MAX_CHAT_TEXT_BYTES)?),` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/chat/guild_room.rs:87` | `read_bytes(input, cursor)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/chat/guild_room.rs:103` | `(4, 2) => set_once(&mut name, read_text(input, cursor, MAX_CHAT_NAME_BYTES)?),` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/chat/guild_room.rs:104` | `(7, 2) => set_once(&mut text, read_text(input, cursor, MAX_CHAT_TEXT_BYTES)?),` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/chat/guild_room.rs:107` | `read_bytes(input, cursor)?` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/chat/guild_snapshot.rs:33` | `(1, 2) => set_once(&mut rooms, decode_chat_rooms(read_bytes(input, cursor)?)?),` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/chat/guild_snapshot.rs:37` | `read_bytes(input, cursor)?` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/chat/guild_snapshot.rs:46` | `let bytes = read_bytes(input, cursor)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/chat/private_channel.rs:118` | `(8 \| 9, 1, 2) => set_once(&mut name, read_text(input, cursor, MAX_CHAT_NAME_BYTES)?),` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/chat/private_channel.rs:120` | `(12, 2, 2) => set_once(&mut text, read_text(input, cursor, MAX_CHAT_TEXT_BYTES)?),` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/chat/private_channel.rs:172` | `read_bytes(input, cursor)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/chat/private_channel.rs:187` | `(4, 2) => set_once(&mut name, read_text(input, cursor, MAX_CHAT_NAME_BYTES)?),` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/chat/private_channel.rs:188` | `(7, 2) => set_once(&mut text, read_text(input, cursor, MAX_CHAT_TEXT_BYTES)?),` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/chat/private_channel.rs:259` | `read_bytes(input, cursor)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/chat/private_channel.rs:262` | `(2, 2) => set_once(&mut owner, read_text(input, cursor, MAX_CHAT_NAME_BYTES)?),` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/container_tree.rs:216` | `entries.push(decode_entry(read_bytes(input, &mut cursor)?)?);` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/container_tree.rs:261` | `.push(decode_view(read_bytes(payload, &mut cursor)?)?);` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/container_tree.rs:332` | `0x0a \| 0x12 \| 0x1a => read_bytes(payload, &mut cursor)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/creature_push.rs:42` | `read_bytes(payload, &mut cursor).map_err(\|_\| malformed())?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/creature_push.rs:50` | `read_bytes(payload, &mut cursor).map_err(\|_\| malformed())?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/cyclopedia.rs:109` | `let count = usize::from(bytes[28]);` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/daily_reward.rs:43` | `let pick = read_bytes(bytes, &mut cursor)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/daily_reward.rs:49` | `let value = read_bytes(pick, &mut at)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/depot.rs:181` | `let bytes = read_bytes(bytes, &mut at)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/guild/codec.rs:29` | `2 => Field::Bytes(read_bytes(bytes, &mut at)?),` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/guild/codec.rs:89` | `let body = read_bytes(bytes, &mut at)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/guild/codec/query_result.rs:28` | `let b = read_bytes(bytes, &mut at)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/guild/codec/query_result.rs:37` | `read_bytes(bytes, &mut at)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/guild_bank.rs:115` | `10 => set_once(&mut guild, uuid(read_bytes(bytes, &mut c)?)?)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/guild_bank.rs:120` | `std::str::from_utf8(read_bytes(bytes, &mut c)?)` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/guild_bank.rs:172` | `10 => set_once(&mut guild, uuid(read_bytes(bytes, &mut c)?)?)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/guild_bank.rs:174` | `26 => set_once(&mut after_claim, uuid(read_bytes(bytes, &mut c)?)?)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/guild_bank.rs:233` | `50 => set_once(&mut actor, uuid(read_bytes(bytes, &mut c)?)?)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/guild_bank.rs:318` | `entries.push(decode_entry(read_bytes(bytes, &mut c)?)?);` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/guild_bank.rs:325` | `claims.push(decode_claim(read_bytes(bytes, &mut c)?)?);` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/guild_bank.rs:327` | `66 => set_once(&mut next_claim, uuid(read_bytes(bytes, &mut c)?)?)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/guild_bank.rs:374` | `10 => set_once(&mut guild, uuid(read_bytes(bytes, &mut c)?)?)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/guild_war.rs:66` | `18 => set_once(&mut id, uuid(read_bytes(bytes, &mut c)?)?)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/guild_war.rs:108` | `18 => set_once(&mut war, uuid(read_bytes(bytes, &mut c)?)?)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/highscores.rs:206` | `let bytes = read_bytes(input, cursor)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/highscores.rs:226` | `read_text(input, &mut cursor, MAX_HIGHSCORE_NAME_BYTES)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/highscores.rs:230` | `read_text(input, &mut cursor, MAX_HIGHSCORE_VOCATION_BYTES)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/highscores.rs:275` | `rows.push(decode_row(read_bytes(payload, &mut cursor)?)?);` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/highscores.rs:277` | `0x3a => set_once(&mut own_row, decode_row(read_bytes(payload, &mut cursor)?)?)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/house.rs:46` | `10 => set_once(&mut guild, uuid(read_bytes(body, &mut cursor)?)?)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/house.rs:47` | `18 => set_once(&mut successor, uuid(read_bytes(body, &mut cursor)?)?)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/house.rs:147` | `10 => set_once(&mut world, uuid(read_bytes(body, &mut c)?)?)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/house.rs:148` | `18 => set_once(&mut house, text(read_bytes(body, &mut c)?)?)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/house.rs:175` | `let p = read_bytes(body, &mut c)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/house.rs:205` | `let id = uuid(read_bytes(body, &mut c)?)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/house.rs:244` | `let nested = read_bytes(body, &mut c)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/house.rs:260` | `let id = uuid(read_bytes(nested, &mut at)?)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/house.rs:285` | `10 => set_once(&mut world, uuid(read_bytes(body, &mut c)?)?)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/house.rs:286` | `18 => set_once(&mut town, text(read_bytes(body, &mut c)?)?)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/house.rs:288` | `let raw = read_bytes(body, &mut c)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/house_move_out.rs:68` | `std::str::from_utf8(read_bytes(bytes, &mut at)?)` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/house_move_out.rs:133` | `<[u8; 16]>::try_from(read_bytes(bytes, &mut at)?).map_err(\|_\| Error::Malformed)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/house_transfer.rs:60` | `18 => set_once(&mut key, house(read_bytes(body, &mut cursor)?)?)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/imbuement.rs:254` | `let [slot, family, tier, remaining] = read_row::<4>(read_bytes(bytes, &mut at)?)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/imbuement.rs:263` | `let [family, tier, gold, premium] = read_row::<4>(read_bytes(bytes, &mut at)?)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/inspection.rs:159` | `let intent = decode_item_look_intent_for(read_bytes(input, cursor)?, EXTENDED)` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/inspection.rs:189` | `fields(read_bytes(input, cursor)?, \|key, input, cursor\| {` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/inspection.rs:194` | `decode_item_look_intent_for(read_bytes(input, cursor)?, EXTENDED)` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/inspection.rs:304` | `fields(read_bytes(input, cursor)?, \|key, input, cursor\| {` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/inspection.rs:310` | `42 => set_once(&mut name, text(read_bytes(input, cursor)?)?)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/inspection.rs:317` | `fields(read_bytes(input, cursor)?, \|key, input, cursor\| match key {` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/inspection.rs:318` | `10 => set_once(&mut label, text(read_bytes(input, cursor)?)?),` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/inspection.rs:319` | `18 => set_once(&mut value, text(read_bytes(input, cursor)?)?),` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/inspection.rs:346` | `let intent = decode_item_look_intent_for(read_bytes(input, cursor)?, EXTENDED)` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/item_look.rs:131` | `let reference = read_bytes(payload, &mut cursor)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/item_look.rs:150` | `let bytes = read_bytes(reference, &mut cursor)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/item_look.rs:222` | `let bytes = read_bytes(payload, &mut cursor)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/item_look.rs:236` | `if !read_bytes(payload, &mut cursor)?.is_empty() {` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/item_view.rs:447` | `0x12 => set_once(&mut item, decode_entry(read_bytes(input, &mut cursor)?)?)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/item_view.rs:508` | `decode_entry(read_bytes(payload, &mut cursor)?)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/item_view.rs:516` | `.push(decode_entry(read_bytes(payload, &mut cursor)?)?);` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/item_view.rs:519` | `let equipped = decode_equipped_item(read_bytes(payload, &mut cursor)?)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/item_view.rs:564` | `entries.push(decode_entry(read_bytes(payload, &mut cursor)?)?);` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/item_view.rs:749` | `if !read_bytes(payload, &mut cursor)?.is_empty() {` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/item_view.rs:755` | `let slot = decode_equipment_destination(read_bytes(payload, &mut cursor)?)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/item_view.rs:759` | `let position = decode_position(read_bytes(payload, &mut cursor)?)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/item_view.rs:763` | `let handle = decode_item_target(read_bytes(payload, &mut cursor)?)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/lib.rs:1231` | `5 => read_singular_bytes(` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/lib.rs:1288` | `5 => read_singular_bytes(` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/lib.rs:1354` | `session = Some(GameSessionId::decode(bounded_length_delimited(` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/lib.rs:1362` | `world = Some(WorldId::decode(bounded_length_delimited(` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/lib.rs:1370` | `channel = Some(ChannelId::decode(bounded_length_delimited(` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/lib.rs:1388` | `let packed = bounded_length_delimited(` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/lib.rs:1463` | `pub fn decode_snapshot_begin(` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/lib.rs:1532` | `data = Some(bounded_length_delimited(` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/lib.rs:1567` | `let entry = bounded_length_delimited(` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/lib.rs:1627` | `entry_payload = Some(bounded_length_delimited(` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/lib.rs:1675` | `4 => read_singular_bytes(` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/lib.rs:1803` | `pub fn skip_field(` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/lib.rs:1853` | `pub fn bounded_length_delimited<'a>(` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/lib.rs:1879` | `bounded_length_delimited(` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/lib.rs:1887` | `fn validate_capability(` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/lib.rs:1932` | `let packed = unbounded_length_delimited(payload, cursor)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/lib.rs:1970` | `fn validate_revision_list(` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/lib.rs:1983` | `let revision = unbounded_length_delimited(payload, cursor)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/lib.rs:2015` | `*value = Some(bounded_length_delimited(` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/lib.rs:2094` | `let packed = unbounded_length_delimited(payload, &mut cursor)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/lib.rs:2103` | `read_singular_bytes(` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/lib.rs:2115` | `read_singular_bytes(` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/lib.rs:2125` | `read_singular_bytes(` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/lib.rs:2194` | `bounded_length_delimited(` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/lib.rs:2322` | `let identity = bounded_length_delimited(` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/lib.rs:2396` | `bounded_length_delimited(` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/lib.rs:2418` | `bounded_length_delimited(` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/lib.rs:2464` | `bounded_length_delimited(` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/lib.rs:2508` | `pub fn decode_wire_envelope(input: &[u8]) -> Result<WireEnvelopeView<'_>, FoundationProtocolError> {` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/lib.rs:2585` | `pub fn decode_framed_envelope(` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/mail.rs:50` | `let length = usize::from(u16::from_be_bytes(` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/market.rs:84` | `let value: [u8; 16] = read_bytes(bytes, &mut at)?` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/npc_service.rs:213` | `let bytes: [u8; ENTITY_IDENTITY_BYTES] = read_bytes(input, cursor)?` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/npc_service.rs:255` | `let bytes = read_bytes(input, cursor)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/npc_service.rs:286` | `(1, 2) => set_once(&mut npc, decode_npc(read_bytes(input, cursor)?)?),` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/npc_service.rs:287` | `(2, 2) => set_once(&mut text, read_text(input, cursor, MAX_TALK_TEXT_BYTES)?),` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/npc_service.rs:326` | `(1, 2) => set_once(&mut npc, decode_npc(read_bytes(input, cursor)?)?),` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/npc_service.rs:397` | `(1, 2) => set_once(&mut npc, decode_npc(read_bytes(input, cursor)?)?),` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/npc_service.rs:406` | `lines.push(read_text(input, cursor, MAX_REPLY_LINE_BYTES)?);` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/npc_service.rs:491` | `(1, 2) => set_once(&mut npc, decode_npc(read_bytes(input, cursor)?)?),` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/npc_service.rs:497` | `offers.push(decode_offer(read_bytes(input, cursor)?)?);` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/npc_shop.rs:121` | `(1, 2) => set_once(&mut npc, decode_npc(read_bytes(input, cursor)?)?),` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/npc_shop.rs:196` | `let bytes = read_bytes(input, cursor)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/party.rs:104` | `read_bytes(input, &mut cursor)?` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/party.rs:196` | `entries.push(decode_entry(read_bytes(input, &mut cursor)?)?);` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/party.rs:226` | `std::str::from_utf8(read_bytes(input, &mut cursor)?)` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/party_skull.rs:58` | `let entry = read_bytes(input, &mut cursor)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/party_skull.rs:65` | `read_bytes(entry, &mut at)?` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/player_report.rs:131` | `let raw = read_bytes(bytes, at)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/player_trade.rs:121` | `10 => set_once(&mut identity, id(read_bytes(bytes, &mut at)?)?)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/player_trade.rs:170` | `let inner = read_bytes(bytes, &mut at)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/player_trade.rs:184` | `10 => set_once(&mut body, read_bytes(inner, &mut at)?)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/presentation.rs:527` | `let position = decode_position(read_bytes(input, cursor)?)` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/presentation.rs:536` | `read_bytes(input, cursor)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/presentation.rs:539` | `let bytes: [u8; ENTITY_IDENTITY_BYTES] = read_bytes(input, cursor)?` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/presentation.rs:559` | `read_bytes(input, cursor)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/presentation.rs:650` | `(3..=7, 2) => set_once(&mut kind, decode_kind(number, read_bytes(input, cursor)?)?),` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/presentation.rs:673` | `let entry = read_bytes(payload, &mut cursor)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/presentation.rs:742` | `let inner = read_bytes(input, cursor)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/prey.rs:108` | `std::str::from_utf8(read_bytes(bytes, &mut at)?)` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/prey.rs:240` | `let key = std::str::from_utf8(read_bytes(bytes, at)?).map_err(\|_\| Error::Malformed)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/prey.rs:263` | `slots.push(decode_slot(read_bytes(bytes, &mut at)?)?);` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/quest_log.rs:226` | `let bytes = read_bytes(input, cursor)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/quest_log.rs:241` | `read_string(input, &mut cursor, MAX_MISSION_NAME_BYTES)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/quest_log.rs:245` | `read_string(input, &mut cursor, MAX_JOURNAL_TEXT_BYTES)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/quest_log.rs:274` | `missions.push(decode_mission(read_bytes(input, &mut cursor)?)?);` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/quest_log.rs:300` | `let body = read_bytes(input, &mut cursor)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/quest_log.rs:356` | `QuestLogView::List(decode_list(read_bytes(payload, &mut cursor)?)?),` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/quest_log.rs:360` | `QuestLogView::Line(decode_line(read_bytes(payload, &mut cursor)?)?),` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/quest_log.rs:367` | `.push(decode_line(read_bytes(payload, &mut cursor)?)?);` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/quest_log.rs:428` | `0x0a \| 0x12 \| 0x1a => read_bytes(payload, &mut cursor)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/quick_loot.rs:201` | `(2, 2) => set_once(&mut corpse, uuid(read_bytes(input, cursor)?)?),` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/quick_loot.rs:226` | `(5, 2) => set_once(&mut recipient_id, uuid(read_bytes(input, cursor)?)?),` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/quick_loot.rs:293` | `let row = read_bytes(input, cursor)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/quick_loot.rs:296` | `(1, 2) => set_once(&mut id, uuid(read_bytes(i, c)?)?),` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/spectate/audit.rs:60` | `crate::GameSessionId::decode(read_bytes(input, &mut cursor)?)` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/spectate/codec.rs:64` | `set_once(&mut payload, read_bytes(input, &mut cursor)?)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/spectate/envelope.rs:36` | `34 => set_once(&mut payload, read_bytes(input, &mut cursor)?)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/spell_presentation_candidate.rs:445` | `let count = usize::from(r.u16()?);` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/spell_presentation_candidate.rs:459` | `let len = usize::from(r.u8()?);` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/spell_presentation_candidate.rs:548` | `let count = usize::from(r.u16()?);` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/staff.rs:87` | `let bytes = read_bytes(input, &mut cursor).map_err(\|_\| bad())?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/staff.rs:145` | `read_bytes(input, &mut at).map_err(\|_\| bad())?.to_vec(),` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/stash.rs:137` | `<[u8; 16]>::try_from(read_bytes(bytes, &mut at)?).map_err(\|_\| Error::Malformed)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/stash.rs:222` | `<[u8; 16]>::try_from(read_bytes(bytes, &mut at)?).map_err(\|_\| Error::Malformed)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/stash.rs:230` | `let b = read_bytes(bytes, &mut at)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/stash.rs:238` | `<[u8; 16]>::try_from(read_bytes(b, &mut pos)?)` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/stash.rs:243` | `std::str::from_utf8(read_bytes(b, &mut pos)?)` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/stash.rs:249` | `std::str::from_utf8(read_bytes(b, &mut pos)?)` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/stash.rs:271` | `50 => set_once(&mut message, read_bytes(bytes, &mut at)?)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/target_move.rs:28` | `read_bytes(payload, &mut cursor).map_err(\|_\| malformed())?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/target_move.rs:34` | `read_bytes(payload, &mut cursor).map_err(\|_\| malformed())?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/target_move.rs:38` | `read_bytes(payload, &mut cursor).map_err(\|_\| malformed())?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/task_board.rs:159` | `std::str::from_utf8(read_bytes(bytes, &mut at)?)` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/task_board.rs:208` | `tasks.push(decode_task(read_bytes(bytes, &mut at)?)?);` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/timed_item.rs:99` | `read_bytes(input, &mut cursor)?` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/tutorial_notices.rs:44` | `let text = read_bytes(bytes, &mut cursor)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/vip.rs:54` | `let value = read_bytes(bytes, &mut cursor)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/world_event.rs:71` | `let bytes = read_bytes(payload, &mut cursor)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/world_interaction.rs:92` | `let bytes: [u8; ENTITY_IDENTITY_BYTES] = read_bytes(payload, &mut cursor)?` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/world_map.rs:539` | `read_bytes(input, cursor).map_err(\|_\| WorldMapWireError::Malformed)` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/world_map.rs:586` | `<[u8; 32]>::try_from(bytes(input, &mut cursor)?)` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/world_map.rs:591` | `<[u8; 32]>::try_from(bytes(input, &mut cursor)?)` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/world_map.rs:595` | `0x22 => once(&mut origin, decode_position(bytes(input, &mut cursor)?)?)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/world_map.rs:687` | `0x0a => once(&mut position, decode_position(bytes(input, &mut cursor)?)?)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/world_map.rs:692` | `let item = bytes(input, &mut cursor)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/world_map.rs:729` | `0x0a => once(&mut header, decode_header(bytes(payload, &mut cursor)?)?)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/world_map.rs:734` | `tiles.push(decode_tile(bytes(payload, &mut cursor)?)?);` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/world_map.rs:758` | `0x0a => once(&mut header, decode_header(bytes(payload, &mut cursor)?)?)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/world_map.rs:763` | `let field = bytes(payload, &mut cursor)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/world_object.rs:285` | `let bytes = read_bytes(input, &mut cursor)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/world_object.rs:438` | `let bytes: [u8; ENTITY_IDENTITY_BYTES] = read_bytes(input, &mut cursor)?` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/world_object.rs:445` | `map_item = Some(map_item_handle(read_bytes(input, &mut cursor)?)?);` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/world_object.rs:448` | `let object = decode_world_object_target(read_bytes(input, &mut cursor)?)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/world_object.rs:453` | `let bytes = read_bytes(input, &mut cursor)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/world_object.rs:506` | `read_bytes(payload, &mut cursor)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/world_object.rs:513` | `read_bytes(payload, &mut cursor)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/world_object.rs:518` | `read_bytes(payload, &mut cursor)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/world_object.rs:522` | `target = Some(UseTarget::Item(map_item_handle(read_bytes(` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/world_object.rs:679` | `let bytes: [u8; CONTENT_GENERATION_BYTES] = read_bytes(input, &mut cursor)?` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/world_object.rs:685` | `let bytes = read_bytes(input, &mut cursor)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/world_object.rs:692` | `let bytes = read_bytes(input, &mut cursor)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/world_object.rs:736` | `entry = Some(decode_world_object_overlay_entry(read_bytes(` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/world_object.rs:781` | `entries.push(decode_world_object_overlay_entry(read_bytes(` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/world_spatial.rs:335` | `let bytes: [u8; 32] = read_bytes(payload, &mut cursor)?` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/world_spatial.rs:341` | `position = Some(decode_position(read_bytes(payload, &mut cursor)?)?);` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/world_spatial_entities.rs:285` | `identity_bytes(read_bytes(input, &mut cursor)?)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/world_spatial_entities.rs:317` | `identity_bytes(read_bytes(input, &mut cursor)?)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/world_spatial_entities.rs:322` | `decode_position(read_bytes(input, &mut cursor)?)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/world_spatial_entities.rs:607` | `let bytes = read_bytes(payload, cursor)?` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/world_spatial_entities.rs:614` | `decode_position(read_bytes(payload, cursor)?)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/world_spatial_entities.rs:656` | `0x1a => once(&mut own, identity_bytes(read_bytes(payload, &mut cursor)?)?)?,` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/world_spatial_entities.rs:661` | `let (entity, light) = decode_entity(read_bytes(payload, &mut cursor)?, fields)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/world_spatial_entities.rs:719` | `crate::party_skull::decode_party_icons(read_bytes(payload, &mut cursor)?)` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/world_spatial_entities.rs:734` | `let body = read_bytes(payload, &mut cursor)?;` | PASS / synthetic composition |
| `crates/protocol-oteryn/src/world_weather.rs:96` | `let token: [u8; 32] = read_bytes(payload, &mut cursor)?` | PASS / synthetic composition |

## Validation result

PASS: 21 F26 tests, 507 total protocol tests, package all-target clippy with `-D warnings`, rustfmt, governance validator, 129 governance unit tests and `git diff --check`. Exact commands and the complete changed-file list are recorded in `F26-FINAL.md` at the worktree root. No PostgreSQL test applies to this protocol-only test change.
