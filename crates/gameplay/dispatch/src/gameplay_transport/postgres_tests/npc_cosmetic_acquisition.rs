//! S06: current World's Norma source through real command 7, item custody and appearance facts.
use super::fixture::{Harness, debug, fence, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::foundation::{
    CharacterLease, MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements,
};
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use std::sync::Arc;

#[test]
fn srv7_npc_norma_real_talk_costs_grant_refusal_replay_and_reconnect() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(false).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "NPC cosmetic acquisition thread panicked")??;
    Ok(())
}

fn scenario(hairycles: bool) -> TestResult {
    scenario_for(if hairycles {
        "oteryn:npc.hairycles"
    } else {
        "oteryn:npc.norma"
    })
}
fn scenario_for(npc_key: &'static str) -> TestResult {
    scenario_for_world(npc_key, false)
}
fn scenario_for_world(npc_key: &'static str, hanna_world: bool) -> TestResult {
    scenario_for_trigger(npc_key, hanna_world, false)
}
fn scenario_for_trigger(npc_key: &'static str, hanna_world: bool, ajax_greet: bool) -> TestResult {
    scenario_for_ajax_case(
        npc_key,
        hanna_world,
        if ajax_greet {
            AjaxCase::Greet
        } else {
            AjaxCase::Fire
        },
    )
}
#[derive(Clone, Copy)]
enum AjaxCase {
    Fire,
    Greet,
    Lifecycle,
}
fn scenario_for_ajax_case(
    npc_key: &'static str,
    hanna_world: bool,
    ajax_case: AjaxCase,
) -> TestResult {
    scenario_for_primal_case(npc_key, hanna_world, ajax_case, false)
}
fn scenario_for_primal_case(
    npc_key: &'static str,
    hanna_world: bool,
    ajax_case: AjaxCase,
    primal_below_prize: bool,
) -> TestResult {
    run(async |admin| {
        reset();
        let hairycles = npc_key == "oteryn:npc.hairycles";
        let royal = npc_key == "oteryn:npc.king_tibianus";
        let registered_greet = matches!(
            npc_key,
            "oteryn:npc.bron" | "oteryn:npc.bozo" | "oteryn:npc.a_majestic_warwolf"
        );
        let navigator_source = npc_key == "oteryn:npc.navigator";
        let simon_lifetime_source = npc_key == "oteryn:npc.hugo";
        let irmana_source = npc_key == "oteryn:npc.irmana";
        let sandra_source = npc_key == "oteryn:npc.sandra";
        let gnomilly_source = npc_key == "oteryn:npc.gnomilly";
        let started = std::time::Instant::now();
        let h = if npc_key == "oteryn:npc.bozo" {
            Harness::create_female_appearance_with_admission_seed(
                admin,
                "srv7_npc_cosmetics",
                false,
                false,
            )
            .await?
        } else {
            Harness::create_appearance_with_admission_seed(
                admin,
                if hairycles {
                    "srv7_npc_hairycles"
                } else {
                    "srv7_npc_cosmetics"
                },
                false,
                false,
            )
            .await?
        };
        let royal_house_evidence = if royal && royal::full() {
            Some(super::house_transfer::RecoverySource::start(&h).await?)
        } else {
            None
        };
        if royal_house_evidence.is_some() {
            // The native House continuation starts under the SAME existing
            // composition publisher as every other real House fixture.
            super::house_transfer::prepare_owned_session(&h).await?;
        } else {
            prepare_current_session(&h).await?;
        }
        super::item_look::seed_main_backpack(
            &h,
            &[
                ("oteryn:item.tibia.i3374", 1),
                ("oteryn:item.tibia.i5890", 50),
                ("oteryn:item.tibia.i5902", 50),
            ],
        )
        .await?;
        // Arrange the real source operand in Head custody, not only the backpack.
        // The production NPC exchange must consume it and remove its armor effect.
        let mut setup = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *setup)
            .await?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1) ON CONFLICT(character_id) DO NOTHING")
            .bind(id(41).as_slice()).execute(&mut *setup).await?;
        sqlx::query(
            "DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid",
        )
        .bind(id(71).as_slice())
        .execute(&mut *setup)
        .await?;
        sqlx::query("INSERT INTO game_character_equipment_slots(character_id,world_id,slot,item_instance_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,encode($3,'hex')::uuid,encode($3,'hex')::uuid)")
            .bind(id(41).as_slice()).bind(id(42).as_slice()).bind(id(71).as_slice()).execute(&mut *setup).await?;
        setup.commit().await?;
        let equipment_before: i64 = sqlx::query_scalar("SELECT revision::bigint FROM game_character_equipment_state WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world_id = fence(1)?.runtime_scope.world_id();
        let (controller, pin) = super::death_ml_loss_composed::production_generation_with_source(
            world_id,
            "content-1",
            None,
        )?;
        let active = controller.active().ok_or("active generation")?;
        let (bundle, actual_npcs) = if simon_lifetime_source {
            let (bundle, npcs) = simon_lifetime::world(
                world_id,
                active.native_gameplay().ok_or("Hugo/Simon native")?,
            )?;
            (bundle, Some(npcs))
        } else if irmana_source {
            let (bundle, npcs) =
                irmana::world(world_id, active.native_gameplay().ok_or("Irmana native")?)?;
            (bundle, Some(npcs))
        } else if registered_greet {
            let (bundle, npcs) = registered_greet::world(
                npc_key,
                world_id,
                active.native_gameplay().ok_or("registered greet native")?,
            )?;
            (bundle, Some(npcs))
        } else if navigator_source {
            let (bundle, npcs) = navigator::world(
                world_id,
                active.native_gameplay().ok_or("Navigator native")?,
            )?;
            (bundle, Some(npcs))
        } else if sandra_source {
            let (bundle, npcs) =
                sandra::world(world_id, active.native_gameplay().ok_or("Sandra native")?)?;
            (bundle, Some(npcs))
        } else if gnomilly_source {
            let (bundle, npcs) =
                gnomilly::world(world_id, active.native_gameplay().ok_or("Gnomilly native")?)?;
            (bundle, Some(npcs))
        } else if hanna_world {
            let (bundle, npcs) =
                hanna_world::world(world_id, active.native_gameplay().ok_or("Hanna native")?)?;
            (bundle, Some(npcs))
        } else if royal {
            let (bundle, npcs) =
                royal::world(world_id, active.native_gameplay().ok_or("royal native")?)?;
            (bundle, Some(npcs))
        } else {
            (super::grizzly_npc::wire::world(world_id)?, None)
        };
        let entry = if simon_lifetime_source {
            simon_lifetime::ENTRY
        } else if irmana_source {
            irmana::ENTRY
        } else if registered_greet {
            registered_greet::entry(npc_key)
        } else if navigator_source {
            navigator::entry()
        } else if sandra_source {
            sandra::ENTRY
        } else if gnomilly_source {
            gnomilly::ENTRY
        } else if hanna_world {
            hanna_world::ENTRY
        } else if royal {
            royal::ENTRY
        } else {
            (2, 0, 7)
        };
        let cells = bundle
            .movement_cells(pin.qualified_room().movement_cells())
            .map_err(debug)?;
        let room = pin
            .qualified_room()
            .bundle_view(
                cells.clone(),
                bundle.frame_binding_digest(world_id),
                bundle.map_revision_digest(),
                crate::content::NativeEntryStart {
                    x: entry.0,
                    y: entry.1,
                    floor: entry.2,
                },
            )
            .ok_or("bundle room")?;
        let channel_pin = crate::foundation::ChannelContentPin::from_activation(
            world_id,
            1,
            pin.identity().server_artifact_digest(),
            if royal
                || gnomilly_source
                || hanna_world
                || sandra_source
                || navigator_source
                || irmana_source
                || simon_lifetime_source
                || registered_greet
            {
                room.compiled().client_digest()
            } else {
                pin.identity().client_artifact_digest()
            },
            room.frame_binding().digest(),
            room.map_revision_digest(),
            entry,
        );
        // The in-transaction custody owner independently reads this real activation.
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4) ON CONFLICT DO NOTHING")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(channel_pin.server_artifact_digest().as_slice())
            .bind(channel_pin.client_artifact_digest().as_slice())
            .bind(channel_pin.frame_binding_digest().as_slice()).execute(&h.pool).await?;
        let deps = Dependencies::with_content_and_capacity(
            channel_pin,
            room,
            active
                .native_gameplay()
                .ok_or("native")?
                .spell_book()
                .clone(),
            if simon_lifetime_source {
                4
            } else if gnomilly_source && gnomilly::full() {
                // The activation owner reserves half a small carrier for player
                // headroom. Preserve every actual source NPC and all seven
                // original targets; derive this fixture's capacity from that census.
                let npcs = actual_npcs.as_ref().ok_or("actual Gnom source NPC table")?;
                let placed = npcs
                    .npcs
                    .iter()
                    .map(|npc| npc.placements.len())
                    .sum::<usize>();
                let occupied = placed.checked_add(1).ok_or("fixture census overflow")?;
                occupied
                    .checked_add(7)
                    .and_then(|total| total.checked_mul(2))
                    .ok_or("fixture spawn capacity overflow")?
            } else {
                2
            },
        )?;
        let simon_account: Option<[u8; 16]> = if simon_lifetime_source {
            let account:sqlx::types::Uuid=sqlx::query_scalar("SELECT account_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
            Some(*account.as_bytes())
        } else {
            None
        };
        let simon_premium = if let Some(account) = simon_account {
            use crate::gameplay_transport::spell_premium_coordinator::{
                SpellPremiumCoordinator, purchase_fixture,
            };
            let (consumer, clock, activation, _) = purchase_fixture::current(account);
            h.root
                .latch_premium_activation(
                    &activation.activation_id,
                    activation.switch_over_us,
                    activation.switch_over_us + 60_000_000,
                )
                .await
                .map_err(debug)?;
            Some(SpellPremiumCoordinator::new(
                consumer,
                Some(clock),
                Some(activation),
                Some(h.root.clone()),
            ))
        } else {
            None
        };
        let mut owner = deps.owner(&h, &authority);
        if (npc_key == "oteryn:npc.ajax" && matches!(ajax_case, AjaxCase::Lifecycle))
            || (gnomilly_source && gnomilly::full())
        {
            // deps.room is the real bundle_view built above from this first
            // published pin/cells/start, exactly the production combat seam.
            owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&deps.room));
        }
        owner.active_generation = Some(active);
        if simon_lifetime_source {
            owner.premium_coordinator = simon_premium.as_ref();
        }
        if navigator_source {
            owner.quest_catalogue = Some(Arc::new(
                oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                    .map_err(debug)?
                    .catalogue()
                    .clone(),
            ));
        }
        owner.movement_cells = &cells;
        if let Some(evidence) = &royal_house_evidence {
            owner.evidence = &evidence.evidence;
        }
        let actor = deps.actor;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        if simon_lifetime_source {
            owner.admit_premium(
                simon_account.ok_or("actual Simon account")?,
                admitted.game_session_id,
            );
        }
        owner.admit_progression_session(&mut admitted).await;
        {
            let mut runtime = owner.runtime.lock().await;
            runtime
                .initialize_first_entry_position(actor)
                .map_err(debug)?;
            runtime
                .bind_attacker_lease(
                    actor,
                    admitted.game_session_id,
                    CharacterLease::new(
                        crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                        2,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
        }
        let current = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(debug)?
            .ok_or("current fence")?;
        admitted.item_fence = Some(current.item_fence());
        let (crate::gameplay_transport::spell_character_facts::CastFactsLoad::Ready {
            facts, ..
        }
        | crate::gameplay_transport::spell_character_facts::CastFactsLoad::NoVocation {
            facts,
            ..
        }) = crate::gameplay_transport::spell_character_facts::load_character_cast_facts(
            &h.root,
            &authority,
            current.character_id,
        )
        .await
        else {
            return Err("durable NPC customer build".into());
        };
        {
            let runtime = owner.runtime.lock().await;
            assert!(
                owner
                    .states
                    .spell_states
                    .lock()
                    .await
                    .initialize(
                        &runtime,
                        actor,
                        admitted.game_session_id,
                        facts,
                        (0, 0),
                        owner.owner_now()
                    )
                    .is_some()
            );
        }
        {
            let _permit = owner.spell_lane_permit().await.ok_or("equipment lane")?;
            let effects = owner
                .read_admission_effects(admitted.game_session_id)
                .await
                .map_err(|()| "worn source effects")?;
            let runtime = owner.runtime.lock().await;
            owner
                .install_admission_suppressions(&runtime, actor, admitted.game_session_id, effects)
                .await
                .ok_or("install worn source effects")?;
            assert!(
                owner
                    .states
                    .spell_states
                    .lock()
                    .await
                    .get(&runtime, actor, admitted.game_session_id)
                    .ok_or("worn customer state")?
                    .equipped_armor()
                    > 0,
                "the source helmet contributes armor before exchange"
            );
        }
        if hairycles {
            hairycles::historical_stage(&h, 17).await?;
            owner.admit_quest_session(&admitted).await;
        }
        let model = NpcDataCatalogue::load(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
            std::ffi::OsStr::new("world"),
            npc_catalogue_preproduction_limits(),
        )
        .map_err(debug)?
        .service_model_for_project()
        .map_err(debug)?;
        let book = Arc::new(NpcTalkBook::from_model(&model));
        assert!(book.get(npc_key).is_some(), "real source NPC talk profile");
        owner.states.chat.install_npc_talk(Arc::clone(&book));
        owner
            .states
            .chat
            .admit(admitted.game_session_id, "Citizen", owner.owner_now());
        owner.states.chat.set_npc_service(admitted.game_session_id);
        let npc = {
            let mut runtime = owner.runtime.lock().await;
            runtime
                .place_npcs(
                    actual_npcs.as_ref().unwrap_or(&NpcPlacementTable {
                        npcs: vec![NpcPlacements {
                            key: npc_key.into(),
                            placements: vec![NpcPlacement {
                                x: 3,
                                y: 0,
                                floor: -7,
                                direction: MovementFacing::West,
                            }],
                        }],
                    }),
                    |key| book.get(key).is_some(),
                    true,
                )
                .map_err(debug)?;
            let placed = runtime
                .visible_entities()
                .npcs
                .into_iter()
                .find(|n| runtime.npc_key(n.actor) == Some(npc_key))
                .ok_or("actual requested source NPC actor")?;
            oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
                identity: placed.actor.placement_identity(),
                generation: placed.generation,
            }
        };
        let offered = if hanna_world {
            crate::gameplay_transport::appearance_change::offer::offered(
                owner.offered_capabilities(),
                owner.appearance_available(),
            )
        } else {
            owner.offered_capabilities().to_vec()
        };
        admitted.continuity.selected_capabilities =
            crate::gameplay_transport::capabilities::SelectedCapabilities::select(
                &offered,
                if royal && royal::full() {
                    &[
                        3, 4, 6, 7, 11, 15, 18, 28, 37, 49, 62, 86, 153, 207, 385, 386,
                    ]
                } else if npc_key == "oteryn:npc.gnomadness" {
                    &[3, 6, 7, 11, 15, 18, 37, 92, 153, 207, 385]
                } else if simon_lifetime_source || (navigator_source && navigator::owned()) {
                    &[3, 4, 6, 11, 15, 18, 385]
                } else if hanna_world {
                    &[3, 6, 25, 385]
                } else {
                    &[3, 6, 385]
                },
            )
            .ok_or("NPC capability")?;
        if royal && royal::full() {
            // This gameplay helper starts with an inventory-only boot binding. The
            // coupled real-world pipeline also admits into a House, so bind its actual
            // negotiated subset using the same ADMIT0 boot owner as House map fixtures.
            // Every row/dependency comes from the real composed offer; no capability is
            // invented or added to the selected session at the handoff boundary.
            let boot_offers: &'static [crate::gameplay_transport::capabilities::OfferedCapability] =
        Box::leak(owner.offered_capabilities().iter().copied()
            .filter(|entry|admitted.continuity.selected_capabilities.contains(entry.id))
            .collect::<Vec<_>>().into_boxed_slice());
            owner.required_capabilities = crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(
        &[11,18], crate::foundation::RuntimeScopeRefV1::channel(admitted.world_id,admitted.channel_id),
        1,"policy-1",boot_offers,
    ).map_err(debug)?;
            assert_eq!(
                owner
                    .required_capabilities
                    .authenticate_resume_support(
                        admitted.continuity.selected_capabilities.as_slice(),
                        admitted.continuity.selected_capabilities,
                    )
                    .map_err(debug)?
                    .selected(),
                admitted.continuity.selected_capabilities,
                "Royal native House handoff retains the boot-qualified negotiated selection"
            );
        }
        if hanna_world {
            assert!(
                admitted
                    .continuity
                    .selected_capabilities
                    .contains(oteryn_protocol_oteryn::appearance::CAPABILITY_APPEARANCE_V1),
                "actual availability-qualified appearance offer must select25"
            );
        }
        if registered_greet {
            registered_greet::exercise(&h, &mut owner, &mut admitted, npc, npc_key).await?;
            drop(owner);
            drop(authority);
            drop(seal);
            return h.cleanup().await;
        }
        if navigator_source {
            navigator::exercise(&h, &mut owner, &mut admitted, npc, actor).await?;
            drop(owner);
            drop(authority);
            drop(seal);
            return h.cleanup().await;
        }
        if simon_lifetime_source {
            simon_lifetime::exercise(&h, &mut owner, &mut admitted, npc).await?;
            drop(owner);
            drop(authority);
            drop(seal);
            return h.cleanup().await;
        }
        if irmana_source {
            irmana::exercise(&h, &mut owner, &mut admitted, npc).await?;
            drop(owner);
            drop(authority);
            drop(seal);
            return h.cleanup().await;
        }
        if sandra_source {
            sandra::exercise(&h, &mut owner, &mut admitted, npc).await?;
            drop(owner);
            drop(authority);
            drop(seal);
            return h.cleanup().await;
        }
        if gnomilly_source {
            gnomilly::exercise(&h, &mut owner, &mut admitted, npc).await?;
            drop(owner);
            drop(authority);
            drop(seal);
            return h.cleanup().await;
        }
        if npc_key == "oteryn:npc.vescu" || npc_key == "oteryn:npc.gnomadness" {
            if npc_key == "oteryn:npc.vescu" {
                vescu::exercise(&h, &mut owner, &mut admitted, npc).await?;
            } else {
                primal::exercise(&h, &mut owner, &mut admitted, npc, primal_below_prize).await?;
            }
            drop(owner);
            drop(authority);
            drop(seal);
            return h.cleanup().await;
        }
        if npc_key == "oteryn:npc.ajax" {
            match ajax_case {
                AjaxCase::Greet => ajax::exercise_greet(&h, &mut owner, &mut admitted, npc).await?,
                AjaxCase::Fire => ajax::exercise(&h, &mut owner, &mut admitted, npc).await?,
                AjaxCase::Lifecycle => {
                    ajax::exercise_lifecycle(&h, &mut owner, &mut admitted, npc).await?
                }
            }
            drop(owner);
            drop(authority);
            drop(seal);
            return h.cleanup().await;
        }
        if royal {
            royal::exercise(&h, &mut owner, &mut admitted, npc).await?;
            drop(owner);
            drop(authority);
            drop(seal);
            return h.cleanup().await;
        }
        if hairycles {
            hairycles::exercise(&h, &mut owner, &mut admitted, npc).await?;
            drop(owner);
            drop(authority);
            drop(seal);
            return h.cleanup().await;
        }
        let first_talk = super::grizzly_npc::wire::talk(
            &mut owner,
            &mut admitted,
            npc,
            &[(1, "hi"), (2, "outfit"), (3, "yes")],
        )
        .await;
        if first_talk.is_err() {
            let receipts: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_npc_cosmetic_receipts WHERE command_id=3",
            )
            .fetch_one(&h.pool)
            .await?;
            let tracks:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_quest_tracks WHERE track_key LIKE 'oteryn:quest-progress/quest/u7_8/citizen_outfits_rook/%'").fetch_one(&h.pool).await?;
            eprintln!("SRV7_NORMA_FIRST_TALK receipts={receipts} tracks={tracks}");
        }
        first_talk?;
        super::grizzly_npc::wire::talk(
            &mut owner,
            &mut admitted,
            npc,
            &[(4, "hi"), (5, "hat"), (6, "yes")],
        )
        .await?;
        let grant = if npc_key == "oteryn:npc.hanna" {
            "oteryn:appearance-grant/npc/hanna/citizen/addon2"
        } else {
            "oteryn:appearance-grant/npc/norma/citizen/addon2"
        };
        assert_eq!(
            grants(&h, grant).await?,
            0,
            "insufficient feathers grant nothing"
        );
        let before: i64 = sqlx::query_scalar("SELECT sum(quantity)::bigint FROM game_item_instances WHERE item_instance_id IN(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid)")
            .bind(id(71).as_slice()).bind(id(72).as_slice()).bind(id(73).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(before, 101, "a refused multi-item cost spends no prefix");
        super::item_forge::support::seed_items(&h, &[(90, "oteryn:item.tibia.i5890", 50, 0)])
            .await?;
        let (effects, appearance_frames) = if hanna_world {
            let frames = super::grizzly_npc::wire::talk_appearance(
                &mut owner,
                &mut admitted,
                npc,
                &[(7, "hi"), (8, "hat"), (9, "yes")],
            )
            .await?;
            let effects = hanna_world::effects(&frames)?;
            hanna_world::assert_catalogues(&frames, false, true)?;
            (effects, frames)
        } else {
            (
                super::grizzly_npc::wire::talk_effects(
                    &mut owner,
                    &mut admitted,
                    npc,
                    &[(7, "hi"), (8, "hat"), (9, "yes")],
                )
                .await?,
                Vec::new(),
            )
        };
        assert_eq!(
            grants(&h, grant).await?,
            1,
            "real NPC source must grant Citizen addon 2"
        );
        hairycles::assert_blue(&effects)?;
        let equipment_after: i64 = sqlx::query_scalar("SELECT revision::bigint FROM game_character_equipment_state WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(
            equipment_after,
            equipment_before + 1,
            "worn source cost advances its equipment owner once"
        );
        {
            let runtime = owner.runtime.lock().await;
            assert_eq!(
                owner
                    .states
                    .spell_states
                    .lock()
                    .await
                    .get(&runtime, actor, admitted.game_session_id)
                    .ok_or("customer after source cost")?
                    .equipped_armor(),
                0,
                "committed NPC hand-in removes the worn helmet armor immediately"
            );
        }
        let presentation = owner
            .appearance_observation(deps.actor, admitted.game_session_id, Vec::new())
            .await
            .ok_or("earned appearance catalogue")?;
        let citizen = presentation
            .catalogue
            .outfits
            .iter()
            .find(|outfit| outfit.key == "oteryn:content.outfit.citizen_outfits")
            .ok_or("Citizen appearance option")?;
        assert_ne!(
            citizen.permitted_addons & 2,
            0,
            "native appearance catalogue must decode committed NPC addon entitlement"
        );
        let remaining: i64 = sqlx::query_scalar("SELECT sum(quantity)::bigint FROM game_item_instances WHERE item_instance_id=ANY($1::uuid[])")
            .bind([71_u8,72,73,90].map(|n| sqlx::types::Uuid::from_bytes(id(n)))).fetch_one(&h.pool).await?;
        assert_eq!(remaining, 0);
        let replay_frames = if hanna_world {
            let frames = super::grizzly_npc::wire::talk_appearance(
                &mut owner,
                &mut admitted,
                npc,
                &[(9, "yes"), (10, "hi")],
            )
            .await?;
            hanna_world::assert_catalogues(&frames, true, true)?;
            hanna_world::capture(&owner, &appearance_frames, &frames, npc, &admitted).await?;
            Some(frames)
        } else {
            None
        };
        let replay_effects = if let Some(frames) = replay_frames {
            hanna_world::effects(&frames)?
        } else {
            super::grizzly_npc::wire::talk_effects(
                &mut owner,
                &mut admitted,
                npc,
                &[(9, "yes"), (10, "hi")],
            )
            .await?
        };
        assert!(
            replay_effects.is_empty(),
            "NPC grant replay emits no new source effect"
        );
        assert_eq!(
            grants(&h, grant).await?,
            1,
            "command replay and reconnect keep one grant"
        );
        eprintln!(
            "SRV7_NPC_ACQUISITION_POSTGRES_EXECUTED elapsed={:.3}s",
            started.elapsed().as_secs_f64()
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn grants(h: &Harness, grant: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_account_appearance_unlock_receipts WHERE granting_character_id=encode($1,'hex')::uuid AND unlock_key=$2")
        .bind(id(41).as_slice()).bind(grant).fetch_one(&h.pool).await?)
}

#[path = "npc_cosmetic_acquisition/hairycles.rs"]
mod hairycles;
#[test]
fn srv7_npc_hairycles_real_talk_shaman_base_prerequisite_once_and_replay() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(true).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "Hairycles NPC source thread")??;
    Ok(())
}

