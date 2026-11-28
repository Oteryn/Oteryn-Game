//! E02 client replay capture: the NPC gold-to-platinum exchange changes the backpack coins.
//! Client owner: the CharacterInventory domain in `crates/session` (`Session::inventory`). The
//! replay consumes the incoming admission and inventory frames from this live session.
use super::*;
use oteryn_dev_client::client_runtime::npc_shop as runtime;
use oteryn_dev_client::{CharacterInventory, DevClientSession, EntityKind, EntityRef};
use oteryn_protocol_oteryn::item_view;
use oteryn_protocol_oteryn::npc_service as wire;

// Global TibiaWiki (Kepar) documents his coin exchange at the Adventurers' Guild.
const BANKER: &str = "oteryn:npc.kepar";
const GOLD_ID: u32 = 3031;
const PLATINUM_ID: u32 = 3035;
const CRYSTAL_ID: u32 = 3043;
const PROMPT: &str = "How many platinum coins would you like to get?";

pub(super) fn encoded_backpack(client: &DevClientSession) -> TestResult<Vec<u8>> {
    // The coin replays negotiate the backpack-only inventory capability.
    let mut inventory = client.inventory().cloned().ok_or("inventory absent")?;
    inventory.equipment.clear();
    item_view::encode_character_inventory(&inventory)
        .map_err(|error| format!("inventory encode: {error:?}").into())
}

pub(super) fn coin_refs() -> TestResult<[std::num::NonZeroU32; 3]> {
    let index = provisioning::item_index()?;
    Ok([
        provisioning::item_reference(&index, GOLD_ID)?,
        provisioning::item_reference(&index, PLATINUM_ID)?,
        provisioning::item_reference(&index, CRYSTAL_ID)?,
    ])
}

pub(super) fn backpack_coins(client: &DevClientSession, refs: [std::num::NonZeroU32; 3]) -> TestResult<[u64; 3]> {
    let inventory = client.inventory().ok_or("step=inventory observed=snapshot-absent")?;
    let mut counts = [0; 3];
    for entry in &inventory.entries {
        if let Some(index) = refs.iter().position(|reference| *reference == entry.item_definition_ref) {
            counts[index] += u64::from(entry.count.get());
        }
    }
    Ok(counts)
}

pub(super) async fn wait_coins(
    client: &mut DevClientSession,
    refs: [std::num::NonZeroU32; 3],
    expected: [u64; 3],
    step: &'static str,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    loop {
        let observed = backpack_coins(client, refs)?;
        if observed == expected {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("step={step} observed=coin-delta-timeout:{observed:?} expected={expected:?}").into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
        client.take_events();
    }
}

async fn talk(client: &mut DevClientSession, actor: EntityRef, text: &str) -> TestResult<wire::NpcConversation> {
    tokio::time::sleep(Duration::from_millis(260)).await;
    let disposition = runtime::talk(
        client.session_mut(),
        &wire::NpcTalkIntent { npc_actor: actor, text: text.into() },
    )
    .await?;
    if disposition != wire::NpcIntentDisposition::Ok {
        return Err(format!("step=talk-{text} observed=disposition:{disposition:?}").into());
    }
    let delta = client
        .session_mut()
        .read_gated_delta(wire::STATE_DOMAIN_NPC_CONVERSATION, 1)
        .await?;
    wire::decode_npc_conversation(&delta.payload)
        .map_err(|error| format!("step=talk-{text} observed=malformed-conversation:{error:?}").into())
}

fn visible_npc(client: &DevClientSession, npc: &oteryn_world_bundle::npc::Npc) -> Option<EntityRef> {
    client.world_entities()?.iter().find(|entity| {
        entity.kind == EntityKind::Npc
            && npc.placements.iter().any(|placement| {
                entity.position.x == i32::from(placement.x)
                    && entity.position.y == i32::from(placement.y)
                    && entity.position.floor == -i16::from(placement.floor)
            })
    }).map(|entity| entity.entity)
}

pub(super) async fn approach(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npc: &oteryn_world_bundle::npc::Npc,
) -> TestResult<EntityRef> {
    let path = npc_approach_path(client, world, npc)
        .await?
        .ok_or_else(|| format!("step=walk-npc observed=no-route:{}", npc.key))?;
    approach_along(client, world, npc, &path).await
}

async fn approach_along(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npc: &oteryn_world_bundle::npc::Npc,
    path: &[StepDirection],
) -> TestResult<EntityRef> {
    let from = client.world_spatial().actor_position;
    evidence(&format!(
        "E02_ROUTE npc={} from={from:?} steps={}",
        npc.key,
        path.len()
    ));
    walk(client, world, path).await?;
    visible_npc(client, npc)
        .ok_or_else(|| format!("step=walk-npc observed=actor-not-visible:{}", npc.key).into())
}

