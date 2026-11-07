//! Complete source item-grant stories against the stock admitted generation.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::domain::appearance::{AppearanceContent, EarnedCosmetic, UnlockFact};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_items::item_cosmetics::PROVENANCE;
use oteryn_gameplay_quests::quest::reward_callbacks::ItemAmount;
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, UseIntent, UseTarget, encode_item_use_intent,
};

#[test]
fn qengine_l9_afflicted_complete_native_item_story() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-l9-afflicted-story")
}
#[test]
fn qengine_l9_soil_guardian_complete_native_voucher_story() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-l9-soil-guardian-story")
}
#[test]
fn qengine_l9_crystal_warlord_complete_native_voucher_story() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-l9-crystal-warlord-story")
}

#[test]
fn qengine_l9b_soil_guardian_native_supply_package_story() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-l9-soil-guardian-package-story")
}
#[test]
fn qengine_l9b_crystal_warlord_native_supply_package_story() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-l9-crystal-warlord-package-story")
}

pub(super) async fn quantity(h: &Harness, item: u16) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1")
        .bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?)
}
async fn grants(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_account_appearance_unlock_receipts WHERE source_kind='item_action'").fetch_one(&h.pool).await?)
}
pub(super) async fn mint(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    item: u16,
    count: u16,
    ordinal: &mut u64,
) -> TestResult {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: Position {
                x: 33293,
                y: 31677,
                floor: -15,
            },
            item: format!("oteryn:item.tibia.i{item}"),
            count,
        },
        *ordinal,
        true,
    )
    .await?;
    *ordinal += 1;
    Ok(())
}
pub(super) async fn use_native(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    item: u16,
    expected: UseDisposition,
) -> TestResult {
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let source = native
        .item_index()
        .ok_or("item index")?
        .definition_ref(
            "Item",
            &format!("oteryn:item.tibia.i{item}"),
            "definition-r1",
        )
        .ok_or("native item reference")?;
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::new(1).ok_or("handle")?),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(debug)?;
    let command = admitted.continuity.next_command_id;
    let (end, result, actual) =
        super::rust_lottery::exchange(owner, *admitted, command, &payload, Some(source), None)
            .await
            .map_err(|e| format!("native USE {item} command {command}: {e}"))?;
    *admitted = end;
    assert_eq!(result, expected, "native source USE {item}");
    if expected == UseDisposition::Committed {
        let (end, replayed, _) =
            super::rust_lottery::exchange(owner, *admitted, command, &actual, None, None).await?;
        *admitted = end;
        assert_eq!(replayed, expected, "literal source USE replay");
    }
    Ok(())
}
pub(super) async fn mask(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    key: &str,
    expected: u8,
) -> TestResult {
    let observed = owner
        .appearance_observation(
            admitted.runtime_actor.ok_or("actor")?,
            admitted.game_session_id,
            vec![],
        )
        .await
        .ok_or("normal appearance")?;
    let entry = observed
        .catalogue
        .outfits
        .iter()
        .find(|o| o.key == key)
        .ok_or("earned canonical outfit")?;
    assert_eq!(
        entry.permitted_addons, expected,
        "normal native appearance mask"
    );
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    let (family, looks, vouchers) = match name {
        "qengine-rewards-l9-afflicted-story" => ("afflicted", (430, 431), [12551, 12787, 12786]),
        "qengine-rewards-l9-soil-guardian-story"
        | "qengine-rewards-l9-soil-guardian-package-story" => {
            ("soil_guardian", (514, 516), [16252, 16253, 16254])
        }
        "qengine-rewards-l9-crystal-warlord-story"
        | "qengine-rewards-l9-crystal-warlord-package-story" => {
            ("crystal_warlord", (512, 513), [16255, 16256, 16257])
        }
        _ => return Err("unknown native voucher story".into()),
    };
    let package_required = name.contains("-package-story");
    if package_required {
        assert!(
            oteryn_gameplay_items::reward_bag::is_reward_bag("oteryn:item.tibia.i15698"),
            "missing original Gnomish Supply Package action"
        );
    }
    let key = format!("oteryn:outfit.{family}");
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let cosmetics = native.cosmetics().ok_or("stock cosmetics")?;
    let base = format!("oteryn:item-grant.outfit.{}-{}.base", looks.0, looks.1);
    assert_eq!(
        cosmetics.earned_unlock(&base, PROVENANCE, EarnedCosmetic::Outfit(&key)),
        UnlockFact::Compatible,
        "missing admitted source item grant {base}"
    );
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([3, 4, 6, 11, 15, 18, 25]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("capabilities")?;
    owner
        .states
        .chat
        .admit(admitted.game_session_id, "Quest witness", owner.owner_now());
    owner.admit_premium(super::fixture::id(40), admitted.game_session_id);
    assert!(
        owner
            .session_premium_gate(admitted.game_session_id)
            .await
            .premium_current(),
        "ordinary USE has the existing delivered Premium registration"
    );
    let mut ordinal = 600;
    if family == "afflicted" {
        let [npc] = super::classic_quests::install(owner, admitted, ["oteryn:npc.ottokar"]).await?;
        mint(owner, admitted, 12787, 1, &mut ordinal).await?;
        use_native(owner, admitted, 12787, UseDisposition::RequirementNotMet).await?;
        assert_eq!(
            quantity(h, 12787).await?,
            1,
            "base prerequisite cannot spend the addon"
        );
        for item in 12551..=12555 {
            mint(owner, admitted, item, 1, &mut ordinal).await?;
        }
        use_native(owner, admitted, 12551, UseDisposition::RequirementNotMet).await?;
        for item in 12551..=12555 {
            assert_eq!(quantity(h, item).await?, 1, "partial set retained");
        }
        mint(owner, admitted, 12556, 1, &mut ordinal).await?;
        let premium = owner.premium_coordinator.take();
        use_native(owner, admitted, 12551, UseDisposition::RequirementNotMet).await?;
        owner.premium_coordinator = premium;
        for item in 12551..=12556 {
            assert_eq!(
                quantity(h, item).await?,
                1,
                "premium refusal retains all six pieces"
            );
        }
        use_native(owner, admitted, 12551, UseDisposition::Committed).await?;
        for item in 12551..=12556 {
            assert_eq!(
                quantity(h, item).await?,
                0,
                "exact source cloth set consumed"
            );
        }
        mask(owner, admitted, &key, 0).await?;

        let callbacks = oteryn_gameplay_quests::quest::reward_callbacks::load()?;
        let trade = callbacks
            .iter()
            .find(|c| c.key == "oteryn:quest-reward/l9-afflicted-pouch")
            .ok_or("missing Ottokar source exchange")?;
        assert!(
            trade.storage.is_empty(),
            "repeatable source trade has no invented progress flags"
        );
        assert_eq!(
            trade.take,
            vec![ItemAmount {
                fluid_subtype: None,

                item: "oteryn:item.tibia.i12517".into(),
                count: 1
            }]
        );
        assert_eq!(
            trade.give.as_ref().map(|g| g.item.as_str()),
            Some("oteryn:item.tibia.i12413")
        );
        let (result, _, _) =
            super::qengine_rewards::wire_reward(owner, admitted, npc, trade, false).await?;
        assert_eq!(
            result,
            oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Rejected
        );
        for expected in 1..=2 {
            mint(owner, admitted, 12517, 1, &mut ordinal).await?;
            let (result, _, command) =
                super::qengine_rewards::wire_reward(owner, admitted, npc, trade, false).await?;
            assert_eq!(
                result,
                oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Ok
            );
            assert_eq!(quantity(h, 12517).await?, 0, "exact source pouch consumed");
            assert_eq!(
                quantity(h, 12413).await?,
                expected,
                "repeatable native backpack MINT"
            );
            let progress:i64=sqlx::query_scalar("SELECT progress FROM game_character_achievement_progress WHERE achievement_key='oteryn:achievement/doctor_doctor'").fetch_one(&h.pool).await?;
            assert_eq!(
                progress, expected,
                "source Doctor progress once per paid exchange"
            );
            let payload = oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent(
                &oteryn_protocol_oteryn::npc_service::NpcTalkIntent {
                    npc_actor: npc,
                    text: "yes".into(),
                },
            )
            .map_err(debug)?;
            super::quest_world_callbacks::wire(owner, admitted, command, 7, payload).await?;
            assert_eq!(
                quantity(h, 12413).await?,
                expected,
                "literal replay cannot mint twice"
            );
            let progress:i64=sqlx::query_scalar("SELECT progress FROM game_character_achievement_progress WHERE achievement_key='oteryn:achievement/doctor_doctor'").fetch_one(&h.pool).await?;
            assert_eq!(
                progress, expected,
                "literal replay cannot advance achievement twice"
            );
        }
        let claims:i64=sqlx::query_scalar("SELECT count(*) FROM game_reward_claims WHERE claim_production_key LIKE 'oteryn:quest-exchange/%'").fetch_one(&h.pool).await?;
        assert_eq!(
            claims, 2,
            "distinct paid commands retain two one-shot MINT claims; replay adds none"
        );
        // The SQL proof and native identity agree even above signed bigint.
        use sha2::{Digest, Sha256};
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|_| "fence")?
            .ok_or("fence")?;
        let bytes = format!("{}:{}", u64::MAX, trade.key);
        let expected = Sha256::new()
            .chain_update(b"oteryn:quest-exchange:v1\0")
            .chain_update(fence.character_id.as_bytes())
            .chain_update(admitted.game_session_id.as_bytes())
            .chain_update(bytes.as_bytes())
            .finalize();
        let expected = format!(
            "oteryn:quest-exchange/{}",
            expected
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        );
        let actual: String = sqlx::query_scalar(
            "SELECT game_quest_reward_mint_claim_key($1,$2,$3::text::numeric,$4,$5)",
        )
        .bind(sqlx::types::Uuid::from_bytes(
            *fence.character_id.as_bytes(),
        ))
        .bind(sqlx::types::Uuid::from_bytes(
            *admitted.game_session_id.as_bytes(),
        ))
        .bind(u64::MAX.to_string())
        .bind(&trade.key)
        .bind(serde_json::to_value(trade)?)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(
            actual, expected,
            "complete unsigned CommandId identity domain"
        );
        use_native(owner, admitted, 12787, UseDisposition::Committed).await?;
        mask(owner, admitted, &key, 1).await?;
        mint(owner, admitted, 12786, 1, &mut ordinal).await?;
        use_native(owner, admitted, 12786, UseDisposition::Committed).await?;
    } else {
        let [npc] =
            super::classic_quests::install(owner, admitted, ["oteryn:npc.gnomally"]).await?;
        let callbacks = oteryn_gameplay_quests::quest::reward_callbacks::load().map_err(debug)?;
        for (suffix, token, count, voucher, addon) in [
            ("base", 16129, 20, vouchers[0], 0),
            ("first", 16128, 70, vouchers[1], 1),
        ] {
            let callback = callbacks
                .iter()
                .find(|c| {
                    c.key
                        == format!(
                            "oteryn:quest-reward/l9-{}-{suffix}",
                            family.replace('_', "-")
                        )
                })
                .ok_or("missing source token-voucher exchange")?;
            assert!(
                callback.storage.is_empty(),
                "source trade has no invented progress flag"
            );
            assert_eq!(
                callback.take,
                vec![ItemAmount {
                    fluid_subtype: None,

                    item: format!("oteryn:item.tibia.i{token}"),
                    count
                }]
            );
            assert_eq!(
                callback.give.as_ref().map(|g| (&g.item, g.count)),
                Some((&format!("oteryn:item.tibia.i{voucher}"), 1))
            );
            let (result, _, _) =
                super::qengine_rewards::wire_reward(owner, admitted, npc, callback, false).await?;
            assert_eq!(
                result,
                oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Rejected
            );
            mint(owner, admitted, token, count - 1, &mut ordinal).await?;
            let (result, _, _) =
                super::qengine_rewards::wire_reward(owner, admitted, npc, callback, false).await?;
            assert_eq!(
                result,
                oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Rejected,
                "one token short must refuse atomically"
            );
            assert_eq!(quantity(h, token).await?, i64::from(count - 1));
            assert_eq!(
                quantity(h, voucher).await?,
                0,
                "partial payment cannot MINT"
            );
            mint(owner, admitted, token, 2, &mut ordinal).await?;
            let (result, _, _) =
                super::qengine_rewards::wire_reward(owner, admitted, npc, callback, false).await?;
            assert_eq!(
                result,
                oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Ok
            );
            assert_eq!(
                quantity(h, token).await?,
                1,
                "exact source price and retained change"
            );
            assert_eq!(quantity(h, voucher).await?, 1, "native backpack MINT");
            let premium = owner.premium_coordinator.take();
            use_native(owner, admitted, voucher, UseDisposition::RequirementNotMet).await?;
            owner.premium_coordinator = premium;
            assert_eq!(
                quantity(h, voucher).await?,
                1,
                "unpaid premium gate cannot consume voucher"
            );
            use_native(owner, admitted, voucher, UseDisposition::Committed).await?;
            assert_eq!(quantity(h, voucher).await?, 0);
            mask(owner, admitted, &key, addon).await?;
        }
        if package_required {
            let callback = callbacks
                .iter()
                .find(|c| c.key == "oteryn:quest-reward/l9-gnomish-supply")
                .ok_or("source supply trade")?;
            assert_eq!(
                callback.take,
                vec![ItemAmount {
                    fluid_subtype: None,

                    item: "oteryn:item.tibia.i16128".into(),
                    count: 2
                }]
            );
            mint(owner, admitted, 16128, 1, &mut ordinal).await?;
            let (result, _, _) =
                super::qengine_rewards::wire_reward(owner, admitted, npc, callback, false).await?;
            assert_eq!(
                result,
                oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Ok
            );
            assert_eq!(
                quantity(h, 15698).await?,
                1,
                "actual Gnomally supply-package MINT"
            );
            let source:sqlx::types::Uuid=sqlx::query_scalar("SELECT i.item_instance_id FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key='oteryn:item.tibia.i15698'").fetch_one(&h.pool).await?;
            let definition = crate::durability::item_mint::TypedDefinitionRef {
                family: "Item".into(),
                production_key: "oteryn:item.tibia.i15698".into(),
                revision_ref: "definition-r1".into(),
            };
            let native = owner
                .active_generation
                .and_then(|g| g.native_gameplay())
                .ok_or("native")?;
            let wanted = format!("oteryn:item.tibia.i{}", vouchers[2]);
            let first = admitted.continuity.next_command_id;
            let witness = (first..first + 10000)
                .find(|command| {
                    let draw = oteryn_gameplay_items::reward_bag::reward_bag_draw(
                        *admitted.game_session_id.as_bytes(),
                        *command,
                        *source.as_bytes(),
                    );
                    oteryn_gameplay_items::reward_bag::reward_bag_conversion(
                        native,
                        &definition,
                        draw,
                    )
                    .is_some_and(|plan| {
                        plan.outputs
                            .iter()
                            .any(|(item, count)| item.production_key == wanted && *count == 1)
                    })
                })
                .ok_or("reachable unchanged source rare-voucher draw")?;
            // Advance through real harmless NPC commands. Never rewrite a bag
            // identity, skip command admission or change a production probability.
            while admitted.continuity.next_command_id < witness {
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(std::time::Duration::from_secs(2))
                    .ok_or("clock")?;
                let command = admitted.continuity.next_command_id;
                let payload = oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent(
                    &oteryn_protocol_oteryn::npc_service::NpcTalkIntent {
                        npc_actor: npc,
                        text: "hi".into(),
                    },
                )
                .map_err(debug)?;
                super::quest_world_callbacks::wire(owner, admitted, command, 7, payload).await?;
                assert_eq!(
                    admitted.continuity.next_command_id,
                    command + 1,
                    "ordinary admitted witness command"
                );
            }
            use_native(owner, admitted, 15698, UseDisposition::Committed).await?;
            assert_eq!(
                quantity(h, 15698).await?,
                0,
                "package consumed exactly once, including literal replay"
            );
            assert_eq!(
                quantity(h, vouchers[2]).await?,
                1,
                "rare voucher came from ordinary package USE, not a fixture MINT"
            );
        } else {
            mint(owner, admitted, vouchers[2], 1, &mut ordinal).await?;
        }
        use_native(owner, admitted, vouchers[2], UseDisposition::Committed).await?;
    }
    // A free gift cannot borrow the paid-exchange receipt arm. Check this
    // specific deferred guard before the other same-transaction proof guards.
    let mut tx = h.pool.begin().await?;
    sqlx::query("INSERT INTO game_character_quest_receipts SELECT (jsonb_populate_record(NULL::game_character_quest_receipts,to_jsonb(r)||jsonb_build_object('cause_ordinal',999999,'transition_key','oteryn:quest-reward/qengine2-free-voucher','track_keys','[]'::jsonb,'values_before','[]'::jsonb,'values_after','[]'::jsonb,'completes',false,'original_character_revision',999998,'committed_character_revision',999999,'created_xact_id',pg_current_xact_id()::text))).* FROM game_character_quest_receipts r LIMIT 1").execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_quest_reward_sources SELECT (jsonb_populate_record(NULL::game_quest_reward_sources,to_jsonb(s)||jsonb_build_object('cause_ordinal',999999,'mint_transaction_id',NULL,'transition_key','oteryn:quest-reward/qengine2-free-voucher','plan',s.plan||jsonb_build_object('key','oteryn:quest-reward/qengine2-free-voucher','take','[]'::jsonb),'created_xact_id',pg_current_xact_id()::text))).* FROM game_quest_reward_sources s LIMIT 1").execute(&mut *tx).await?;
    let error = sqlx::query("SET CONSTRAINTS game_quest_reward_value_only_complete IMMEDIATE")
        .execute(&mut *tx)
        .await;
    assert!(
        matches!(error,Err(sqlx::Error::Database(ref e)) if e.code().as_deref()==Some("23514") && e.message().contains("lacks paid MINT or one-shot grant")),
        "free voucher must fail the paid exchange guard: {error:?}"
    );
    tx.rollback().await?;
    mask(owner, admitted, &key, 3).await?;
    assert_eq!(
        grants(h).await?,
        3,
        "one base and exactly two source addons"
    );
    let achievement = match family {
        "afflicted" => "beak_doctor",
        "soil_guardian" => "funghitastic",
        _ => "crystal_clear",
    };
    let earned: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_account_achievements WHERE achievement_key=$1",
    )
    .bind(format!("oteryn:achievement/{achievement}"))
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(earned, 1, "complete source outfit achievement");
    mint(owner, admitted, vouchers[1], 1, &mut ordinal).await?;
    use_native(owner, admitted, vouchers[1], UseDisposition::Committed).await?;
    assert_eq!(
        quantity(h, vouchers[1]).await?,
        1,
        "already-owned grant never charges the new source"
    );
    assert_eq!(grants(h).await?, 3);
    Ok(())
}
