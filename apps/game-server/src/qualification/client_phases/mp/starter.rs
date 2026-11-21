//! Starter profile readback in the disposable node-boot topology.
use super::*;
use bots::Bot;
use mp01_party_pvp::tab;
use oteryn_dev_client::client_ui::cyclopedia_character as panel;
use oteryn_protocol_oteryn::cyclopedia_character as character;

pub(super) async fn check_profile(bot: &mut Bot<'_, '_>, birth: Option<&str>) -> TestResult {
    let vocation = birth.unwrap_or("none");
    let starter = if birth.is_some() {
        "newhaven-starter-r1"
    } else {
        "nb-starter-1"
    };
    let vocation_id =
        character::vocation_id(vocation).ok_or("starter vocation has no protocol id")?;
    let view = if birth.is_none() {
        legacy_general_stats(bot).await?
    } else {
        tab(bot, character::CharacterTab::GeneralStats).await?
    };
    let stats = view.general.ok_or("starter GeneralStats missing")?;
    if view.vocation_id != Some(vocation_id) || (stats.level, stats.experience) != (1, 0) {
        return Err(format!(
            "starter GeneralStats expected vocation={vocation} level=1 XP=0; vocation_id={:?} level={} XP={}",
            view.vocation_id, stats.level, stats.experience
        ).into());
    }
    let mut database = sqlx::PgConnection::connect(bot.pair.url).await?;
    let (stored_birth, stored_vocation, stored_starter, level, xp): (
        Option<String>,
        String,
        String,
        i64,
        i64,
    ) = sqlx::query_as(
        "SELECT r.creation_vocation, coalesce(b.vocation, 'none'), r.starter_template_revision, \
                    p.level, p.total_experience \
               FROM game_character_roots r \
               JOIN game_character_progression_state p USING (character_id) \
               LEFT JOIN game_character_build_state b USING (character_id) \
              WHERE r.character_id = encode($1, 'hex')::uuid AND r.account_id = $2::uuid \
                AND r.world_id = encode($3, 'hex')::uuid",
    )
    .bind(bot.character.as_slice())
    .bind(&bot.pair.accounts[bot.index])
    .bind(bot.pair.world.as_slice())
    .fetch_one(&mut database)
    .await?;
    database.close().await?;
    if (
        stored_birth.as_deref(),
        stored_vocation.as_str(),
        stored_starter.as_str(),
        level,
        xp,
    ) != (birth, vocation, starter, 1, 0)
    {
        return Err(format!(
            "starter PG expected birth={birth:?} vocation={vocation} starter={starter} level=1 XP=0; birth={stored_birth:?} vocation={stored_vocation} starter={stored_starter} level={level} XP={xp}"
        ).into());
    }
    if birth.is_some() {
        evidence(&format!(
            "MP_CHECK MP01 starter-profile PASS vocation={vocation} level=1 xp=0 pg=matched"
        ));
    }
    Ok(())
}

async fn legacy_general_stats(bot: &mut Bot<'_, '_>) -> TestResult<panel::CyclopediaCharacterPanel> {
    let client = bot.client()?;
    let mut view = panel::CyclopediaCharacterPanel {
        selected: true,
        limit: 50,
        extensions: panel::TabExtensions {
            vocation: general_stats::selected(client),
            death_details: client.selected_capabilities().contains(&488),
        },
        ..Default::default()
    };
    let command = panel::request(
        &mut view,
        panel::Intent::SelectTab(character::CharacterTab::GeneralStats),
    )
    .ok_or("D150 GeneralStats request absent")?;
    let session = client.session_mut();
    let mut reply = session
        .gated_command(command.capability, command.kind, &command.payload)
        .await?;
    if reply.status == CommandStatus::Rejected && reply.payload.is_empty() {
        reply = session
            .gated_command(command.capability, command.kind, &command.payload)
            .await?;
    }
    if reply.status != CommandStatus::Accepted || !panel::apply_result(&mut view, &reply) {
        return Err(format!("D150 GeneralStats refused or malformed: {:?}", reply.status).into());
    }
    Ok(view)
}

pub(in crate::qualification) async fn legacy_d150(clients: &SeamClients<'_>) -> TestResult {
    let (mut bots, admission) = bots::connect_bots(&[(clients, 0, None)]).await;
    admission?;
    let [bot] = bots.as_mut_slice() else {
        return Err("D150 needs one legacy Character".into());
    };
    check_profile(bot, None).await?;
    let first = bot.client()?.join_snapshot().game_session_id;
    let position = bot.client()?.world_spatial().actor_position;
    evidence(
        "D150 legacy_login=PASS transport=tls birth=null vocation=none level=1 xp=0 pg=matched",
    );
    bots::relog_all(&mut bots).await?;
    let [bot] = bots.as_mut_slice() else {
        return Err("D150 lost its legacy Character".into());
    };
    let client = bot.client()?;
    if client.join_snapshot().game_session_id == first
        || client.world_spatial().actor_position != position
    {
        return Err("D150 relog did not load the saved position into a new session".into());
    }
    check_profile(bot, None).await?;
    bots::logout_at(bot, "d150-terminal-logout").await?;
    wait_character_released(clients, 0).await?;
    evidence(
        "D150 legacy_relogin=PASS transport=tls session=new position=saved birth=null vocation=none level=1 xp=0 pg=matched release=confirmed",
    );
    Ok(())
}
