//! Opt-in recording of a single admitted qualification exchange.
use std::cell::RefCell;
use std::future::Future;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use oteryn_session_tcp::TcpTlsStream;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

const MAX_CAPTURE_BYTES: usize = 256 * 1024;

tokio::task_local! {
    static EXCHANGE: RefCell<QualificationExchange>;
}

/// TLS plaintext read and written within the capture scope.
#[derive(Default)]
pub struct QualificationExchange {
    pub inbound: Vec<u8>,
    pub outbound: Vec<u8>,
    pub write_offsets: Vec<(usize, usize)>,
}

/// Record one operation after admission on the current task.
pub async fn capture_qualification_exchange<T>(
    operation: impl Future<Output = T>,
) -> (T, QualificationExchange) {
    EXCHANGE
        .scope(RefCell::default(), async {
            let result = operation.await;
            (result, EXCHANGE.with(RefCell::take))
        })
        .await
}

fn record(buffer: &mut Vec<u8>, bytes: &[u8]) -> io::Result<()> {
    if bytes.len() > MAX_CAPTURE_BYTES - buffer.len() {
        return Err(io::Error::other(
            "qualification capture exceeded its byte limit",
        ));
    }
    buffer.extend_from_slice(bytes);
    Ok(())
}

/// The dev-client TLS stream with optional qualification capture.
#[derive(Debug)]
pub struct QualificationTransport(pub(super) TcpTlsStream);

impl AsyncRead for QualificationTransport {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let start = buffer.filled().len();
        let result = Pin::new(&mut self.0).poll_read(cx, buffer);
        if matches!(result, Poll::Ready(Ok(())))
            && let Ok(Err(error)) = EXCHANGE.try_with(|exchange| {
                record(
                    &mut exchange.borrow_mut().inbound,
                    &buffer.filled()[start..],
                )
            })
        {
            return Poll::Ready(Err(error));
        }
        result
    }
}

impl AsyncWrite for QualificationTransport {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        let result = Pin::new(&mut self.0).poll_write(cx, bytes);
        if let Poll::Ready(Ok(written)) = result
            && let Ok(Err(error)) = EXCHANGE.try_with(|exchange| {
                let mut exchange = exchange.borrow_mut();
                let outbound = exchange.outbound.len();
                let inbound = exchange.inbound.len();
                record(&mut exchange.outbound, &bytes[..written])?;
                if written != 0 {
                    exchange.write_offsets.push((outbound, inbound));
                }
                Ok(())
            })
        {
            return Poll::Ready(Err(error));
        }
        result
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.0).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.0).poll_shutdown(cx)
    }
}

/// Identity of the current Linux node-boot runner, including the process start tick.
pub fn qualification_runner_identity() -> io::Result<String> {
    let mut pid = std::process::id();
    let mut runner = None;
    while pid > 1 {
        let stat = std::fs::read_to_string(format!("/proc/{pid}/stat"))?;
        let fields: Vec<_> = stat
            .rsplit_once(") ")
            .ok_or_else(|| io::Error::other("malformed process stat"))?
            .1
            .split_whitespace()
            .collect();
        let args = std::fs::read(format!("/proc/{pid}/cmdline"))?;
        if args
            .split(|&byte| byte == 0)
            .nth(1)
            .is_some_and(|script| script.ends_with(b"tools/qualification/node_boot/run.sh"))
        {
            let start = fields
                .get(19)
                .ok_or_else(|| io::Error::other("missing process start"))?;
            runner = Some(format!("{pid}:{start}"));
        }
        pid = fields
            .get(1)
            .and_then(|parent| parent.parse().ok())
            .ok_or_else(|| io::Error::other("missing parent process"))?;
    }
    runner.ok_or_else(|| io::Error::other("C18a replay requires the current node-boot runner"))
}

/// Verify ordered authoritative publications for the fed Sorcerer.
/// Each completed outdoor mana interval can credit two points, capped at max.
pub fn qualification_mana_history(
    charged: (u64, u32, u32),
    history: &[(u64, u32, u32, u64)],
    max_mana: u32,
    logout: (u64, u32, u32),
) -> bool {
    let mut last = charged;
    let mut last_observation_ms = 0;
    let mut last_credit_ms = None;
    let mut observed_full_interval = false;
    // The caster may already have a partial interval when Scorch is cast.
    const FED_MANA_INTERVAL_MS: u64 = 3_000;
    for &(sequence, mana, soul, elapsed_ms) in history {
        if sequence <= last.0
            || sequence >= logout.0
            || elapsed_ms < last_observation_ms
            || soul != last.2
            || (mana != last.1 && mana != last.1.saturating_add(2).min(max_mana))
        {
            return false;
        }
        if mana != last.1 {
            if last_credit_ms.is_some_and(|previous| elapsed_ms - previous < FED_MANA_INTERVAL_MS) {
                return false;
            }
            observed_full_interval |= last_credit_ms.is_some();
            last_credit_ms = Some(elapsed_ms);
        }
        last_observation_ms = elapsed_ms;
        last = (sequence, mana, soul);
    }
    (last_credit_ms.is_none() || observed_full_interval) && (last.1, last.2) == (logout.1, logout.2)
}

