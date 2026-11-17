use super::*;
use oteryn_client_runtime::{
    cui3::item_forge::Operation,
    forge_preview as wire,
    raw_feature::{CommandStatus, GatedCommandResult},
};
fn effect() -> wire::Effects {
    wire::Effects {
        item: wire::Item {
            handle: std::num::NonZeroU64::MIN,
            definition_ref: 1,
            appearance: 3264,
            name: "Test sword".into(),
            classification: 1,
            tier: 0,
            maximum_tier: 1,
            container: false,
        },
        tiers: vec!["0".into(), "1 (0.50% Onslaught)".into()],
    }
}
#[test]
fn exaltation_reply_refuses_stale_selection_and_scope() -> Result<(), &'static str> {
    let e = effect();
    let mut p = ItemForgePanel {
        selected: true,
        first_item: 1,
        ..Default::default()
    };
    p.preview.selected = true;
    p.preview.remember(e.item.clone());
    let c = super::super::preview::request(&mut p, wire::Selection::Effects(e.item.handle))
        .ok_or("request")?;
    let request = wire::Request::decode(&c.payload).ok_or("decode")?;
    let r = GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload: wire::Reply {
            nonce: request.nonce,
            payload: wire::Payload::Effects(e.clone()),
        }
        .encode()
        .ok_or("encode")?,
    };
    p.first_item = 2;
    super::super::preview::apply(&mut p, &c, &r);
    assert!(p.tier_effects.is_empty());
    p.first_item = 1;
    let c = super::super::preview::request(&mut p, wire::Selection::Effects(e.item.handle))
        .ok_or("request")?;
    p.scope_changed();
    super::super::preview::apply(&mut p, &c, &r);
    assert!(p.tier_effects.is_empty());
    Ok(())
}
#[test]
fn exaltation_actual_receipt_tiers_spend_and_all_bonuses_are_presented() -> Result<(), &'static str>
{
    let e = effect();
    let labels = [
        "No additional bonus",
        "Dust cost waived",
        "Core cost waived",
        "Gold cost waived",
        "Donor retained one tier lower",
        "Donor retained at its original tier",
        "Donor retained at the normal result tier",
        "Extra target tier bonus",
        "Failure protection preserved the donor tier",
    ];
    for bonus in 0..=8 {
        let r = reply::Receipt {
            operation: Operation::Fusion,
            succeeded: bonus != 8,
            applied: true,
            reason: None,
            bonus,
            resources: Some(reply::Resources {
                dust: 0,
                dust_limit: 100,
                first_tier: Some(1),
                second_tier: None,
                gold_spent: 123,
                cores_spent: 2,
                slivers_spent: 0,
                slivers_created: 0,
                cores_created: 0,
            }),
        };
        let text = result_text(&r, Some(&e), None);
        assert!(text.contains("Target: tier 1 (0.50% Onslaught) (reference effect)"));
        assert!(text.contains("Donor: consumed"));
        assert!(text.contains("123 gold, 2 cores"));
        assert!(text.contains(labels[usize::from(bonus)]));
        assert!(!text.contains("Unrecognized"));
    }
    let mut r = reply::Receipt {
        operation: Operation::Fusion,
        succeeded: false,
        applied: false,
        reason: Some("INSUFFICIENT_GOLD".into()),
        bonus: 0,
        resources: None,
    };
    assert!(!result_text(&r, None, None).contains("consumed"));
    r.applied = true;
    r.bonus = 8;
    assert!(result_text(&r, None, None).contains("Failure protection preserved"));
    assert!(effects_text(&e).contains("not a claim of active combat effects"));
    Ok(())
}