#[test]
fn srv7_npc_hanna_real_talk_costs_grant_refusal_replay_and_reconnect() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario_for("oteryn:npc.hanna").map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Hanna source acquisition thread")??;
    Ok(())
}

#[path = "npc_cosmetic_acquisition/ajax.rs"]
mod ajax;
#[test]
fn srv7_npc_ajax_real_source_fire_stage_timer_and_replay_are_atomic() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario_for("oteryn:npc.ajax").map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Ajax source condition thread")??;
    Ok(())
}

#[path = "npc_cosmetic_acquisition/royal.rs"]
mod royal;
#[test]
fn srv7_npc_king_actual_world_500m_base_purchase_mints_native_inbox_display_kit() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario_for("oteryn:npc.king_tibianus").map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "royal acquisition thread")??;
    Ok(())
}

#[path = "npc_cosmetic_acquisition/gnomilly.rs"]
mod gnomilly;
#[test]
fn srv7_npc_gnomilly_actual_world_unassigned_kill_report_is_honest() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario_for("oteryn:npc.gnomilly").map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "Gnomilly source thread")??;
    Ok(())
}

#[path = "npc_cosmetic_acquisition/hanna_world.rs"]
mod hanna_world;
#[test]
fn srv7_npc_hanna_actual_world_costs_wire_wardrobe_and_reconnect() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario_for_world("oteryn:npc.hanna", true).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Hanna actual World thread")??;
    Ok(())
}