pub(super) async fn find_exchange_npc(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
    book: &crate::gameplay_transport::NpcTalkBook,
) -> TestResult<(EntityRef, String)> {
    let banker = npcs
        .npcs
        .iter()
        .find(|npc| npc.key == BANKER)
        .filter(|npc| book.get(&npc.key).is_some_and(|entry| entry.bank()))
        .ok_or_else(|| {
            format!("step=find-exchange-npc observed=missing-authored-banker:{BANKER}")
        })?;
    let path = npc_approach_path(client, world, banker)
        .await?
        .filter(|path| !path.is_empty())
        .ok_or("step=walk-npc observed=no-discriminating-route")?;
    for direction in path {
        let before = tile(client.world_spatial().actor_position)?;
        let expected = destination(world, before, direction)
            .filter(|landing| *landing != before)
            .ok_or("step=walk-npc observed=no-discriminating-landing")?;
        let outcome = client.step(direction).await?;
        let after = tile(client.world_spatial().actor_position)?;
        if outcome.disposition != StepDisposition::Moved
            || outcome.world_spatial_delta.is_none()
            || after != expected
        {
            return Err(format!(
                "step=walk-npc observed={after:?} expected={expected:?} outcome={outcome:?}"
            )
            .into());
        }
        verdict::Harness::global().progress("walk");
        client.take_events();
    }
    let actor = visible_npc(client, banker).ok_or("step=walk-npc observed=kepar-not-visible")?;
    Ok((actor, banker.key.clone()))
}

pub(super) const GOLD_PROMPT: &str = PROMPT;
pub(super) const COMPLETE: &str = "Here you are.";
pub(super) const PLATINUM_DESTINATION: &str =
    "Would you like to change your platinum coins into gold or crystal?";

fn require_dialogue(reply: &wire::NpcConversation, actor: EntityRef, expected: &str) -> TestResult {
    if reply.open != Some((actor, wire::NpcConversationState::Open)) || reply.lines != [expected] {
        return Err(format!("E02 dialogue expected={expected:?} observed={reply:?}").into());
    }
    Ok(())
}

fn unrelated_inventory(
    mut inventory: CharacterInventory,
    refs: [std::num::NonZeroU32; 3],
    relog: bool,
) -> CharacterInventory {
    inventory
        .entries
        .retain(|item| !refs.contains(&item.item_definition_ref));
    if relog {
        // The wire contract assigns handles per session. Compare every other field across relog.
        for item in inventory
            .main_backpack
            .iter_mut()
            .chain(inventory.entries.iter_mut())
            .chain(
                inventory
                    .equipment
                    .iter_mut()
                    .map(|equipment| &mut equipment.item),
            )
        {
            item.handle = std::num::NonZeroU64::MIN;
        }
    }
    inventory
}

fn require_custody(
    expected_coins: [u64; 3],
    expected_bank: u64,
    coins: [u64; 3],
    bank: u64,
    collateral: bool,
) -> TestResult {
    if coins != expected_coins || bank != expected_bank || collateral {
        return Err(format!(
            "E02 custody expected_coins={expected_coins:?} observed_coins={coins:?} expected_bank={expected_bank} observed_bank={bank} collateral={collateral}"
        ).into());
    }
    Ok(())
}

pub(super) struct ExchangeProof {
    refs: [std::num::NonZeroU32; 3],
    inventory: CharacterInventory,
    pub(super) coins: [u64; 3],
    pub(super) bank: u64,
    checkpoints: String,
}

impl ExchangeProof {
    pub(super) async fn start(
        client: &DevClientSession,
        clients: &SeamClients<'_>,
        refs: [std::num::NonZeroU32; 3],
        coins: [u64; 3],
    ) -> TestResult<Self> {
        let inventory = client
            .inventory()
            .cloned()
            .ok_or("E02 initial inventory absent")?;
        let rope = provisioning::item_reference(&provisioning::item_index()?, 3003)?;
        if !inventory
            .entries
            .iter()
            .any(|item| item.item_definition_ref == rope)
        {
            return Err("E02 unrelated rope fixture missing".into());
        }
        let bank = super::full_row_e06::bank_balance(clients, 1).await?;
        let mut proof = Self {
            refs,
            inventory,
            coins,
            bank,
            checkpoints: format!("refs={},{},{}\n", refs[0], refs[1], refs[2]),
        };
        proof.check(client, clients, false).await?;
        proof.checkpoint(client, 0);
        Ok(proof)
    }

