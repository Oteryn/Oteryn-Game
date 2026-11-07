#![allow(clippy::expect_used)]

use super::*;
use oteryn_gameplay_domain::domain::item_definition::TypedDefinitionRef;

#[derive(Clone)]
pub(super) struct Node {
    pub item: InventoryItem,
    pub facts: ItemDefinitionFacts,
    pub parent: Option<[u8; 16]>,
    pub ordinal: u64,
    pub text: Option<MailText>,
}

impl Node {
    pub(super) fn new(id: u128, key: &str, parent: Option<[u8; 16]>) -> Self {
        let definition = TypedDefinitionRef {
            family: "Item".into(),
            production_key: key.into(),
            revision_ref: "definition-r1".into(),
        };
        Self {
            item: InventoryItem {
                item_instance_id: id.to_be_bytes(),
                definition: definition.clone(),
                quantity: 1,
            },
            facts: ItemDefinitionFacts {
                definition,
                weight: Some(match key {
                    PARCEL_KEY | STAMPED_PARCEL_KEY => 1_800,
                    LETTER_KEY | STAMPED_LETTER_KEY => 50,
                    LABEL_KEY => 10,
                    _ => 100,
                }),
                stack: ItemStackClass::NonStackable,
                container_capacity: match key {
                    PARCEL_KEY | STAMPED_PARCEL_KEY => Some(10),
                    "bag" => Some(20),
                    _ => None,
                },
                container_slot_equip_pattern: false,
            },
            parent,
            ordinal: id as u64,
            text: None,
        }
    }

    fn text(mut self, text: &str) -> Self {
        self.text =
            Some(MailText::parse(MailTextClass::Letter, text).expect("bounded fixture text"));
        self
    }
}

#[derive(Clone)]
pub(super) struct Fixture {
    pub nodes: Vec<Node>,
    pub store_owner: bool,
    pub sender_junior: bool,
    pub recipient_junior: bool,
    pub inbox: u64,
}

impl Fixture {
    pub(super) fn letter() -> Self {
        Self {
            nodes: vec![Node::new(1, LETTER_KEY, None).text("Aldric\nThais\nprivate body")],
            store_owner: false,
            sender_junior: false,
            recipient_junior: false,
            inbox: 0,
        }
    }

    pub(super) fn parcel() -> Self {
        Self {
            nodes: vec![
                Node::new(1, PARCEL_KEY, None),
                Node::new(2, LABEL_KEY, Some(1_u128.to_be_bytes())).text("Aldric\nThais"),
            ],
            ..Self::letter()
        }
    }

    pub(super) fn plan(&self) -> Result<MailPostingPlan, MailRuleRefusal> {
        let nodes: Vec<_> = self
            .nodes
            .iter()
            .map(|node| MailTreeItem {
                item: &node.item,
                facts: &node.facts,
                parent: node.parent,
                ordinal: node.ordinal,
                text: node.text.as_ref(),
            })
            .collect();
        MailPostingPlan::try_from(MailPostingInput {
            root: 1_u128.to_be_bytes(),
            items: &nodes,
            root_has_store_owner: self.store_owner,
            sender_junior: self.sender_junior,
            recipient_junior: self.recipient_junior,
            inbox_items: self.inbox,
        })
    }
}