#[test]
fn srv7_npc_ajax_actual_fresh_greet_after_source_hour_advances_once() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            scenario_for_trigger("oteryn:npc.ajax", false, true).map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Ajax fresh source greet thread")??;
    Ok(())
}

#[test]
fn srv7_npc_ajax_real_periodic_ten_ticks_logout_and_fresh_lease_restore_remaining_fire()
-> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            scenario_for_ajax_case("oteryn:npc.ajax", false, AjaxCase::Lifecycle)
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Ajax native Fire lifecycle thread")??;
    Ok(())
}

#[test]
fn srv7_gnomilly_real_700_deaths_reports_xp_and_earned_gnommander_outfit() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| gnomilly::full_scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "real Spike acquisition thread panicked")??;
    Ok(())
}

#[path = "npc_cosmetic_acquisition/royal_downstream.rs"]
mod royal_downstream;
#[path = "npc_cosmetic_acquisition/royal_full_purchase.rs"]
mod royal_full_purchase;
#[path = "npc_cosmetic_acquisition/royal_peer.rs"]
mod royal_peer;
#[test]
fn srv7_npc_king_one_billion_actual_inbox_house_golden_display_pipeline() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| royal::full_scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Royal coupled acquisition thread")??;
    Ok(())
}

#[path = "npc_cosmetic_acquisition/sandra.rs"]
mod sandra;
#[test]
fn srv7_actual_sandra_vial_deposit_mints_ticket_atomically() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario_for("oteryn:npc.sandra").map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "actual source Sandra thread panicked")??;
    Ok(())
}