    pub(super) async fn check(
        &self,
        client: &DevClientSession,
        clients: &SeamClients<'_>,
        relog: bool,
    ) -> TestResult {
        let coins = backpack_coins(client, self.refs)?;
        let bank = super::full_row_e06::bank_balance(clients, 1).await?;
        let inventory = client.inventory().cloned().ok_or("E02 inventory absent")?;
        let collateral = unrelated_inventory(inventory, self.refs, relog)
            != unrelated_inventory(self.inventory.clone(), self.refs, relog);
        require_custody(self.coins, self.bank, coins, bank, collateral)
    }

    fn checkpoint(&mut self, client: &DevClientSession, confirmation: u64) {
        use std::fmt::Write as _;
        let [gold, platinum, crystal] = self.coins;
        let _ = writeln!(
            self.checkpoints,
            "checkpoint={},{confirmation},{gold},{platinum},{crystal}",
            client.last_server_sequence()
        );
    }

    pub(super) async fn prompts(
        &mut self,
        client: &mut DevClientSession,
        clients: &SeamClients<'_>,
        actor: EntityRef,
        words: &[(&str, &str)],
    ) -> TestResult {
        for &(word, expected) in words {
            self.check(client, clients, false).await?;
            let inventory = client.inventory().cloned();
            let reply = talk(client, actor, word).await?;
            require_dialogue(&reply, actor, expected)?;
            client.service_liveness(Duration::from_millis(25)).await?;
            self.check(client, clients, false).await?;
            if client.inventory() != inventory.as_ref() {
                return Err(
                    format!("E02 inventory mutated before confirmation after {word:?}").into(),
                );
            }
            self.checkpoint(client, 0);
        }
        Ok(())
    }

    pub(super) async fn confirm(
        &mut self,
        client: &mut DevClientSession,
        clients: &SeamClients<'_>,
        actor: EntityRef,
        expected: &str,
        coins: [u64; 3],
        bank: u64,
    ) -> TestResult {
        self.check(client, clients, false).await?;
        let inventory = client.inventory().cloned();
        let unchanged_coins = self.coins == coins;
        let confirmation = client.next_command_id();
        let reply = talk(client, actor, "yes").await?;
        require_dialogue(&reply, actor, expected)?;
        wait_coins(client, self.refs, coins, "confirmed-exchange").await?;
        self.coins = coins;
        self.bank = bank;
        self.check(client, clients, false).await?;
        if unchanged_coins && client.inventory() != inventory.as_ref() {
            return Err("E02 refused confirmation changed inventory".into());
        }
        self.checkpoint(client, confirmation);
        Ok(())
    }

    pub(super) fn write_capture(
        self,
        variant: &str,
        frames: Option<Vec<u8>>,
        session_id: String,
        sequence: u64,
    ) -> TestResult<String> {
        let root = std::env::var_os("E02_CAPTURE_DIR")
            .ok_or("E02 replay capture directory missing")?;
        let root = Path::new(&root);
        std::fs::write(
            root.join(format!("e02-{variant}.frames")),
            frames.ok_or("E02 capture overflow")?,
        )?;
        std::fs::write(
            root.join(format!("e02-{variant}.txt")),
            format!(
                "session={session_id}\nafter_sequence={sequence}\n{}",
                self.checkpoints
            ),
        )?;
        Ok(format!("session={session_id} sequence={sequence}"))
    }
}

pub(super) fn gold_quote(amount: u64) -> String {
    format!(
        "So you would like me to change {} of your gold coins into {amount} platinum coins?",
        amount * 100
    )
}

pub(super) async fn greet(
    proof: &mut ExchangeProof,
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    actor: EntityRef,
    name: &str,
) -> TestResult {
    // Global Kepar opens a banking conversation on "hi". Also fence the exact
    // authored live line so extra or contradictory dialogue fails this step.
    let line = format!("Yes? What may I do for you, {name}? Bank business, perhaps?");
    proof
        .prompts(client, clients, actor, &[("hi", &line)])
        .await
}

