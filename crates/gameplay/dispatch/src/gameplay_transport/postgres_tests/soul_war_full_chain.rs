//! Soul War entry prerequisites, native death settlement, and one-time reward MINT.
use super::fixture::{Harness, debug};
use super::kill_reward_e2e_fixtures::scenario_thread::run_async;
use super::soul_war_fixtures::{command, current, kill};
use super::w11_chain::prerequisite;
use super::{ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session, reset};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::quest_state::soul_war::{
    SOUL_WAR_REWARD_PLACEMENT, SOUL_WAR_REWARD_REVISION, SoulWarCommand,
};
use crate::durability::reward_claim_mint::{
    RewardClaimMintError, RewardClaimMintOutcome, RewardClaimMintRequest, RewardClaimRefusal,
};
use crate::durability::starter_grant::StarterTemplate;
use crate::foundation::{CommandId, CommandRef, GameSessionId};
use crate::gameplay_transport::item_ref_admission::{
    NativeItemWeights, generation_item_facts, pinned_index,
};
use oteryn_gameplay_encounters::bosses::endgame_entry::EntryRefusal;
use oteryn_gameplay_quests::quest::soul_war::{
    ALL_MINI_BOSSES, FINAL_REWARD_ITEM_IDS, MINIMUM_LEVEL, SoulWarBoss, SoulWarRefusal,
    SoulWarState, SoulWarTransition,
};

const MINIS: [(&str, SoulWarBoss); 5] = [
    ("oteryn:creature.goshnar_s_cruelty", SoulWarBoss::Cruelty),
    ("oteryn:creature.goshnar_s_greed", SoulWarBoss::Greed),
    ("oteryn:creature.goshnar_s_hatred", SoulWarBoss::Hatred),
    ("oteryn:creature.goshnar_s_malice", SoulWarBoss::Malice),
    ("oteryn:creature.goshnar_s_spite", SoulWarBoss::Spite),
];

#[test]
fn soul_war_postgres_five_minis_then_megalomania_claim_once() -> TestResult {
    scenario(Case::Claim)
}

#[test]
fn soul_war_postgres_claim_replay_refused() -> TestResult {
    scenario(Case::Replay)
}

#[test]
fn soul_war_postgres_entry_per_mini_admitted() -> TestResult {
    scenario(Case::Entry)
}

#[derive(Clone, Copy)]
enum Case {
    Claim,
    Replay,
    Entry,
}