// Reference parity rows R1–R10/R13 in MAIL-1.md. Evidence, not copied engine code:
// https://github.com/opentibiabr/canary/blob/04b83b512114bfd888000d6e1433ed8ecaec7c5b/src/items/containers/mailbox/mailbox.cpp
// https://github.com/opentibiabr/canary/blob/04b83b512114bfd888000d6e1433ed8ecaec7c5b/data/items/items.xml
// Weight includes contents: same commit, src/items/containers/container.cpp,
// Container::getWeight/updateItemWeight; quantities use existing stack rules.
#[test]
fn mail_reference_parity_table() {
    let mut stamped_parcel = Fixture::parcel();
    stamped_parcel.nodes[0] = Node::new(1, STAMPED_PARCEL_KEY, None);
    let mut stamped_letter = Fixture::letter();
    stamped_letter.nodes[0] = Node::new(1, STAMPED_LETTER_KEY, None).text("Aldric");
    let mut store = Fixture::letter();
    store.store_owner = true;
    let mut ten = Fixture::parcel();
    for id in 3..=11 {
        ten.nodes
            .push(Node::new(id, "stone", Some(1_u128.to_be_bytes())));
    }
    let mut eleven = ten.clone();
    eleven
        .nodes
        .push(Node::new(12, "stone", Some(1_u128.to_be_bytes())));
    let mut nested_label = Fixture::parcel();
    nested_label.nodes[1].parent = Some(3_u128.to_be_bytes());
    nested_label
        .nodes
        .push(Node::new(3, "bag", Some(1_u128.to_be_bytes())));
    let mut town = Fixture::letter();
    town.nodes[0] = Node::new(1, LETTER_KEY, None).text("  Al Dric  \nnot a town 🦀\nbody");
    let mut first = Fixture::parcel();
    first.nodes[1] = Node::new(2, LABEL_KEY, Some(1_u128.to_be_bytes())).text("Beatrice");
    first
        .nodes
        .push(Node::new(3, LABEL_KEY, Some(1_u128.to_be_bytes())).text("Aldric"));
    first.nodes.swap(1, 2); // Input enumeration never overrides entry order.
    let mut empty = Fixture::parcel();
    empty.nodes[1] = Node::new(2, LABEL_KEY, Some(1_u128.to_be_bytes())).text("");
    empty
        .nodes
        .push(Node::new(3, LABEL_KEY, Some(1_u128.to_be_bytes())).text("Aldric"));
    let mut blank = empty.clone();
    blank.nodes[1] = Node::new(2, LABEL_KEY, Some(1_u128.to_be_bytes())).text("\nAldric");
    let mut heavy = Fixture::parcel();
    heavy.nodes[0].facts.weight = Some(u32::MAX);
    let mut contents = Fixture::parcel();
    contents
        .nodes
        .push(Node::new(3, "bag", Some(1_u128.to_be_bytes())));
    contents
        .nodes
        .push(Node::new(4, "stone", Some(3_u128.to_be_bytes())));

    // Each tuple is one independently asserted Reference rule, with positive
    // and refusal controls collected into that row where appropriate.
    let rows = [
        (
            "R1 letter/stamp",
            vec![
                (Fixture::letter(), Ok((STAMPED_LETTER_KEY, "aldric", 50))),
                (stamped_letter, Err(MailRuleRefusal::NotMailable)),
            ],
        ),
        (
            "R2 parcel/stamp",
            vec![
                (Fixture::parcel(), Ok((STAMPED_PARCEL_KEY, "aldric", 1_810))),
                (stamped_parcel, Err(MailRuleRefusal::NotMailable)),
            ],
        ),
        (
            "R3 Store owner",
            vec![(store, Err(MailRuleRefusal::NotMailable))],
        ),
        (
            "R4 parcel direct capacity",
            vec![
                (ten, Ok((STAMPED_PARCEL_KEY, "aldric", 2_710))),
                (eleven, Err(MailRuleRefusal::MalformedTree)),
            ],
        ),
        (
            "R5 first line; ignore town",
            vec![(town, Ok((STAMPED_LETTER_KEY, "aldric", 50)))],
        ),
        (
            "R6 direct labels only",
            vec![(nested_label, Err(MailRuleRefusal::NoAddress))],
        ),
        (
            "R7 first label in entry order",
            vec![(first, Ok((STAMPED_PARCEL_KEY, "beatrice", 1_820)))],
        ),
        (
            "R8 empty label skipped",
            vec![(empty, Ok((STAMPED_PARCEL_KEY, "aldric", 1_820)))],
        ),
        (
            "R9 nonempty blank first line stops search",
            vec![(blank, Err(MailRuleRefusal::NoAddress))],
        ),
        (
            "R10 no postal weight cap",
            vec![(
                heavy,
                Ok((STAMPED_PARCEL_KEY, "aldric", u64::from(u32::MAX) + 10)),
            )],
        ),
        (
            "R13 nested bags retain contents",
            vec![(contents, Ok((STAMPED_PARCEL_KEY, "aldric", 2_010)))],
        ),
    ];
    for (rule, controls) in rows {
        for (fixture, expected) in controls {
            let before: Vec<_> = fixture.nodes.iter().map(|node| node.item.clone()).collect();
            let result = fixture.plan().map(|plan| {
                let mut ids: Vec<_> = before.iter().map(|item| item.item_instance_id).collect();
                ids.sort();
                assert_eq!(plan.sorted_item_ids, ids, "{rule}: identity conservation");
                assert_eq!(plan.root, before[0].item_instance_id);
                assert_eq!(plan.inbox_items_after, before.len() as u64);
                (
                    plan.stamped_production_key,
                    plan.recipient.comparison_key(),
                    plan.weight_centi_oz,
                )
            });
            assert_eq!(
                result,
                expected.map(|(key, name, weight)| (key, name.to_owned(), weight)),
                "{rule}"
            );
            assert_eq!(
                before,
                fixture
                    .nodes
                    .iter()
                    .map(|node| node.item.clone())
                    .collect::<Vec<_>>(),
                "{rule}: quantities/definitions are unchanged"
            );
        }
    }
}

