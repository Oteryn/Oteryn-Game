#![allow(clippy::expect_used)]
use super::*;
use crate::chat::{ChatEgress, ChatText, EGRESS_MAX_LINES};
#[test]
fn guild_room_lines_retain_destination_and_shed_with_rooms() {
    let mut id = [1; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    let guild = GuildId::from_bytes(id).expect("valid fixture");
    let line = guild_line(
        guild,
        GuildRank::LEADER,
        "Knight",
        &ChatText::parse("hello").expect("valid fixture"),
    );
    let mut queue = ChatEgress::new();
    queue.push(line.clone());
    for _ in 1..EGRESS_MAX_LINES {
        queue.push(ChatLine::Private {
            speaker_name: "Knight".into(),
            text: "private".into(),
        });
    }
    queue.push(line);
    assert_eq!(queue.len(), EGRESS_MAX_LINES);
    assert_eq!(queue.pop(), Some(ChatLine::Dropped));
    assert!(matches!(queue.pop(), Some(ChatLine::Private { .. })));
}
