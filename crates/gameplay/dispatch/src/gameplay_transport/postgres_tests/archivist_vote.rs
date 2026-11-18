//! Crystal 201189f8 data-global/npc/marvin.lua:57-85.
use std::sync::Arc;

use super::classic_quests::{install, talk};
use super::fixture::{Harness, debug, id, run};
use super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session,
};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::gameplay_transport::connection::FirstEntryOutcome;
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, QuestTrack, authored_bindings::Bindings};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

const DARK_TRAILS: &str = "oteryn:quest.dark_trails_quest";
const ORAMOND: &str = "oteryn:quest.oramond_quest";
const VOTES: &str = "oteryn:quest-progress/quest/u10_50/oramond_quest/voting_points";
const ARCHIVES: &str = "oteryn:quest-progress/quest/u10_50/dark_trails/mission08";

#[test]
fn archivist_confirmation_debits_oramond_and_credits_dark_trails_in_one_revision() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            run(async |_| {
                let h = Harness::create_with_admission_seed(admin, "archivist_vote", false, false)
                    .await?;
                let result = Box::pin(exercise(&h)).await;
                let cleanup = h.cleanup().await;
                result?;
                cleanup
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "archivist vote case panicked")??;
    Ok(())
}

fn catalogue() -> TestResult<QuestStateCatalogue> {
    let base = QuestStateCatalogue::new(
        "content-1",
        vec![QuestTrack {
            key: VOTES.into(),
            quest: ORAMOND.into(),
            initial: 2,
            min: -1,
            max: i64::MAX,
        }],
        vec![],
    )
    .map_err(debug)?;
    let source =
        "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/marvin.lua:77-85";
    let binding = Bindings::parse(serde_json::json!({
        "schema":1,"quests":[{
            "quest":DARK_TRAILS,
            "recipe_sha256":<sha2::Sha256 as sha2::Digest>::digest(source).iter().map(|byte| format!("{byte:02x}")).collect::<String>(),
            "tracks":[{"key":ARCHIVES,"initial":-1,"minimum":-1,"maximum":1}],
            "stages":[{"key":"s1","completes":false,"trigger":{"kind":"npc_dialogue","callback":{
                "key":"oteryn:quest-reward/fixture/archive-vote","quest":DARK_TRAILS,"npc":"oteryn:npc.marvin",
                "topic":0,"next_topic":0,"keywords":["yes"],
                "storage":[{"track":ARCHIVES,"minimum":-1,"maximum":0,"set":1}],
                "participants":[{
                    "quest":ORAMOND,"storage":[],"state_ops":[{
                        "track":VOTES,
                        "from":{"kind":"between","minimum":1,"maximum":i64::MAX},
                        "effect":{"kind":"add","value":-1}
                    }]
                }],
                "take":[],"reply":["Vote recorded."],"sources":[source]
            }}}],
            "sources":[source]
        }]
    }).to_string().as_bytes())?;
    Ok(binding.attach(&base)?)
}

async fn exercise(h: &Harness) -> TestResult {
    with_npc(h, Arc::new(catalogue()?), "oteryn:npc.marvin", async |owner, admitted, marvin| {
    talk(owner, admitted, marvin, 1, "hi").await?;
    let reply = talk(owner, admitted, marvin, 2, "yes").await?;
    assert_eq!(reply, ["Vote recorded."]);
    let read = || async {
        sqlx::query_as::<_, (String, i64, String)>(
            "SELECT track_key, value, committed_character_revision::text FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key IN ($2,$3) ORDER BY track_key",
        ).bind(id(41).as_slice()).bind(ARCHIVES).bind(VOTES).fetch_all(&h.pool).await
    };
    let rows = read().await?;
    assert_eq!(rows.len(), 2);
    assert_eq!((&*rows[0].0, rows[0].1), (ARCHIVES, 1));
    assert_eq!((&*rows[1].0, rows[1].1), (VOTES, 1));
    assert_eq!(rows[0].2, rows[1].2);
    let _ = talk(owner, admitted, marvin, 2, "yes").await?;
    assert_eq!(read().await?, rows);
    Ok(())
    }).await
}

pub(super) async fn with_npc(
    h: &Harness,
    catalogue: Arc<QuestStateCatalogue>,
    npc: &str,
    exercise: impl AsyncFnOnce(
        &mut ComposedFreshAdmission<'_, '_, '_>,
        &AdmittedSession,
        EntityRef,
    ) -> TestResult,
) -> TestResult {
    prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world_id = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, activated) =
        super::death_ml_loss_composed::production_generation_with_pin(world_id, "content-1")?;
    let generation = controller.active().ok_or("active content")?;
    let room = activated.qualified_room();
    let start = room.entry_start();
    let digest = generation.identity().server_artifact_digest();
    let client_digest = generation.identity().client_artifact_digest();
    let frame = activated.frame_binding().digest();
    let pin = ChannelContentPin::from_activation(
        world_id,
        1,
        digest,
        client_digest,
        frame,
        room.map_revision_digest(),
        (start.x, start.y, start.floor),
    );
    let deps = Dependencies::with_content(
        pin,
        room.clone(),
        crate::spell::cast::v1_spell_book().map_err(debug)?,
    )?;
    deps.runtime
        .lock()
        .await
        .initialize_first_entry_position(deps.actor)
        .map_err(debug)?;
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(generation);
    owner.quest_catalogue = Some(catalogue);
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;

    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(digest.as_slice()).bind(client_digest.as_slice())
        .bind(frame.as_slice()).execute(&h.pool).await?;
    let [npc] = install(&owner, &admitted, [npc]).await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    admitted.item_fence = Some(fence.item_fence());
    let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
    h.root
        .initialize_admission_equipment(
            &authority,
            &h.node,
            &fence.item_fence(),
            digest,
            fresh.operation(),
        )
        .await
        .map_err(debug)?;
    let native = generation.native_gameplay().ok_or("native gameplay")?;
    let template = crate::durability::starter_grant::StarterTemplate {
        definition: crate::durability::item_mint::TypedDefinitionRef {
            family: "Item".into(),
            production_key: "oteryn:item.tibia.i2854".into(),
            revision_ref: "definition-r1".into(),
        },
        ..crate::durability::starter_grant::StarterTemplate::main()
    };
    h.root
        .grant_starter_items(
            &authority,
            &h.node,
            &fence.item_fence(),
            &crate::gameplay_transport::item_ref_admission::NativeItemWeights(native),
            &template,
        )
        .await
        .map_err(debug)?;
    deps.initialize(&owner).await;
    owner.admit_progression_session(&mut admitted).await;
    owner.admit_quest_session(&admitted).await;
    Box::pin(exercise(&mut owner, &admitted, npc)).await
}
