//! Crystal prison StepIn through the fenced world callback and item mint owners.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::content::native_gameplay::NativeGameplayState;
use crate::map::overlay::TilePos;
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue,
    authored_bindings::{Bindings, BoundQuest, Stage, Track, Trigger as BindingTrigger},
    reward_callbacks::ItemReward,
    world_callbacks::{NativeSelector, Position, Selector, Storage, Trigger},
};
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result,
    encode_step_intent,
};
use std::sync::Arc;

pub(super) const NAME: &str = "qengine-rewards-wrathm1-prison-release";
pub(super) const SOURCE: Position = Position {
    x: 33362,
    y: 31202,
    floor: -8,
};
pub(super) const START: Position = Position {
    y: SOURCE.y + 1,
    ..SOURCE
};
const OUTSIDE: Position = Position {
    x: 33359,
    y: 31183,
    floor: -8,
};
const QUEST: &str = "oteryn:quest.wrath_of_the_emperor_quest";
const RELEASE: &str = "oteryn:quest-progress/quest/u8_6/wrath_of_the_emperor/prison_release_status";
const CAUGHT: &str = "oteryn:quest-progress/quest/u8_6/wrath_of_the_emperor/guardcaught_you";
const QUESTLINE: &str = "oteryn:quest-progress/quest/u8_6/wrath_of_the_emperor/questline";
const SCRIPT: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/wrath_of_the_emperor/movements_prison.lua:3-26";
const CRATE: &str = "oteryn:item.tibia.i11328";

#[test]
fn wrathm1_prison_release_step_returns_a_crate_on_each_authorized_escape() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, SOURCE, false)
    })
}

pub(super) fn world(native: &NativeGameplayState) -> TestResult<crate::map::boot::BundleWorld> {
    super::questpc8_native_slice::world(
        native,
        &[
            (33362, 31202, 33362, 31203, -8),
            (33359, 31183, 33359, 31183, -8),
        ],
        &[],
        TilePos {
            x: START.x,
            y: START.y,
            floor: START.floor,
        },
    )
}

fn catalogue(revision: &str) -> TestResult<QuestStateCatalogue> {
    let mut callback = super::authored_world_rewards::placeholder(SOURCE)?;
    callback.key = "oteryn:quest-callback/test/wrathm1-prison-release".into();
    callback.quest = QUEST.into();
    callback.trigger = Trigger::StepIn;
    callback.selector = Selector {
        position: None,
        position_aliases: Vec::new(),
        ordinal: None,
        unique: None,
        action: None,
        appearance: None,
        native: Some(NativeSelector {
            position: SOURCE,
            ordinal: 0,
            appearance: 351,
        }),
    };
    callback.requires_world_admission = true;
    callback.teleport = Some(OUTSIDE);
    callback.sources = vec![SCRIPT.into()];
    callback.storage = [
        (RELEASE, 1, 1, 0, false),
        (CAUGHT, -1, 1, -1, false),
        (QUESTLINE, 2, 2, 2, true),
    ]
    .into_iter()
    .map(|(track, minimum, maximum, set, preserve)| Storage {
        track: track.into(),
        minimum,
        maximum,
        set,
        preserve,
        set_if: None,
        add: None,
        increment: false,
        at_least: false,
        read_only: false,
    })
    .collect();
    callback.effects.restore_disguise = true;
    callback.effects.item_reward_per_use = true;
    callback.effects.item_reward = Some(ItemReward {
        item: CRATE.into(),
        count: 1,
        premium: false,
        contents: Vec::new(),
        key_number: None,
        backpack_outputs: Vec::new(),
        text: None,
        additional: Vec::new(),
    });
    // Model the release flag setup through a separately admitted fixture callback.
    let mut arm = callback.clone();
    arm.key = "oteryn:quest-callback/test/wrathm1-arm-prison-release".into();
    arm.selector.native = Some(NativeSelector {
        position: START,
        ordinal: 0,
        appearance: 353,
    });
    arm.effects = Default::default();
    arm.teleport = None;
    arm.storage[0].minimum = 0;
    arm.storage[0].maximum = 0;
    arm.storage[0].set = 1;
    arm.storage[1].set = 1;
    arm.storage.truncate(2);
    let bindings = Bindings {
        schema: 1,
        quests: vec![BoundQuest {
            quest: QUEST.into(),
            recipe_sha256: "2a725710abcabd47283d130ef63b222e17a7fc156b6c891257382261d76fdb42"
                .into(),
            occurrence: None,
            admission: None,
            tracks: [
                (RELEASE, 0, 0, 1),
                (CAUGHT, -1, -1, 1),
                (QUESTLINE, 2, 0, 34),
            ]
            .into_iter()
            .map(|(key, initial, minimum, maximum)| Track {
                key: key.into(),
                initial,
                minimum,
                maximum,
            })
            .collect(),
            stages: vec![
                Stage {
                    key: "fixture-release-permission".into(),
                    completes: false,
                    trigger: BindingTrigger::StepIn {
                        callback: Box::new(arm),
                    },
                },
                Stage {
                    key: "prison-exit".into(),
                    completes: false,
                    trigger: BindingTrigger::StepIn {
                        callback: Box::new(callback),
                    },
                },
            ],
            sources: vec![SCRIPT.into()],
        }],
    };
    let base = QuestStateCatalogue::empty(revision).ok_or("quest catalogue")?;
    Ok(bindings.attach(&base)?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let revision = owner
        .quest_catalogue
        .as_ref()
        .ok_or("quest catalogue")?
        .content_revision();
    owner.quest_catalogue = Some(Arc::new(catalogue(revision)?));
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let quantity = || async {
        sqlx::query_scalar::<_, i64>("SELECT coalesce(sum(quantity), 0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1")
            .bind(CRATE)
            .fetch_one(&h.pool)
            .await
    };
    for expected in [0, 1, 2] {
        if expected > 0 {
            super::questpc8_mystery_geometry::arrive(
                owner,
                admitted,
                TilePos {
                    x: SOURCE.x,
                    y: SOURCE.y,
                    floor: SOURCE.floor,
                },
            )
            .await?;
            step(owner, admitted, StepDirection::South).await?;
        }
        step(owner, admitted, StepDirection::North).await?;
        let current = owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?;
        assert_eq!(
            current.position(),
            super::quest_world_callbacks::local(if expected == 0 { SOURCE } else { OUTSIDE })
        );
        assert_eq!(quantity().await?, expected);
        if expected > 0 {
            let values: Vec<i64> = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key IN ($1, $2) ORDER BY track_key")
                .bind(RELEASE)
                .bind(CAUGHT)
                .fetch_all(&h.pool)
                .await?;
            assert_eq!(values, [-1, 0]);
        }
    }
    Ok(())
}

async fn step(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    direction: StepDirection,
) -> TestResult {
    owner.clock_origin -= std::time::Duration::from_secs(2);
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        admitted.continuity.next_command_id,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        encode_step_intent(direction),
    )
    .await?;
    assert_eq!(
        decode_step_result(&reply).map_err(debug)?,
        StepDisposition::Moved
    );
    Ok(())
}