pub(super) async fn seed_coin_profile(
    clients: &SeamClients<'_>,
    row: &str,
    name: &str,
    tag: u8,
    refs: [std::num::NonZeroU32; 3],
) -> TestResult<Vec<[u8; 16]>> {
    mod hunt_route {
        use super::*;
        include!("qualification_hunt_route.rs");
    }

    let mut characters = clients.characters.to_vec();
    characters[1] = hunt_route::qualification_character(clients, 1, tag, name, "Knight").await?;
    let dedicated = SeamClients {
        characters: &characters,
        ..*clients
    };
    let mut client = connect_world(&dedicated, tag).await?;
    let before = backpack_coins(&client, refs)?;
    if before != [50, 0, 0] {
        return Err(format!("E02 starter gold expected [50, 0, 0], observed {before:?}").into());
    }
    character_prep::declare_seeded(
        row,
        "100 carried gold coins",
        "complete the 50-coin starter balance on the dedicated character",
    );
    // The fixture accepts at most 20 per ordinary named profile and replays
    // identical claims in one session, so use three distinct grants totaling 50.
    for count in [11, 19, 20] {
        character_prep::grant_item(&client, GOLD_ID, count).await?;
    }
    character_prep::declare_seeded(row, "one rope", "witness unrelated backpack custody");
    character_prep::grant_item(&client, 3003, 1).await?;
    if row == "E02.full" {
        character_prep::declare_seeded(
            row,
            "one crystal coin",
            "exercise both crystal exchange directions",
        );
        character_prep::grant_item(&client, CRYSTAL_ID, 1).await?;
    }
    let logout = client.session_mut().logout().await?;
    if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("E02 setup logout refused: {:?}", logout.result).into());
    }
    Ok(characters)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let refs = coin_refs()?;
    use oteryn_dev_client::native_session::test_support::capture_server_frames;

    let characters =
        seed_coin_profile(clients, "E02.client", "Coin Exchange Capture", 0x33, refs).await?;
    let dedicated = SeamClients {
        characters: &characters,
        ..*clients
    };
    let (exchange_result, frames) = capture_server_frames(async {
        let mut client = connect_world(&dedicated, 0x33).await?;
        let mut proof = ExchangeProof::start(&client, &dedicated, refs, [100, 0, 0]).await?;
        let talk_book = full_e2ef20::qualification_npc_talk_book(&mut client).await?;
        let (actor, npc_key) =
            find_exchange_npc(&mut client, world, &metadata.npcs, &talk_book).await?;
        greet(
            &mut proof,
            &mut client,
            &dedicated,
            actor,
            "Coin Exchange Capture",
        )
        .await?;
        proof
            .prompts(
                &mut client,
                &dedicated,
                actor,
                &[("change gold", GOLD_PROMPT), ("1 platinum", &gold_quote(1))],
            )
            .await?;
        let bank = proof.bank;
        proof
            .confirm(&mut client, &dedicated, actor, COMPLETE, [0, 1, 0], bank)
            .await?;
        TestResult::Ok((client, npc_key, proof))
    })
    .await;
    let (mut client, npc_key, proof) = exchange_result?;
    let old_session = client.join_snapshot().game_session_id;
    let session_id = uuid_text(old_session.as_bytes());
    let sequence = client.last_server_sequence();
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=client-capture-logout observed=refused".into());
    }
    drop(client);

    let mut fresh = connect_world(&dedicated, 0x33).await?;
    proof.check(&fresh, &dedicated, true).await?;
    if fresh.join_snapshot().game_session_id == old_session {
        return Err("step=client-capture-relogin observed=session-reused".into());
    }
    if fresh.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=client-capture-final-logout observed=refused".into());
    }
    let binding = proof.write_capture("client", frames, session_id, sequence)?;
    Ok(format!(
        "npc={npc_key} before=[100, 0, 0] after=[0, 1, 0] relog=[0, 1, 0] {binding}"
    ))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(150), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=E02.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=E02.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=E02.client status=FAIL evidence=timeout"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_dialogue_rejects_contradictory_lines() {
        let actor = EntityRef {
            identity: [1; 16],
            generation: 1,
        };
        let mut reply = wire::NpcConversation {
            open: Some((actor, wire::NpcConversationState::Open)),
            lines: vec![COMPLETE.into()],
        };
        assert!(require_dialogue(&reply, actor, COMPLETE).is_ok());
        reply.lines.push("I could not change your coins.".into());
        assert!(require_dialogue(&reply, actor, COMPLETE).is_err());
    }

    #[test]
    fn e02_custody_rejects_early_conversion_bank_credit_and_collateral() {
        let gold = [100, 0, 0];
        let platinum = [0, 1, 0];
        assert!(require_custody(gold, 0, gold, 0, false).is_ok());
        assert!(require_custody(gold, 0, platinum, 0, false).is_err());
        assert!(require_custody(platinum, 0, platinum, 100, false).is_err());
        assert!(require_custody(platinum, 0, platinum, 0, true).is_err());
        assert!(require_custody([100, 100, 0], 0, [100, 1, 0], 0, false).is_err());
        assert!(require_custody([100, 0, 1], 0, [100, 0, 0], 0, false).is_err());
    }
}
