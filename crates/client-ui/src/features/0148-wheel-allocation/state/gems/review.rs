use super::*;
use std::time::{Duration, Instant};
#[derive(Clone, Debug, PartialEq, Eq)]
struct Fingerprint {
    wheel: wheel::WheelAllocationReply,
    atelier: atelier::AtelierReply,
    resources: Option<atelier::resources::ResourceBalances>,
    points: [u16; 36],
    vessels: wheel::WheelVessels,
    gem: Option<[u8; 16]>,
    modifier: Option<(u8, u16)>,
    quality: u8,
    domain: u8,
}
fn fingerprint(p: &WheelPanel) -> Option<Fingerprint> {
    Some(Fingerprint {
        wheel: p.confirmed_wheel.clone()?,
        atelier: p.confirmed_atelier.clone()?,
        resources: p.confirmed_resources,
        points: p.points,
        vessels: p.vessels,
        gem: p.gem,
        modifier: p.modifier,
        quality: p.quality,
        domain: p.domain,
    })
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Review {
    pub text: String,
    pub expires: Instant,
    pub command: WheelCommand,
    fingerprint: Fingerprint,
    pub nonce: u64,
}
pub fn stage(p: &mut WheelPanel, intent: Intent, occurrence: [u8; 16]) -> bool {
    if p.gems.review.is_some()
        || p.gems.pending.is_some()
        || p.diagram.pending.is_some()
        || !p.ready()
    {
        return false;
    }
    let Some(fingerprint) = fingerprint(p) else {
        return false;
    };
    let mut copy = p.clone();
    let Some(command) = super::super::request(&mut copy, intent, occurrence) else {
        p.feedback = copy.feedback;
        return false;
    };
    let a = &fingerprint.atelier;
    let text=match intent {
  Intent::Reveal=>a.quality_quotes.iter().find(|q|q.quality==p.quality).map(|q|format!("Reveal one quality {} gem?\nCost: {} gold and one unrevealed gem. The result is decided by the server.",p.quality+1,q.reveal_gold)),
  Intent::Dismantle=>p.selected_gem().and_then(|gem|a.quality_quotes.iter().find(|q|q.quality==gem.quality)).map(|q|format!("Dismantle the selected gem permanently?\nReturn: {}–{} {} fragments (server draw).",q.fragment_min,q.fragment_max,if q.fragment_kind==0{"basic"}else{"supreme"})),
  Intent::SwitchDomain=>p.selected_gem().and_then(|gem|a.quality_quotes.iter().find(|q|q.quality==gem.quality)).map(|q|format!("Switch the selected gem's domain?\nCost: {} gold. The server decides its new domain.",q.switch_gold)),
  Intent::GradeUp=>p.modifier.and_then(|(kind,id)|a.grade_quotes.iter().find(|q|q.kind==kind&&q.grade==p.grade(kind,id)+1).map(|q|format!("Upgrade {}?\nCost: {} gold and {} {} fragments. This changes every matching modifier.",super::super::modifier_label(p,kind,id),q.gold,q.fragments,if kind==0{"basic"}else{"supreme"}))),
  _=>None,
 };
    let Some(mut text) = text else {
        return false;
    };
    text.push_str(&format!("\n\n{}\n\nConfirm sends this exact reviewed intent once; the server rechecks current resources.",super::view::balances(p)));
    p.gems.review = Some(Review {
        text,
        expires: Instant::now() + Duration::from_secs(15),
        command,
        fingerprint,
        nonce: p.gems.nonce,
    });
    true
}
pub fn confirm(p: &mut WheelPanel) -> Option<WheelCommand> {
    let review = p.gems.review.take()?;
    if !p.selected
        || p.pending.is_some()
        || p.retry.is_some()
        || p.gems.pending.is_some()
        || p.diagram.pending.is_some()
        || review.expires <= Instant::now()
        || review.nonce != p.gems.nonce
        || fingerprint(p).as_ref() != Some(&review.fingerprint)
    {
        p.gems.feedback = Some(
            "The review expired or its current state changed; choose the operation again".into(),
        );
        return None;
    }
    p.pending = Some(review.command.clone());
    Some(review.command)
}
