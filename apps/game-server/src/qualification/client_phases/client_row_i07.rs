//! I07 client replay capture: real quick-loot filter, Gold Pouch destination and loot-all replies.
use super::*;
use oteryn_protocol_oteryn::quick_loot::{
    QuickLootAction, QuickLootCategory, QuickLootFilterMode, QuickLootRecipient, QuickLootRequest,
    decode_quick_loot_result, encode_quick_loot_request,
};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result =
        verdict::indexed_timeout(Duration::from_secs(30), verdict::observe(journey(clients))).await;
    let result = result.unwrap_or_else(|_| Err("I07 client capture timed out".into()));
    evidence(&format!(
        "e2e_all scenario=I07.client status={}",
        if result.is_ok() { "PASS" } else { "FAIL" }
    ));
    if let Err(error) = result {
        evidence(&format!("I07_CLIENT_GAP {error}"));
    }
}

fn request(action: QuickLootAction) -> QuickLootRequest {
    QuickLootRequest {
        action,
        corpse: None,
        category: None,
        recipient: None,
        recipient_id: None,
        fallback_to_main: false,
        filter_mode: None,
        filter_definition_ref: None,
    }
}

fn write_capture(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write as _;
    std::fs::File::create_new(path)?.write_all(bytes)
}

async fn require_filters(
    database: &mut sqlx::PgConnection,
    character: &[u8; 16],
    expected_mode: bool,
    expected: &[(bool, &str)],
) -> TestResult {
    let mode: Option<bool> = sqlx::query_scalar(
        "SELECT accept_only FROM game_character_quick_loot_settings
         WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(character.as_slice())
    .fetch_optional(&mut *database)
    .await?;
    let filters: Vec<(bool, String)> = sqlx::query_as(
        "SELECT accepted, item_production_key FROM game_character_quick_loot_filters
         WHERE character_id=encode($1,'hex')::uuid ORDER BY accepted, item_production_key LIMIT 3",
    )
    .bind(character.as_slice())
    .fetch_all(database)
    .await?;
    if mode.unwrap_or(false) != expected_mode
        || !filters
            .iter()
            .map(|(accept, key)| (*accept, key.as_str()))
            .eq(expected.iter().copied())
    {
        return Err(format!(
            "I07 durable filters expected mode={expected_mode} entries={expected:?}; observed mode={mode:?} entries={filters:?}"
        ).into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    use sqlx::Connection as _;
    const FILTERS: [(bool, &str); 2] = [
        (false, "oteryn:item.tibia.i3031"),
        (true, "oteryn:item.tibia.i3350"),
    ];
    let capture_dir = std::env::var_os("E2E_ALL_CAPTURE_DIR")
        .ok_or("I07 requires a fresh E2E_ALL_CAPTURE_DIR")?;
    let capture_dir = Path::new(&capture_dir);
    let index = provisioning::item_index()?;
    let skip_reference = provisioning::item_reference(&index, 3031)?.get();
    let accept_reference = provisioning::item_reference(&index, 3350)?.get();
    let mut client = connect_world(clients, 0x07).await?;
    let character = &clients.characters[1];
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    require_filters(&mut database, character, false, &[]).await?;
    let mut skip = request(QuickLootAction::FilterAdd);
    skip.filter_mode = Some(QuickLootFilterMode::Skip);
    skip.filter_definition_ref = Some(skip_reference);
    let mut accept = request(QuickLootAction::FilterAdd);
    accept.filter_mode = Some(QuickLootFilterMode::Accept);
    accept.filter_definition_ref = Some(accept_reference);
    let mut pouch = request(QuickLootAction::Assign);
    pouch.category = Some(QuickLootCategory::Gold);
    pouch.recipient = Some(QuickLootRecipient::GoldPouch);
    pouch.recipient_id = Some([7; 16]);
    pouch.fallback_to_main = true;
    let requests = [skip, accept, pouch, request(QuickLootAction::LootAll)];
    let mut item_count = 0;
    for (index, request) in requests.into_iter().enumerate() {
        let payload = encode_quick_loot_request(&request).map_err(|error| format!("{error:?}"))?;
        let reply = client
            .session_mut()
            .gated_command(98, 114, &payload)
            .await?;
        let expected_status = if index == 2 {
            CommandStatus::Rejected
        } else {
            CommandStatus::Accepted
        };
        if reply.status != expected_status {
            return Err(format!(
                "quick-loot {:?} refused: {:?}",
                request.action, reply.status
            )
            .into());
        }
        if index == 0 {
            require_filters(&mut database, character, false, &FILTERS[..1]).await?;
        } else if index == 1 {
            require_filters(&mut database, character, true, &FILTERS).await?;
        }
        if reply.status == CommandStatus::Accepted {
            item_count = decode_quick_loot_result(&reply.payload)
                .map_err(|error| format!("quick-loot reply: {error:?}"))?
                .items
                .len();
        }
        let key = index + 1;
        write_capture(
            &capture_dir.join(format!("i07-{key}-request.bin")),
            &payload,
        )?;
        write_capture(
            &capture_dir.join(format!("i07-{key}-status.bin")),
            &(reply.status as u32).to_be_bytes(),
        )?;
        write_capture(&capture_dir.join(format!("i07-{key}.bin")), &reply.payload)?;
    }
    let original_session = client.join_snapshot().game_session_id;
    let logout = client.session_mut().logout().await?;
    if logout.status != CommandStatus::Accepted
        || logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err(format!("I07 first logout refused: {logout:?}").into());
    }
    drop(client);
    let mut fresh = connect_world(clients, 0x07).await?;
    if fresh.join_snapshot().game_session_id == original_session {
        return Err("I07 relog reused its original session".into());
    }
    require_filters(&mut database, character, true, &FILTERS).await?;
    let logout = fresh.session_mut().logout().await?;
    if logout.status != CommandStatus::Accepted
        || logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err(format!("I07 post-readback logout refused: {logout:?}").into());
    }
    evidence(
        "I07_FILTER_CUSTODY prestate=empty skip=3031 accept=3350 mode=accept fresh_session=true",
    );
    write_capture(
        &capture_dir.join("i07-expected.txt"),
        format!(
            "skip={skip_reference} accept={accept_reference} confirmed=2 results={item_count}\n"
        )
        .as_bytes(),
    )?;
    Ok(())
}