// R11/R12: same pinned Canary items.xml, maxtextlen=1999/79. The L0
// bounded value remains the single text parser, including scalar/byte safety.
#[test]
fn mail_reference_text_limits_table() {
    for (rule, class, maximum) in [
        ("R11 letter text", MailTextClass::Letter, 1_999),
        ("R12 label text", MailTextClass::Label, 79),
    ] {
        for character in ['a', '🦀'] {
            for count in [maximum, maximum + 1] {
                let value = character.to_string().repeat(count);
                assert_eq!(
                    MailText::parse(class, &value).is_ok(),
                    count <= maximum,
                    "{rule}"
                );
            }
        }
    }
    let mut fixture = Fixture::parcel();
    // A Letter-class MailText attached to a label cannot bypass the label cap.
    fixture.nodes[1] = Node::new(2, LABEL_KEY, Some(1_u128.to_be_bytes())).text(&"a".repeat(80));
    assert_eq!(
        fixture.plan(),
        Err(MailRuleRefusal::InvalidText(MailTextRefusal::TooLong))
    );
}

#[test]
fn mail_juniors_and_inbox_ceiling_are_owner_differences() {
    for sender_junior in [false, true] {
        for recipient_junior in [false, true] {
            let mut letter = Fixture::letter();
            letter.sender_junior = sender_junior;
            letter.recipient_junior = recipient_junior;
            assert!(letter.plan().is_ok());
            let parcel = Fixture {
                sender_junior,
                recipient_junior,
                ..Fixture::parcel()
            };
            let expected = if sender_junior {
                Err(MailRuleRefusal::NotMailable)
            } else if recipient_junior {
                Err(MailRuleRefusal::RecipientCannotReceiveParcels)
            } else {
                Ok(())
            };
            assert_eq!(parcel.plan().map(|_| ()), expected);
        }
    }
    for mut fixture in [Fixture::letter(), Fixture::parcel()] {
        fixture.inbox = MAIL_INBOX_ITEMS_MAX - fixture.nodes.len() as u64;
        assert_eq!(
            fixture.plan().map(|plan| plan.inbox_items_after),
            Ok(MAIL_INBOX_ITEMS_MAX)
        );
        for before in [fixture.inbox + 1, MAIL_INBOX_ITEMS_MAX, u64::MAX] {
            fixture.inbox = before;
            assert_eq!(fixture.plan(), Err(MailRuleRefusal::RecipientInboxFull));
        }
    }
}

