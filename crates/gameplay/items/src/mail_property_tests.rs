use super::tests::{Fixture, Node};
use super::*;
use oteryn_test_support::property::for_each_case;

// A full, legal parcel tree, with root + direct label + a 20-way nested tree.
fn tree(count: usize) -> Fixture {
    let mut fixture = Fixture::parcel();
    for id in 3..=count as u128 {
        let parent = if id == 3 { 1 } else { 3 + (id - 4) / 20 };
        fixture
            .nodes
            .push(Node::new(id, "bag", Some(parent.to_be_bytes())));
    }
    fixture
}

#[test]
fn mail_property_weight_and_item_conservation_are_order_independent() {
    for_each_case("mail weight and identities", |rng, _| {
        let count = *rng.pick(&[2, 3, 10, 100, PARCEL_ITEMS_MAX]);
        let mut fixture = tree(count);
        let parents: BTreeSet<_> = fixture
            .nodes
            .iter()
            .filter_map(|node| node.parent)
            .collect();
        for node in &mut fixture.nodes {
            let random_weight = rng.u64() as u32;
            node.facts.weight = Some(*rng.pick(&[0, 1, u32::MAX, random_weight]));
            // Non-root, non-label leaves may be ordinary whole stacks.
            if node.item.item_instance_id > 2_u128.to_be_bytes()
                && !parents.contains(&node.item.item_instance_id)
            {
                node.facts.container_capacity = None;
                node.facts.stack = ItemStackClass::Stackable {
                    proven_maximum: Some(100),
                };
                node.item.quantity = 1 + rng.below(100) as u32;
            }
        }
        let before: Vec<_> = fixture.nodes.iter().map(|node| node.item.clone()).collect();
        let expected: u128 = fixture
            .nodes
            .iter()
            .map(|node| u128::from(node.facts.weight.unwrap_or(0)) * u128::from(node.item.quantity))
            .sum();
        let plan = fixture
            .plan()
            .map_err(|error| format!("valid tree: {error:?}"))?;
        assert_eq!(u128::from(plan.weight_centi_oz), expected);
        assert_eq!(plan.sorted_item_ids.len(), count);
        assert_eq!(plan.inbox_items_after, count as u64);
        assert!(
            plan.sorted_item_ids
                .windows(2)
                .all(|pair| pair[0] < pair[1])
        );
        assert_eq!(
            before,
            fixture
                .nodes
                .iter()
                .map(|node| node.item.clone())
                .collect::<Vec<_>>()
        );
        // Input row order is irrelevant to both address selection and totals.
        let rotation = rng.below(count as u64) as usize;
        fixture.nodes.rotate_left(rotation);
        assert_eq!(fixture.plan(), Ok(plan));
        Ok(())
    });
}

#[test]
fn mail_property_inbox_addition_never_wraps_and_counts_the_whole_tree() {
    for_each_case("mail inbox arithmetic", |rng, _| {
        let count = *rng.pick(&[2, 10, PARCEL_ITEMS_MAX]);
        let mut fixture = tree(count);
        for before in [
            0,
            MAIL_INBOX_ITEMS_MAX - count as u64,
            MAIL_INBOX_ITEMS_MAX,
            u64::MAX - count as u64,
            u64::MAX,
            rng.u64(),
        ] {
            fixture.inbox = before;
            let wide = u128::from(before) + count as u128;
            let expected = if wide <= u128::from(MAIL_INBOX_ITEMS_MAX) {
                Ok(wide as u64)
            } else {
                Err(MailRuleRefusal::RecipientInboxFull)
            };
            assert_eq!(fixture.plan().map(|plan| plan.inbox_items_after), expected);
        }
        Ok(())
    });
}

#[test]
fn mail_tree_item_limit_is_inclusive() {
    assert!(tree(PARCEL_ITEMS_MAX).plan().is_ok());
    assert_eq!(
        tree(PARCEL_ITEMS_MAX + 1).plan(),
        Err(MailRuleRefusal::MalformedTree)
    );
}
