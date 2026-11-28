//! I08 uses the Primal Bag over TLS, verifies fresh admission, and replays the live server bytes.
use super::*;
use oteryn_dev_client::native_session::{Admission, Session, SessionStream};
use oteryn_dev_client::{CharacterInventory, SessionEvent};
use oteryn_protocol_oteryn::{logout::LogoutResult, world_object::UseDisposition};
use std::io::{self, Cursor, Write};
use std::num::NonZeroU32;
use std::pin::Pin;
use std::sync::Mutex;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio_rustls::rustls::pki_types::ServerName;

const LIMIT: usize = 16 * 1024 * 1024;

struct Capture<'a, S> {
    stream: S,
    incoming: &'a Mutex<Vec<u8>>,
}

impl<S: AsyncRead + Unpin> AsyncRead for Capture<'_, S> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let capture = self.get_mut();
        let start = buf.filled().len();
        match Pin::new(&mut capture.stream).poll_read(cx, buf) {
            Poll::Ready(Ok(())) => {
                let bytes = &buf.filled()[start..];
                let mut incoming = capture
                    .incoming
                    .lock()
                    .map_err(|_| io::Error::other("I08 capture lock poisoned"))?;
                if bytes.len() > LIMIT - incoming.len() {
                    return Poll::Ready(Err(io::Error::other("I08 capture exceeded byte limit")));
                }
                incoming.extend_from_slice(bytes);
                Poll::Ready(Ok(()))
            }
            other => other,
        }
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for Capture<'_, S> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.get_mut().stream).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().stream).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().stream).poll_shutdown(cx)
    }
}

struct Replay(Cursor<Vec<u8>>);

impl AsyncRead for Replay {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let reader = &mut self.get_mut().0;
        if reader.position() == reader.get_ref().len() as u64 {
            // The live capture ended after the final observed frame, with the connection open.
            return Poll::Pending;
        }
        Pin::new(reader).poll_read(cx, buf)
    }
}