#[test]
fn mail_rejects_disconnected_cyclic_duplicate_and_overdeep_trees() {
    let base = Fixture::parcel();
    let mut variants = Vec::new();
    let mut empty = base.clone();
    empty.nodes.clear();
    variants.push(empty);
    let mut missing_root = base.clone();
    missing_root.nodes.remove(0);
    variants.push(missing_root);
    let mut duplicate = base.clone();
    duplicate.nodes.push(duplicate.nodes[1].clone());
    variants.push(duplicate);
    for parent in [
        None,
        Some(99_u128.to_be_bytes()),
        Some(2_u128.to_be_bytes()),
    ] {
        let mut fixture = base.clone();
        fixture.nodes[1].parent = parent;
        variants.push(fixture);
    }
    let mut root_parent = base.clone();
    root_parent.nodes[0].parent = Some(2_u128.to_be_bytes());
    variants.push(root_parent);
    let mut slot = base.clone();
    slot.nodes
        .push(Node::new(3, "stone", Some(1_u128.to_be_bytes())));
    slot.nodes[2].ordinal = slot.nodes[1].ordinal;
    variants.push(slot);
    let mut zero = base.clone();
    zero.nodes[1].ordinal = 0;
    variants.push(zero);
    let mut noncontainer = base.clone();
    noncontainer
        .nodes
        .push(Node::new(3, "stone", Some(2_u128.to_be_bytes())));
    variants.push(noncontainer);
    for fixture in variants {
        assert_eq!(fixture.plan(), Err(MailRuleRefusal::MalformedTree));
    }
    let mut chain = base;
    for id in 3..=9 {
        let parent = if id == 3 { 1 } else { id - 1 };
        chain
            .nodes
            .push(Node::new(id, "bag", Some(parent.to_be_bytes())));
    }
    assert!(chain.plan().is_ok(), "height seven is inclusive");
    chain
        .nodes
        .push(Node::new(10, "stone", Some(9_u128.to_be_bytes())));
    assert_eq!(chain.plan(), Err(MailRuleRefusal::MalformedTree));
}

#[test]
fn mail_requires_exact_definitions_known_weights_and_valid_quantities() {
    let base = Fixture::parcel();
    let mut mismatch = base.clone();
    mismatch.nodes[1].facts.definition.revision_ref = "other".into();
    assert_eq!(mismatch.plan(), Err(MailRuleRefusal::DefinitionMismatch));
    for quantity in [0, 2, u32::MAX] {
        let mut fixture = base.clone();
        fixture.nodes[1].item.quantity = quantity;
        assert_eq!(fixture.plan(), Err(MailRuleRefusal::InvalidQuantity));
    }
    for index in 0..base.nodes.len() {
        let mut fixture = base.clone();
        fixture.nodes[index].facts.weight = None;
        assert_eq!(fixture.plan(), Err(MailRuleRefusal::WeightUnknown));
    }
    let mut unknown = base.clone();
    unknown.nodes[1].facts.stack = ItemStackClass::Unknown;
    assert_eq!(unknown.plan(), Err(MailRuleRefusal::InvalidQuantity));
    let mut stamped = base;
    stamped.nodes[0].facts.container_capacity = Some(20);
    assert_eq!(stamped.plan(), Err(MailRuleRefusal::NotMailable));
}

#[test]
fn mail_addresses_and_diagnostics_never_leak_player_text() {
    for (text, refusal) in [
        ("", MailRuleRefusal::NoAddress),
        ("\nAldric", MailRuleRefusal::NoAddress),
        ("Invalid1", MailRuleRefusal::UnknownRecipient),
    ] {
        let mut fixture = Fixture::letter();
        fixture.nodes[0] = Node::new(1, LETTER_KEY, None).text(text);
        assert_eq!(fixture.plan(), Err(refusal));
    }
    let fixture = Fixture::letter();
    let plan = fixture.plan().expect("letter plan");
    let debug = format!("{plan:?}");
    assert!(!debug.contains("Aldric"));
    assert!(!debug.contains("private body"));
}