fn scenario(case: Case) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    run_async(async move |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "w11chain_soul", false, false).await?;
        let name = match case {
            Case::Claim => stringify!(soul_war_postgres_five_minis_then_megalomania_claim_once),
            Case::Replay => stringify!(soul_war_postgres_claim_replay_refused),
            Case::Entry => stringify!(soul_war_postgres_entry_per_mini_admitted),
        };
        eprintln!("w11chain-marker: {name} db ready");
        let world = crate::foundation::WorldId::decode(&super::fixture::id(42)).map_err(debug)?;
        let (controller, pin) =
            super::death_ml_loss_composed_fixtures::production_generation_with_pin(
                world,
                "content-1",
            )?;
        let room = pin.qualified_room().clone();
        let pin = pin.into_channel_pin();
        let deps = Dependencies::with_content(
            pin.clone(),
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(controller.active().ok_or("active generation")?);
        let mut admitted = deps.admitted;
        owner.admit_progression_session(&mut admitted).await;
        super::qengine_outfit_stories_fixtures::earn_level(
            &owner,
            &admitted,
            &authority,
            MINIMUM_LEVEL,
        )
        .await?;

        let session = admitted.game_session_id;
        assert!(!read(&owner, session).await?.access());
        if matches!(case, Case::Entry) {
            for (key, _) in MINIS {
                assert_eq!(
                    prerequisite(&owner, admitted, key).await?,
                    Err(EntryRefusal::QuestAccess),
                    "{key}"
                );
            }
        }
        let start = command(&owner, session, 501, SoulWarCommand::Start).await?;
        assert!(start.result.is_ok() && start.state.access());
        if matches!(case, Case::Entry) {
            for (key, _) in MINIS {
                assert_eq!(prerequisite(&owner, admitted, key).await?, Ok(()), "{key}");
            }
            drop(owner);
            drop(authority);
            drop(seal);
            return h.cleanup().await;
        }

        let final_entry = "oteryn:creature.goshnar_s_megalomania_purple";
        assert_eq!(
            prerequisite(&owner, admitted, final_entry).await?,
            Err(EntryRefusal::BossChain)
        );
        let mut bits = 0;
        let mut first_taint = None;
        for (index, (key, boss)) in MINIS.into_iter().enumerate() {
            assert_eq!(prerequisite(&owner, admitted, key).await?, Ok(()), "{key}");
            kill(
                &owner,
                &h.pool,
                session,
                120 + u8::try_from(index)?,
                key,
                true,
            )
            .await?;
            let state = read(&owner, session).await?;
            bits |= boss.bit();
            assert_eq!(state.defeated_bosses, bits, "{key}");
            if index == 0 {
                let first = state
                    .first_taint_at
                    .ok_or("first mini must start the taint clock")?;
                first_taint = Some(first);
                // Wait for the next database second so a clock reset can be observed.
                tokio::time::timeout(std::time::Duration::from_secs(2), async {
                    loop {
                        let now: i64 = sqlx::query_scalar(
                            "SELECT floor(extract(epoch FROM clock_timestamp()))::bigint",
                        )
                        .fetch_one(&h.pool)
                        .await?;
                        if now > first {
                            return Ok::<(), sqlx::Error>(());
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                    }
                })
                .await
                .map_err(|_| "database clock did not advance past the first taint")??;
            }
            assert_eq!(state.first_taint_at, first_taint, "{key}");
        }
        assert_eq!(bits, ALL_MINI_BOSSES);
        assert_eq!(prerequisite(&owner, admitted, final_entry).await?, Ok(()));
        kill(
            &owner,
            &h.pool,
            session,
            125,
            "oteryn:creature.goshnar_s_megalomania_blue",
            true,
        )
        .await?;
        let completed = read(&owner, session).await?;
        assert!(completed.completed() && !completed.reward_claimed());
        assert_eq!(
            (completed.defeated_bosses, completed.first_taint_at),
            (0, None)
        );

        claim(&owner, &h, session, &pin, case).await?;
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn read(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
) -> TestResult<SoulWarState> {
    owner
        .root
        .read_soul_war_state(
            owner.character,
            owner.holder,
            current(owner, session).await?,
        )
        .await
        .map_err(debug)
}

async fn claim(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    session: GameSessionId,
    pin: &crate::content::channel_content_pin::ChannelContentPin,
    case: Case,
) -> TestResult {
    let fence = current(owner, session).await?;
    let pending = h
        .root
        .pending_soul_war_reward(owner.character, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("earned Soul War reward")?;
    assert!(FINAL_REWARD_ITEM_IDS.contains(&pending.item_source_id));
    let key = format!("oteryn:item.tibia.i{}", pending.item_source_id);
    let generation = owner.active_generation.ok_or("active generation")?;
    let native = generation.native_gameplay().ok_or("native item facts")?;
    let revision = {
        let runtime = owner.runtime.lock().await;
        pinned_index(&runtime, Some(generation))
            .and_then(|index| index.revision(&key))
            .ok_or("pinned reward definition")?
            .to_owned()
    };
    let item = generation_item_facts(
        native,
        &TypedDefinitionRef {
            family: "Item".into(),
            production_key: key.clone(),
            revision_ref: revision,
        },
    )
    .map_err(debug)?;
    let starter = StarterTemplate::main();
    let backpack = generation_item_facts(native, &starter.definition).map_err(debug)?;
    let weights = NativeItemWeights(native);
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(owner.world_id.as_bytes().as_slice()).bind(owner.channel_id.as_bytes().as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,NULL,$4,$5,$6)")
        .bind(owner.world_id.as_bytes().as_slice()).bind(owner.channel_id.as_bytes().as_slice()).bind(pin.activation_sequence().to_string())
        .bind(pin.server_artifact_digest().as_slice()).bind(pin.client_artifact_digest().as_slice()).bind(pin.frame_binding_digest().as_slice()).execute(&h.pool).await?;
    let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
    h.root
        .initialize_admission_equipment(
            owner.character,
            owner.holder,
            &fence.item_fence(),
            generation.identity().server_artifact_digest(),
            fresh.operation(),
        )
        .await
        .map_err(debug)?;
    h.root
        .grant_starter_items(
            owner.character,
            owner.holder,
            &fence.item_fence(),
            &weights,
            &starter,
        )
        .await
        .map_err(debug)?;
    let mut request = RewardClaimMintRequest {
        backpack_outputs: false,
        additional_items: Vec::new(),
        command: CommandRef::new(session, CommandId::new(601).map_err(debug)?),
        claim: TypedDefinitionRef {
            family: "RewardClaim".into(),
            production_key: pending.claim_key,
            revision_ref: SOUL_WAR_REWARD_REVISION.into(),
        },
        source_placement: SOUL_WAR_REWARD_PLACEMENT.into(),
        item,
        quantity: 1,
        backpack,
        contents: Vec::new(),
        root_text: None,
        root_fluid_subtype: None,
        root_charges_remaining: None,
        content_revision: format!(
            "sha256:{}",
            native
                .source_digest()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        ),
        ruleset_revision: crate::interaction_chest_use::entry_chest::RULESET_REVISION.into(),
        sim_revision: crate::interaction_chest_use::entry_chest::SIM_REVISION.into(),
        achievement: None,
        quest_transition: None,
        key_number: None,
    };
    let outcome = h
        .root
        .claim_pending_soul_war_reward(
            owner.character,
            owner.holder,
            current(owner, session).await?,
            request.clone(),
            &weights,
        )
        .await
        .map_err(debug)?;
    let RewardClaimMintOutcome::Committed(receipt) = outcome else {
        return Err(format!("first claim: {outcome:?}").into());
    };
    assert_eq!(receipt.quantity, 1);
    let claimed = read(owner, session).await?;
    assert!(claimed.reward_claimed());
    let rows = || async {
        sqlx::query_as::<_, (i64, i64, i64)>("SELECT (SELECT count(*) FROM game_reward_claim_mint_receipts WHERE character_id=encode($1,'hex')::uuid AND claim_production_key=$2),(SELECT count(*) FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE e.character_id=encode($1,'hex')::uuid AND i.definition_production_key=$3),(SELECT count(*) FROM game_character_soul_war_reward_entitlements WHERE character_id=encode($1,'hex')::uuid AND state='DELIVERED')")
            .bind(fence.character_id.as_bytes().as_slice())
            .bind(crate::durability::quest_state::soul_war::SOUL_WAR_REWARD_CLAIM)
            .bind(&key).fetch_one(&h.pool).await
    };
    assert_eq!(rows().await?, (1, 1, 1));
    if matches!(case, Case::Replay) {
        let before = current(owner, session).await?;
        request.command = CommandRef::new(session, CommandId::new(602).map_err(debug)?);
        let result = h
            .root
            .claim_pending_soul_war_reward(owner.character, owner.holder, before, request, &weights)
            .await;
        assert!(
            matches!(
                result,
                Err(RewardClaimMintError::Refused(
                    RewardClaimRefusal::AlreadyClaimed
                ))
            ),
            "{result:?}"
        );
        assert_eq!(
            claimed.apply(SoulWarTransition::ClaimReward, 0),
            Err(SoulWarRefusal::RewardAlreadyClaimed)
        );
        assert_eq!(read(owner, session).await?, claimed);
        assert_eq!(
            current(owner, session).await?.expected_character_revision,
            before.expected_character_revision
        );
        assert_eq!(rows().await?, (1, 1, 1));
    }
    Ok(())
}
