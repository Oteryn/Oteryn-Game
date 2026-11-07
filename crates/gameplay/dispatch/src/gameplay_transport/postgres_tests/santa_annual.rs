//! Annual Santa: actual native NPC Talk and complete present inventory, no seeded entitlement.
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
fn srv7_santa_real_native_hi_present_delivers_one_current_source_bundle() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("srv7-santa-present-source", scenario)
}

fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let started = std::time::Instant::now();
        let h = Harness::create_appearance_with_admission_seed(
            admin,
            "srv7_santa_annual",
            false,
            false,
        )
        .await?;
        prepare_current_session(&h).await?;
        let seed = [("oteryn:item.tibia.i3031", 1)];
        super::item_look::seed_main_backpack(&h, &seed).await?;
        // Test-only existing authoritative clock, no quota/reward/quest rows are seeded.
        sqlx::query("CREATE OR REPLACE FUNCTION game_daily_effect_now() RETURNS TIMESTAMPTZ LANGUAGE sql VOLATILE SET search_path=pg_catalog AS $$ SELECT TIMESTAMPTZ '2026-12-12 06:00:00+00' $$").execute(&h.pool).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world_id = fence(1)?.runtime_scope.world_id();
        let (controller, pin) = super::raid_live::hare_production_activation::activate(
            world_id,
            "OTERYN_SRV7_MESSENGER_CLIENT_ARTIFACT",
        )?;
        let active = controller.active().ok_or("active generation")?;
        let bundle = super::grizzly_npc::wire::world(world_id)?;
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
                    x: 2,
                    y: 0,
                    floor: 7,
                },
            )
            .ok_or("bundle room")?;
        let channel_pin = crate::foundation::ChannelContentPin::from_activation(
            world_id,
            1,
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            room.frame_binding().digest(),
            room.map_revision_digest(),
            (2, 0, 7),
        );
        // The in-transaction custody owner independently reads this real activation.
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4) ON CONFLICT DO NOTHING")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(channel_pin.server_artifact_digest().as_slice())
            .bind(channel_pin.client_artifact_digest().as_slice())
            .bind(channel_pin.frame_binding_digest().as_slice()).execute(&h.pool).await?;
        let deps = Dependencies::with_content(
            channel_pin,
            room,
            active
                .native_gameplay()
                .ok_or("native")?
                .spell_book()
                .clone(),
        )?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = &cells;
        let actor = deps.actor;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
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
        let model = NpcDataCatalogue::load(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
            std::ffi::OsStr::new("world"),
            npc_catalogue_preproduction_limits(),
        )
        .map_err(debug)?
        .service_model_for_project()
        .map_err(debug)?;
        let book = NpcTalkBook::from_model(&model);
        assert!(book.get("oteryn:npc.santa_claus").is_some());
        owner.states.chat.install_npc_talk(Arc::new(book));
        owner
            .states
            .chat
            .admit(admitted.game_session_id, "Citizen", owner.owner_now());
        owner.states.chat.set_npc_service(admitted.game_session_id);
        let npc = {
            let mut runtime = owner.runtime.lock().await;
            runtime
                .place_npcs(
                    &NpcPlacementTable {
                        npcs: vec![NpcPlacements {
                            key: "oteryn:npc.santa_claus".into(),
                            placements: vec![NpcPlacement {
                                x: 3,
                                y: 0,
                                floor: -7,
                                direction: MovementFacing::West,
                            }],
                        }],
                    },
                    |_| true,
                    true,
                )
                .map_err(debug)?;
            let placed = runtime
                .visible_entities()
                .npcs
                .into_iter()
                .next()
                .ok_or("current Santa actor")?;
            oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
                identity: placed.actor.placement_identity(),
                generation: placed.generation,
            }
        };
        let offered = crate::gameplay_transport::appearance_change::offer::offered(
            owner.offered_capabilities(),
            owner.appearance_available(),
        );
        admitted.continuity.selected_capabilities =
            crate::gameplay_transport::capabilities::SelectedCapabilities::select(
                &offered,
                &[3, 4, 6, 15, 18, 25],
            )
            .ok_or("actual Santa NPC and appearance capabilities")?;
        let before: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE lifecycle=1")
                .fetch_one(&h.pool)
                .await?;
        super::grizzly_npc::wire::talk(
            &mut owner,
            &mut admitted,
            npc,
            &[(1, "hi"), (2, "present")],
        )
        .await?;
        // This is the first new source assertion. No annual backend API or annual
        // table is read: a valid currently placed Santa must actually hand over a gift.
        let bags: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i6510' AND lifecycle=1")
            .fetch_one(&h.pool).await?;
        assert_eq!(
            bags, 1,
            "actual native Santa present must deliver exactly one source6510 container"
        );
        let actual: serde_json::Value=sqlx::query_scalar("SELECT jsonb_agg(jsonb_build_object('source_id',substring(i.definition_production_key FROM 20)::integer,'count',i.quantity) ORDER BY ce.placement_ordinal) FROM game_item_instances bag JOIN game_item_container_entries ce ON ce.parent_item_instance_id=bag.item_instance_id JOIN game_item_instances i ON i.item_instance_id=ce.item_instance_id WHERE bag.definition_production_key='oteryn:item.tibia.i6510' AND bag.lifecycle=1")
            .fetch_one(&h.pool).await?;
        let source: serde_json::Value = serde_json::from_str(SOURCE_POOLS)?;
        assert!(
            source["regular"]
                .as_array()
                .ok_or("regular source bundles")?
                .iter()
                .chain(source["lucky"].as_array().ok_or("lucky source bundles")?)
                .any(|bundle| bundle == &actual),
            "real native inventory must match one complete current source bundle; this test asserts no unapproved probability"
        );
        let after: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE lifecycle=1")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(
            after - before,
            1 + i64::try_from(actual.as_array().ok_or("gift children")?.len())?,
            "one source container and exactly its children, without extra items"
        );
        eprintln!(
            "SRV7_SANTA_NATIVE_PRESENT_SOURCE_EXECUTED elapsed={:.3}s",
            started.elapsed().as_secs_f64()
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

const SOURCE_POOLS: &str = r###"{
  "regular": [
    [
      {
        "source_id": 3599,
        "count": 10
      },
      {
        "source_id": 2992,
        "count": 10
      }
    ],
    [
      {
        "source_id": 3599,
        "count": 10
      },
      {
        "source_id": 6506,
        "count": 1
      }
    ],
    [
      {
        "source_id": 3599,
        "count": 10
      },
      {
        "source_id": 6507,
        "count": 1
      }
    ],
    [
      {
        "source_id": 3599,
        "count": 10
      },
      {
        "source_id": 6508,
        "count": 1
      }
    ],
    [
      {
        "source_id": 3599,
        "count": 10
      },
      {
        "source_id": 2995,
        "count": 1
      }
    ],
    [
      {
        "source_id": 3051,
        "count": 1
      },
      {
        "source_id": 3097,
        "count": 1
      },
      {
        "source_id": 3098,
        "count": 1
      }
    ],
    [
      {
        "source_id": 3039,
        "count": 1
      }
    ],
    [
      {
        "source_id": 10310,
        "count": 1
      }
    ],
    [
      {
        "source_id": 49271,
        "count": 1
      },
      {
        "source_id": 7443,
        "count": 1
      },
      {
        "source_id": 7440,
        "count": 1
      },
      {
        "source_id": 7439,
        "count": 1
      }
    ],
    [
      {
        "source_id": 6567,
        "count": 1
      }
    ],
    [
      {
        "source_id": 2991,
        "count": 1
      }
    ],
    [
      {
        "source_id": 3036,
        "count": 1
      }
    ],
    [
      {
        "source_id": 9040,
        "count": 1
      }
    ],
    [
      {
        "source_id": 6566,
        "count": 1
      }
    ],
    [
      {
        "source_id": 9604,
        "count": 1
      }
    ]
  ],
  "lucky": [
    [
      {
        "source_id": 5080,
        "count": 1
      }
    ],
    [
      {
        "source_id": 2993,
        "count": 1
      }
    ],
    [
      {
        "source_id": 10338,
        "count": 1
      }
    ],
    [
      {
        "source_id": 3057,
        "count": 1
      }
    ],
    [
      {
        "source_id": 3553,
        "count": 1
      }
    ],
    [
      {
        "source_id": 5919,
        "count": 1
      }
    ],
    [
      {
        "source_id": 3570,
        "count": 1
      }
    ],
    [
      {
        "source_id": 9058,
        "count": 1
      }
    ],
    [
      {
        "source_id": 10339,
        "count": 1
      }
    ],
    [
      {
        "source_id": 3001,
        "count": 1
      }
    ],
    [
      {
        "source_id": 5809,
        "count": 1
      }
    ]
  ],
  "source": "https://tibiawiki.dev/api/npcs/Santa_Claus",
  "probabilities": null
}
"###;
