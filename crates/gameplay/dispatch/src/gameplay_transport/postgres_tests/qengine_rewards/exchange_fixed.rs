//! Crystal 9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:
//! data-global/npc/a_sweaty_cyclops.lua:64-246 sha256:bc694bf7d62f6a0daae1eda9f6542c6bd79fb749805fe58c23275189ce1e5d0a;
//! data-global/npc/marina.lua:55-123 sha256:8574dd5721f258bef8d649657ed428537e3e351e540a990fe72bcc21c89b8682.
//! Molten-gold lifetime: imports/canary/items-xml/items.xml:30991-30994
//! sha256:1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2.
//! Material fixtures provide inputs only; no quest progress or rewards are seeded.
use super::*;

async fn quantity(h: &Harness, item: u32) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1").bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?)
}
async fn progress(h: &Harness, track: &str) -> TestResult<Option<i64>> {
    Ok(
        sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
            .bind(format!(
                "oteryn:quest-progress/quest/u7_8/friends_and_traders/{track}"
            ))
            .fetch_optional(&h.pool)
            .await?,
    )
}
async fn material(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    item: u32,
    count: u16,
    ordinal: u64,
) -> TestResult {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    let at = owner
        .runtime
        .lock()
        .await
        .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
        .map_err(debug)?
        .position();
    super::super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &oteryn_gameplay_quests::quest::world_callbacks::Offering {
            position: Position {
                x: u16::try_from(at.x)?,
                y: u16::try_from(at.y)?,
                floor: -i8::try_from(at.floor)?,
            },
            item: format!("oteryn:item.tibia.i{item}"),
            count,
        },
        ordinal,
        true,
    )
    .await
}
fn recipe<'a>(callbacks: &'a [RewardCallback], name: &str) -> TestResult<&'a RewardCallback> {
    callbacks
        .iter()
        .find(|c| c.key == format!("oteryn:quest-reward/exchange-{name}"))
        .ok_or_else(|| format!("source exchange binding absent: {name}").into())
}
async fn confirm(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: EntityRef,
    callback: &RewardCallback,
    expected: NpcIntentDisposition,
) -> TestResult<u64> {
    let (result, _, command) = wire_grant(owner, admitted, npc, callback).await?;
    assert_eq!(result, expected, "{}", callback.key);
    Ok(command)
}
async fn replay(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: EntityRef,
    command: u64,
) -> TestResult {
    let payload = oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent(&NpcTalkIntent {
        npc_actor: npc,
        text: "yes".into(),
    })
    .map_err(debug)?;
    let bytes =
        super::super::quest_world_callbacks::wire(owner, admitted, command, 7, payload).await?;
    assert_eq!(
        oteryn_protocol_oteryn::npc_service::decode_npc_intent_result(&bytes).map_err(debug)?,
        NpcIntentDisposition::Ok
    );
    Ok(())
}
pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
        .fetch_one(&h.pool)
        .await?;
    println!("POSTGRES_CASE_STARTED fixed-exchanges server_version={version}");
    assert_eq!(progress(h, "the_sweaty_cyclops").await?, None);
    assert_eq!(progress(h, "the_mermaid_marina").await?, None);
    let callbacks = oteryn_gameplay_quests::quest::reward_callbacks::load()?;
    let start = recipe(&callbacks, "cyclops-start")?;
    let [cyclops, marina] = super::super::classic_quests::install(
        owner,
        admitted,
        ["oteryn:npc.a_sweaty_cyclops", "oteryn:npc.marina"],
    )
    .await?;
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([3, 6]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("capabilities")?;
    confirm(owner, admitted, cyclops, start, NpcIntentDisposition::Ok).await?;
    assert_eq!(progress(h, "the_sweaty_cyclops").await?, Some(1));
    let steel = recipe(&callbacks, "cyclops-royal-steel")?;
    confirm(
        owner,
        admitted,
        cyclops,
        steel,
        NpcIntentDisposition::Rejected,
    )
    .await?;
    assert_eq!(quantity(h, 5887).await?, 0);
    for n in 0..2 {
        material(owner, admitted, 3560, 1, 2100 + n).await?;
    }
    let favour = recipe(&callbacks, "cyclops-skirts")?;
    confirm(
        owner,
        admitted,
        cyclops,
        favour,
        NpcIntentDisposition::Rejected,
    )
    .await?;
    assert_eq!(quantity(h, 3560).await?, 2);
    assert_eq!(progress(h, "the_sweaty_cyclops").await?, Some(1));
    material(owner, admitted, 3560, 1, 2102).await?;
    confirm(owner, admitted, cyclops, favour, NpcIntentDisposition::Ok).await?;
    assert_eq!(quantity(h, 3560).await?, 0);
    assert_eq!(progress(h, "the_sweaty_cyclops").await?, Some(2));
    for (index, (name, input, output)) in [
        ("royal-steel", 3381, 5887),
        ("draconian-steel", 3416, 5889),
        ("hell-steel", 3356, 5888),
        ("crude-iron", 3281, 5892),
        ("gold-melt", 9058, 12804),
        ("gear-wheel", 5880, 8775),
    ]
    .into_iter()
    .enumerate()
    {
        let callback = recipe(&callbacks, &format!("cyclops-{name}"))?;
        confirm(
            owner,
            admitted,
            cyclops,
            callback,
            NpcIntentDisposition::Rejected,
        )
        .await?;
        assert_eq!(quantity(h, output).await?, 0);
        for purchase in 0..2 {
            material(
                owner,
                admitted,
                input,
                1,
                2200 + u64::try_from(index)? * 10 + purchase,
            )
            .await?;
            let command =
                confirm(owner, admitted, cyclops, callback, NpcIntentDisposition::Ok).await?;
            assert_eq!(quantity(h, input).await?, 0);
            assert_eq!(quantity(h, output).await?, i64::try_from(purchase)? + 1);
            if name == "gold-melt" {
                let scheduled:(i64,i64)=sqlx::query_as("SELECT count(*),count(*) FILTER(WHERE p.duration_millis=36000000 AND p.expires_at_unix_ms=r.occurred_at+36000000 AND p.target_family IS NULL) FROM game_spell_item_temporal_schedules p JOIN game_reward_claim_mint_receipts r ON r.transaction_id=p.source_transaction_id WHERE p.definition_production_key='oteryn:item.tibia.i12804'").fetch_one(&h.pool).await?;
                let expected = i64::try_from(purchase)? + 1;
                assert_eq!(
                    scheduled,
                    (expected, expected),
                    "each actual molten-gold reward retains its full source deadline"
                );
            }
            replay(owner, admitted, cyclops, command).await?;
            assert_eq!(quantity(h, output).await?, i64::try_from(purchase)? + 1);
        }
        assert_eq!(progress(h, "the_sweaty_cyclops").await?, Some(2));
    }
    confirm(
        owner,
        admitted,
        marina,
        recipe(&callbacks, "marina-start")?,
        NpcIntentDisposition::Ok,
    )
    .await?;
    assert_eq!(progress(h, "the_mermaid_marina").await?, Some(1));
    let yarn = recipe(&callbacks, "marina-yarn")?;
    confirm(
        owner,
        admitted,
        marina,
        yarn,
        NpcIntentDisposition::Rejected,
    )
    .await?;
    material(owner, admitted, 5902, 49, 2300).await?;
    let honey = recipe(&callbacks, "marina-honey")?;
    confirm(
        owner,
        admitted,
        marina,
        honey,
        NpcIntentDisposition::Rejected,
    )
    .await?;
    assert_eq!(quantity(h, 5902).await?, 49);
    assert_eq!(progress(h, "the_mermaid_marina").await?, Some(1));
    material(owner, admitted, 5902, 1, 2301).await?;
    confirm(owner, admitted, marina, honey, NpcIntentDisposition::Ok).await?;
    assert_eq!(quantity(h, 5902).await?, 0);
    assert_eq!(progress(h, "the_mermaid_marina").await?, Some(2));
    material(owner, admitted, 5879, 9, 2302).await?;
    confirm(
        owner,
        admitted,
        marina,
        yarn,
        NpcIntentDisposition::Rejected,
    )
    .await?;
    assert_eq!(quantity(h, 5879).await?, 9);
    assert_eq!(quantity(h, 5886).await?, 0);
    material(owner, admitted, 5879, 11, 2303).await?;
    for purchase in 1..=2 {
        let command = confirm(owner, admitted, marina, yarn, NpcIntentDisposition::Ok).await?;
        assert_eq!(quantity(h, 5879).await?, 20 - purchase * 10);
        assert_eq!(quantity(h, 5886).await?, purchase);
        replay(owner, admitted, marina, command).await?;
        assert_eq!(quantity(h, 5886).await?, purchase);
    }
    assert_eq!(progress(h, "the_mermaid_marina").await?, Some(2));
    assert_eq!(progress(h, "default_start").await?, Some(1));
    // Source Cyclops184–213 consumes ALL carried orbs and rolls separately per orb.
    let soul = recipe(&callbacks, "cyclops-soul-orbs")?;
    confirm(
        owner,
        admitted,
        cyclops,
        soul,
        NpcIntentDisposition::Rejected,
    )
    .await?;
    assert_eq!(quantity(h, 6528).await?, 0);
    sqlx::query("CREATE OR REPLACE FUNCTION game_quest_reward_rolls(n integer) RETURNS integer[] LANGUAGE sql VOLATILE AS $$ SELECT array_fill(100,ARRAY[n]) $$").execute(&h.pool).await?;
    material(owner, admitted, 5944, 1, 2400).await?;
    let command = confirm(owner, admitted, cyclops, soul, NpcIntentDisposition::Ok).await?;
    assert_eq!(quantity(h, 5944).await?, 0);
    assert_eq!(quantity(h, 6528).await?, 100);
    replay(owner, admitted, cyclops, command).await?;
    assert_eq!(quantity(h, 6528).await?, 100);
    // Isolated server RNG fixture exercises both source outcomes without seeded progress.
    sqlx::query("CREATE OR REPLACE FUNCTION game_quest_reward_rolls(n integer) RETURNS integer[] LANGUAGE sql VOLATILE AS $$ SELECT CASE WHEN n=2 THEN ARRAY[1,100] ELSE array_fill(100,ARRAY[n]) END $$").execute(&h.pool).await?;
    material(owner, admitted, 5944, 2, 2401).await?;
    let command = confirm(owner, admitted, cyclops, soul, NpcIntentDisposition::Ok).await?;
    assert_eq!(quantity(h, 5944).await?, 0);
    assert_eq!(quantity(h, 6528).await?, 400);
    let stacks: Vec<i64> = sqlx::query_scalar("SELECT quantity::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i6528' ORDER BY item_instance_id").fetch_all(&h.pool).await?;
    assert_eq!(stacks, vec![100; 4]);
    replay(owner, admitted, cyclops, command).await?;
    assert_eq!(quantity(h, 6528).await?, 400);
    sqlx::query("CREATE OR REPLACE FUNCTION game_quest_reward_rolls(n integer) RETURNS integer[] LANGUAGE sql VOLATILE AS $$ SELECT array_fill(1,ARRAY[n]) $$").execute(&h.pool).await?;
    material(owner, admitted, 5944, 3, 2402).await?;
    let before: (i64,i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_quest_reward_sources),(SELECT count(*) FROM game_item_audit_outbox)").fetch_one(&h.pool).await?;
    confirm(
        owner,
        admitted,
        cyclops,
        soul,
        NpcIntentDisposition::Rejected,
    )
    .await?;
    assert_eq!(quantity(h, 5944).await?, 3);
    assert_eq!(quantity(h, 6528).await?, 400);
    assert_eq!(sqlx::query_as::<_,(i64,i64)>("SELECT (SELECT count(*) FROM game_quest_reward_sources),(SELECT count(*) FROM game_item_audit_outbox)").fetch_one(&h.pool).await?,before);
    assert_eq!(progress(h, "the_sweaty_cyclops").await?, Some(2));
    Ok(())
}