#[path = "npc_cosmetic_acquisition/primal.rs"]
mod primal;
#[path = "npc_cosmetic_acquisition/vescu.rs"]
mod vescu;
#[test]
fn srv7_npc_vescu_genuine_fresh_greet_requires_current_drunk_and_rejects_expired() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario_for("oteryn:npc.vescu").map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Vescu native prerequisite thread panicked")??;
    Ok(())
}
#[test]
fn srv7_npc_gnomadness_native_hazard_selection_once_prize_and_current_actor_cache() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario_for("oteryn:npc.gnomadness").map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Primal native source thread panicked")??;
    Ok(())
}
#[test]
fn srv7_npc_gnomadness_native_below_prize_first_digit_run_selects_without_rewards() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            scenario_for_primal_case("oteryn:npc.gnomadness", false, AjaxCase::Fire, true)
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Primal native normal selection thread panicked")??;
    Ok(())
}

#[path = "npc_cosmetic_acquisition/navigator.rs"]
mod navigator;

#[path = "npc_cosmetic_acquisition/irmana.rs"]
mod irmana;
#[test]
fn srv7_actual_irmana_source_order_and_cash_only_paid_pickup() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario_for("oteryn:npc.irmana").map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "actual Irmana source thread")??;
    Ok(())
}

#[test]
fn srv7_npc_king_actual_royal_kit_look_retains_source_description() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| royal::description_scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Royal native Look acquisition thread")??;
    Ok(())
}

#[path = "npc_cosmetic_acquisition/simon_lifetime.rs"]
mod simon_lifetime;
#[test]
fn srv7_actual_hugo_lifetime_and_simon_compound_paid_source() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario_for("oteryn:npc.hugo").map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "actual Hugo/Simon lifetime thread")??;
    Ok(())
}

#[path = "npc_cosmetic_acquisition/registered_greet.rs"]
mod registered_greet;

#[test]
fn srv7_bron_default_greet_and_genuine_barbarian_start() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario_for("oteryn:npc.bron").map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "registered greet thread")??;
    Ok(())
}

#[test]
fn srv7_warwolf_fresh_unqualified_greet_refuses() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario_for("oteryn:npc.a_majestic_warwolf").map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "registered greet thread")??;
    Ok(())
}

#[test]
fn srv7_bozo_creation_female_registered_greet() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario_for("oteryn:npc.bozo").map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "registered greet thread")??;
    Ok(())
}