/// The first changed stats frame in the cast exchange must carry exactly the Scorch debit.
pub fn qualification_scorch_charge(
    preceding_sequence: u64,
    sequence: u64,
    before: oteryn_protocol_oteryn::actor_spell::ActorVitals,
    after: oteryn_protocol_oteryn::actor_spell::ActorVitals,
) -> bool {
    sequence > preceding_sequence
        && before.mana.checked_sub(8).is_some_and(|mana| {
            after == oteryn_protocol_oteryn::actor_spell::ActorVitals { mana, ..before }
        })
}

/// Match the east-facing Scorch footprint and fire damage in one cast decision.
pub fn qualification_scorch_impact(
    events: &[oteryn_protocol_oteryn::presentation::PresentationEvent],
    origin: oteryn_protocol_oteryn::world_spatial::ActorPosition,
    caster: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
    rat_position: oteryn_protocol_oteryn::world_spatial::ActorPosition,
    rat: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
    spell: u32,
) -> Result<Option<u64>, String> {
    use oteryn_protocol_oteryn::presentation::{
        PresentationColor, PresentationEventKind as Kind, PresentationSource, PresentationValueKind,
    };
    let mut matching = events.iter().filter(|event| {
        matches!(event.kind,
            Kind::SpellWords { caster: actor, position, spell: index }
            if actor == caster && position == origin && index == spell
        )
    });
    let Some(words) = matching.next() else {
        return Ok(None);
    };
    if matching.next().is_some() {
        return Err("C18a multiple matching spell decisions".into());
    }
    let mut cells = Vec::new();
    let mut damage = None;
    for event in events
        .iter()
        .filter(|event| event.decision_ordinal == words.decision_ordinal)
    {
        match event.kind {
            Kind::MagicEffect {
                position,
                effect_id: 16,
                source: PresentationSource::Own,
            } => {
                cells.push((position.x - origin.x, position.y - origin.y, position.floor));
            }
            Kind::ValueText {
                kind: PresentationValueKind::DamageDealt,
                position,
                target,
                attacker: Some(actor),
                primary,
                secondary: None,
            } if actor == caster && target == rat && position == rat_position => {
                if primary.color != PresentationColor::Orange
                    || damage.replace(primary.value).is_some()
                {
                    return Err(format!(
                        "C18a expected one fire hit in Scorch's decision: {events:?}"
                    ));
                }
            }
            Kind::MagicEffect { .. }
            | Kind::ValueText {
                kind:
                    PresentationValueKind::DamageDealt
                    | PresentationValueKind::DamageReceived
                    | PresentationValueKind::DamageOthers,
                ..
            } => {
                return Err(format!(
                    "C18a unexpected Scorch event: decision={} emission={} expected=own_effect_16_or_single_orange_damage(caster={caster:?}, rat={rat:?}, position={rat_position:?}) actual={:?}",
                    event.decision_ordinal, event.emission_ordinal, event.kind,
                ));
            }
            _ => {}
        }
    }
    cells.sort_unstable();
    // East rotation of the cardinal cone about the caster-adjacent origin.
    let expected = [
        (1, 0),
        (2, -1),
        (2, 0),
        (2, 1),
        (3, -1),
        (3, 0),
        (3, 1),
        (4, -2),
        (4, -1),
        (4, 0),
        (4, 1),
        (4, 2),
    ]
    .map(|(x, y)| (x, y, origin.floor));
    if cells != expected {
        return Err(format!(
            "C18a Scorch cone cells {cells:?}; expected {expected:?}"
        ));
    }
    if !expected.contains(&(
        rat_position.x - origin.x,
        rat_position.y - origin.y,
        rat_position.floor,
    )) {
        return Err("C18a Rat impact is outside the verified Scorch cone".into());
    }
    damage
        .map(Some)
        .ok_or_else(|| "C18a Scorch cone lacks exact Rat fire damage".into())
}
