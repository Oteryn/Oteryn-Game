//! Closed Crystal201 Lynda166–210/244–260; persistent reciprocal consent precedes outfits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Free,
    Proposed,
    Accepted,
    Married,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Participant {
    pub character: [u8; 16],
    pub status: Status,
    pub partner: Option<[u8; 16]>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    UnknownPartner,
    SelfProposal,
    AlreadyProposed,
    AlreadyCommitted,
    MissingMaterials,
    PartnerCommitted,
    PartnerProposedElsewhere,
    NothingToCancel,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Transition {
    Propose { partner: [u8; 16] },
    Accept { partner: [u8; 16] },
    Cancel { partner: [u8; 16] },
}
pub const MATERIALS: [(u32, u32); 2] = [(3004, 1), (9586, 1)];
pub const OUTFIT_LOOKS: [u16; 2] = [328, 329];
pub fn propose(
    actor: Participant,
    target: Option<Participant>,
    has_ring: bool,
    has_box: bool,
) -> Result<Transition, Refusal> {
    match actor.status {
        Status::Proposed => return Err(Refusal::AlreadyProposed),
        Status::Married => return Err(Refusal::AlreadyCommitted),
        Status::Free | Status::Accepted => {}
    }
    let target = target.ok_or(Refusal::UnknownPartner)?;
    if target.character == actor.character {
        return Err(Refusal::SelfProposal);
    }
    if !has_ring || !has_box {
        return Err(Refusal::MissingMaterials);
    }
    match target.status {
        Status::Accepted | Status::Married => Err(Refusal::PartnerCommitted),
        Status::Proposed if target.partner == Some(actor.character) => Ok(Transition::Accept {
            partner: target.character,
        }),
        Status::Proposed => Err(Refusal::PartnerProposedElsewhere),
        Status::Free => Ok(Transition::Propose {
            partner: target.character,
        }),
    }
}
pub fn cancel(actor: Participant) -> Result<Transition, Refusal> {
    if actor.status != Status::Proposed {
        return Err(Refusal::NothingToCancel);
    }
    actor
        .partner
        .map(|partner| Transition::Cancel { partner })
        .ok_or(Refusal::NothingToCancel)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn player(n: u8, status: Status, partner: Option<u8>) -> Participant {
        Participant {
            character: [n; 16],
            status,
            partner: partner.map(|n| [n; 16]),
        }
    }
    #[test]
    fn actual_reciprocal_partner_is_required_before_both_outfit_grants() {
        let a = player(1, Status::Free, None);
        let b = player(2, Status::Free, None);
        assert_eq!(
            propose(a, Some(b), true, true),
            Ok(Transition::Propose { partner: [2; 16] })
        );
        assert_eq!(
            propose(b, Some(player(1, Status::Proposed, Some(2))), true, true),
            Ok(Transition::Accept { partner: [1; 16] })
        );
        assert_eq!(
            propose(b, Some(player(1, Status::Proposed, Some(3))), true, true),
            Err(Refusal::PartnerProposedElsewhere)
        );
        assert_eq!(propose(a, Some(a), true, true), Err(Refusal::SelfProposal));
        assert_eq!(propose(a, None, true, true), Err(Refusal::UnknownPartner));
    }
    #[test]
    fn each_actual_character_supplies_its_own_materials_and_pending_only_refunds() {
        let actor = player(2, Status::Free, None);
        let pending = player(1, Status::Proposed, Some(2));
        assert_eq!(
            propose(actor, Some(pending), false, true),
            Err(Refusal::MissingMaterials)
        );
        assert_eq!(
            propose(actor, Some(pending), true, false),
            Err(Refusal::MissingMaterials)
        );
        assert_eq!(cancel(pending), Ok(Transition::Cancel { partner: [2; 16] }));
        assert_eq!(
            cancel(player(1, Status::Accepted, Some(2))),
            Err(Refusal::NothingToCancel)
        );
        assert_eq!(
            cancel(player(1, Status::Married, Some(2))),
            Err(Refusal::NothingToCancel)
        );
    }
}