impl AsyncWrite for Replay {
    fn poll_write(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

fn quantities(inventory: &CharacterInventory) -> BTreeMap<NonZeroU32, u64> {
    let mut counts = BTreeMap::new();
    for item in &inventory.entries {
        *counts.entry(item.item_definition_ref).or_default() += u64::from(item.count.get());
    }
    counts
}

async fn use_bag<S: SessionStream>(
    session: &mut Session<S>,
    bag_ref: NonZeroU32,
) -> TestResult<(u64, CharacterInventory)> {
    let bag = session
        .inventory()
        .ok_or("I08 inventory missing at admission")?
        .entries
        .iter()
        .find(|item| item.item_definition_ref == bag_ref)
        .ok_or("I08 Primal Bag missing at admission")?;
    let handle = bag.handle;
    session.take_events();
    let reply = session.use_item(handle).await?;
    if reply.status != CommandStatus::Accepted || reply.disposition != UseDisposition::Committed {
        return Err(format!("I08 Primal Bag USE reply={reply:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        for event in session.take_events() {
            if let SessionEvent::Inventory(delta) = event {
                return Ok((delta.server_sequence, delta.value));
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("I08 Primal Bag inventory delta timeout".into());
        }
        session.service_liveness(Duration::from_millis(20)).await?;
    }
}

async fn logout<S: SessionStream>(session: &mut Session<S>) -> TestResult {
    let outcome = session.logout().await?;
    if outcome.result != Some(LogoutResult::Accepted) {
        return Err(format!("I08 logout refused: {outcome:?}").into());
    }
    Ok(())
}

fn assert_conversion(
    before: &CharacterInventory,
    after: &CharacterInventory,
    bag_ref: NonZeroU32,
    rewards: &[NonZeroU32],
) -> TestResult<NonZeroU32> {
    let mut expected = quantities(before);
    if expected.remove(&bag_ref) != Some(1)
        || rewards.iter().any(|reward| expected.contains_key(reward))
    {
        return Err("I08 requires exactly one bag and no rewards before USE".into());
    }
    let actual = quantities(after);
    let mut minted = rewards
        .iter()
        .copied()
        .filter(|reward| actual.contains_key(reward));
    let reward = minted
        .next()
        .ok_or("I08 inventory did not contain a pool reward")?;
    if minted.next().is_some() {
        return Err("I08 USE minted multiple rewards".into());
    }
    expected.insert(reward, 1);
    if actual != expected
        || after.equipment != before.equipment
        || after.main_backpack != before.main_backpack
    {
        return Err("I08 USE did not exchange exactly one bag for one pool reward".into());
    }
    Ok(reward)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let capture_dir = required("NODE_BOOT_I08_CAPTURE_DIR")?;
    let identity = character_prep::dedicated_character(clients, "NODE_BOOT_I08_ACCOUNT").await?;
    let clients = &identity.clients(clients);
    character_prep::declare_seeded(
        "I08.client",
        "one Primal Bag",
        "opening the bag is the mechanic under test",
    );
    let mut seed = connect_character(clients, 0xc8, 0).await?;
    character_prep::grant_item(&seed, 39546, 1).await?;
    logout(seed.session_mut()).await?;
    drop(seed);

    let bags = oteryn_gameplay_items::reward_bag::reward_bags().ok_or("reward pool absent")?;
    let bag = bags
        .iter()
        .find(|bag| bag.constant == "PRIMAL_BAG" && bag.playable)
        .ok_or("playable Primal Bag absent")?;
    let index = provisioning::item_index()?;
    let reference = |key: &str| {
        index
            .revision(key)
            .and_then(|revision| index.definition_ref("Item", key, revision))
            .ok_or_else(|| format!("I08 missing item definition {key}"))
    };
    let bag_ref = reference(&bag.bag)?;
    let rewards = bag
        .rewards
        .iter()
        .map(|reward| reference(&reward.item))
        .collect::<Result<Vec<_>, _>>()?;

    wait_character_released(clients, 0).await?;
    let generation = platform_generation(clients.descriptor, &clients.accounts[0]).await?;
    let (nonce, attempt) = fresh_grant_material(0xc8)?;
    let grant = Grant {
        signing: clients.signing,
        key_id: clients.key_id,
        account_id: &clients.accounts[0],
        character_id: clients.characters[0],
        world_id: clients.world,
        channel_id: clients.channel,
        security_generation: generation,
        scope_generation: clients.scope_generation,
        nonce,
        attempt,
    };
    let token = sign_grant(&grant, now_seconds()?);
    let admission = || -> TestResult<Admission<'_>> {
        Ok(Admission {
            appearance_items: None,
            schema_revision: 1,
            character_id: crate::foundation::CharacterId::decode(&clients.characters[0])?,
            admission_material: token.as_bytes(),
            client_build_id: "oteryn-dev-client/i08-qualification",
            supported_capabilities: oteryn_dev_client::CLIENT_SUPPORTED_CAPABILITIES,
            deadline: Duration::from_secs(20),
        })
    };
    let connector =
        crate::qualification::connector(clients.certificate, &crate::qualification::EXACT)?;
    let stream = tokio::time::timeout(Duration::from_secs(20), async {
        let tcp = tokio::net::TcpStream::connect(clients.address).await?;
        let stream = connector
            .connect(ServerName::try_from("localhost")?, tcp)
            .await?;
        Ok::<_, Box<dyn std::error::Error>>(stream)
    })
    .await??;
    let incoming = Mutex::new(Vec::new());
    let mut live = Session::admit(
        Capture {
            stream,
            incoming: &incoming,
        },
        admission()?,
    )
    .await?;
    let session_id = live.join_snapshot().game_session_id;
    let before = live.inventory().cloned().ok_or("I08 inventory missing")?;
    let (sequence, after) = use_bag(&mut live, bag_ref).await?;
    if live.inventory() != Some(&after) {
        return Err("I08 applied inventory differs from the observed USE delta".into());
    }
    let reward = assert_conversion(&before, &after, bag_ref, &rewards)?;
    // The capture includes the original ServerAccepted, join and USE exchange.
    // The output is created exclusively under the run's private capture directory.
    let path = Path::new(&capture_dir).join(format!(
        "i08-{}-{sequence}.wire",
        uuid_text(session_id.as_bytes())
    ));
    let mut output = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?;
    {
        let capture = incoming.lock().map_err(|_| "I08 capture lock poisoned")?;
        output.write_all(capture.as_slice())?;
    }
    drop(output);
    logout(&mut live).await?;
    drop(live);

    let mut fresh = connect_character(clients, 0xc8, 0).await?;
    if fresh.join_snapshot().game_session_id == session_id
        || quantities(fresh.inventory().ok_or("I08 relog inventory missing")?) != quantities(&after)
    {
        return Err("I08 relog did not retain the exact bag debit and reward credit".into());
    }
    logout(fresh.session_mut()).await?;

    let bytes = std::fs::read(&path)?;
    std::fs::remove_file(path)?;
    let mut replay = Session::admit(Replay(Cursor::new(bytes)), admission()?).await?;
    if replay.join_snapshot().game_session_id != session_id || replay.inventory() != Some(&before) {
        return Err("I08 replay has different session or pre-USE inventory".into());
    }
    let (replayed_sequence, replayed) = use_bag(&mut replay, bag_ref).await?;
    if replayed_sequence != sequence || replayed != after {
        return Err("I08 replay has different USE sequence or post-USE inventory".into());
    }
    Ok(format!(
        "one-bag-consumed reward_ref={reward} session={} sequence={sequence} replay=pass durable=pass",
        uuid_text(session_id.as_bytes())
    ))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::observe(journey(clients)).await;
    evidence(&format!(
        "e2e_all scenario=I08.client status={} seconds={:.3} evidence={}",
        if result.is_ok() { "PASS" } else { "FAIL" },
        started.elapsed().as_secs_f64(),
        result.unwrap_or_else(|error| error.to_string()),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::num::NonZeroU64;
    use tokio::io::AsyncReadExt;

    #[tokio::test]
    async fn capture_and_replay_preserve_every_byte_across_read_boundaries() -> TestResult {
        let bytes = [0, 0, 0, 3, 1, 0, 255];
        let mut reader = Cursor::new(bytes);
        let incoming = Mutex::new(Vec::new());
        let mut capture = Capture {
            stream: &mut reader,
            incoming: &incoming,
        };
        let mut prefix = [0; 4];
        let mut rest = [0; 3];
        capture.read_exact(&mut prefix).await?;
        capture.read_exact(&mut rest).await?;
        let captured = incoming
            .into_inner()
            .map_err(|_| "I08 capture lock poisoned")?;
        assert_eq!(captured, bytes);

        let mut replay = Replay(Cursor::new(captured));
        let mut joined = [0; 7];
        replay.read_exact(&mut joined).await?;
        assert_eq!(joined, bytes);
        Ok(())
    }

    #[test]
    fn conversion_rejects_noop_duplicate_credit_and_existing_reward() -> TestResult {
        fn entry(id: u32, count: u32) -> TestResult<oteryn_dev_client::ItemEntry> {
            Ok(oteryn_dev_client::ItemEntry {
                appearance_id: 0,
                handle: NonZeroU64::try_from(u64::from(id))?,
                item_definition_ref: NonZeroU32::try_from(id)?,
                count: NonZeroU32::try_from(count)?,
                sub_type: 0,
            })
        }

        let bag = entry(39546, 1)?;
        let reward = entry(39150, 1)?;
        let before = CharacterInventory {
            entries: vec![bag],
            ..Default::default()
        };
        let after = CharacterInventory {
            entries: vec![reward],
            ..Default::default()
        };
        let rewards = [reward.item_definition_ref];
        assert_eq!(
            assert_conversion(&before, &after, bag.item_definition_ref, &rewards)?,
            reward.item_definition_ref
        );
        assert!(assert_conversion(&before, &before, bag.item_definition_ref, &rewards).is_err());
        let duplicate = CharacterInventory {
            entries: vec![entry(39150, 2)?],
            ..Default::default()
        };
        assert!(assert_conversion(&before, &duplicate, bag.item_definition_ref, &rewards).is_err());
        let already_owned = CharacterInventory {
            entries: vec![bag, reward],
            ..Default::default()
        };
        assert!(
            assert_conversion(&already_owned, &after, bag.item_definition_ref, &rewards).is_err()
        );
        Ok(())
    }
}
