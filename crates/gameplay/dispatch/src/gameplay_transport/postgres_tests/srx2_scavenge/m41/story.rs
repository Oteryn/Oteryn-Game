//! The Stag beak boundary: twenty self-skinned successes, no purchased/failed/replayed credits.
use super::quest_fixture as q;
use super::*;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::quest_state::QuestTransitionOutcome;
use crate::durability::stag_beaks::{BEAKS, STAGE};
use crate::foundation::ExactActorRef;
use std::collections::BTreeMap;

pub(super) async fn exercise(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    native: &crate::content::native_gameplay::NativeGameplayState,
    fence: CurrentCharacterItemFence,
    actor: ExactActorRef,
) -> TestResult {
    let catalogue = q::catalogue()?;
    // Only Hartford's owned boundary is exercised; earlier phases are prerequisites.
    q::ready(owner, catalogue.clone(), 1, 8999, q::STAG_START).await?;
    eprintln!("M41_PHASE physical-skin-before-mission");
    assert_eq!(
        skinning::quantity(h, "oteryn:item.tibia.i52632").await?,
        20,
        "purchased fixture beaks"
    );
    skinning::skin(owner, h, native, fence, actor, "night_harpy", true, 0).await?;
    assert_eq!(
        skinning::credits(h).await?,
        0,
        "pre-mission successes do not count"
    );
    eprintln!("M41_PHASE accept-Hartford-mission");
    let accepted = q::npc(
        owner,
        native,
        catalogue.clone(),
        2,
        9001,
        q::HARTFORD,
        "yes",
        1,
        q::ACCEPT,
    )
    .await?;
    assert!(
        matches!(accepted.transition, QuestTransitionOutcome::Committed(_)),
        "{accepted:?}"
    );
    let bought = q::npc(
        owner,
        native,
        catalogue.clone(),
        3,
        9002,
        q::HARTFORD,
        "yes",
        2,
        q::REPORT,
    )
    .await?;
    assert!(
        !matches!(
            bought.transition,
            QuestTransitionOutcome::Committed(_) | QuestTransitionOutcome::AlreadyCommitted(_)
        ),
        "purchased beaks are not provenance"
    );
    skinning::skin(owner, h, native, fence, actor, "minotaur", true, 1).await?;
    skinning::skin(owner, h, native, fence, actor, "night_harpy", false, 2).await?;
    assert_eq!(
        skinning::credits(h).await?,
        0,
        "wrong Source race and failed roll give no credit"
    );
    let mut first = None;
    for index in 0..20_u32 {
        let species = if index % 2 == 0 {
            "night_harpy"
        } else {
            "day_night_harpy"
        };
        let command =
            skinning::skin(owner, h, native, fence, actor, species, true, index + 3).await?;
        assert_eq!(
            skinning::credits(h).await?,
            i64::from(index) + 1,
            "each distinct successful self-skinning counts once"
        );
        if index == 0 {
            first = Some(command);
            let replay = owner
                .use_cooking_conversion(actor, command, skinning_intent()?, None, None)
                .await
                .ok_or("M41 skin replay")?;
            assert_eq!(replay.disposition, UseDisposition::Committed);
            assert_eq!(
                skinning::credits(h).await?,
                1,
                "replay cannot make a second proof"
            );
        }
        if index == 18 {
            let nineteen = q::npc(
                owner,
                native,
                catalogue.clone(),
                3,
                9019,
                q::HARTFORD,
                "yes",
                2,
                q::REPORT,
            )
            .await?;
            assert!(
                !matches!(
                    nineteen.transition,
                    QuestTransitionOutcome::Committed(_)
                        | QuestTransitionOutcome::AlreadyCommitted(_)
                ),
                "nineteen genuine credits are insufficient"
            );
        }
    }
    let same:bool=sqlx::query_scalar("SELECT bool_and(p.created_xact_id=c.created_xact_id AND p.created_xact_id=s.created_xact_id) FROM game_stag_beak_proofs p JOIN game_item_craft_records c ON c.transaction_id=p.craft_transaction_id JOIN game_corpse_skinning_records s ON s.transaction_id=c.transaction_id")
        .fetch_one(&h.pool).await?;
    assert!(
        same,
        "quest proof and both physical receipts share the skinning transaction"
    );
    let proof = owner
        .root
        .read_stag_beak_preview(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    let mut tracks = BTreeMap::from([(STAGE.into(), 6), (BEAKS.into(), 999)]);
    proof
        .overlay(fence.character_id, &mut tracks)
        .map_err(debug)?;
    assert_eq!(
        tracks.get(BEAKS),
        Some(&20),
        "sealed provenance replaces unproven legacy counters"
    );
    let beaks = skinning::quantity(h, "oteryn:item.tibia.i52632").await?;
    let report = q::npc(
        owner,
        native,
        catalogue.clone(),
        3,
        9020,
        q::HARTFORD,
        "yes",
        2,
        q::REPORT,
    )
    .await?;
    assert!(
        matches!(report.transition, QuestTransitionOutcome::Committed(_)),
        "{report:?}"
    );
    assert_eq!(
        skinning::quantity(h, "oteryn:item.tibia.i52632").await?,
        beaks,
        "Hartford says keep the beaks"
    );
    let replay = q::npc(
        owner,
        native,
        catalogue.clone(),
        3,
        9020,
        q::HARTFORD,
        "yes",
        2,
        q::REPORT,
    )
    .await?;
    assert!(matches!(
        replay.transition,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    let replay = owner
        .use_cooking_conversion(
            actor,
            first.ok_or("first skin")?,
            skinning_intent()?,
            None,
            None,
        )
        .await
        .ok_or("late skin replay")?;
    assert_eq!(replay.disposition, UseDisposition::Committed);
    skinning::skin(owner, h, native, fence, actor, "night_harpy", true, 23).await?;
    assert_eq!(
        skinning::credits(h).await?,
        20,
        "post-mission skinning is not new quest progress"
    );
    super::finales::exercise(owner, h, native, catalogue).await
}
