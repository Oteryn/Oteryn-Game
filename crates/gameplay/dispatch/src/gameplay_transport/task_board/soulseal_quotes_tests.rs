use super::*;
use crate::foundation::{
    ChannelId, ConnectionGeneration, GameSessionId, RuntimeScopeRefV1, ScopeOwnershipGeneration,
    WorldId,
};
#[test]
fn t114_authorization_is_one_use_and_binds_every_live_fence_and_content()
-> Result<(), Box<dyn std::error::Error>> {
    let bytes = |n: u8| {
        let mut b = [n; 16];
        b[6] = 0x70;
        b[8] = 0x80;
        b
    };
    let fence = CurrentCharacterItemFence {
        character_id: crate::domain::CharacterId::from_bytes(bytes(1))
            .map_err(|e| format!("{e:?}"))?,
        game_session_id: GameSessionId::decode(&bytes(2))?,
        connection_generation: ConnectionGeneration::new(1)?,
        character_lease_generation: 1,
        runtime_scope: RuntimeScopeRefV1::channel(
            WorldId::decode(&bytes(3))?,
            ChannelId::decode(&bytes(4))?,
        ),
        scope_ownership_generation: ScopeOwnershipGeneration::new(1)?,
    };
    let key = *fence.game_session_id.as_bytes();
    let token = bytes(5);
    let content = [6; 32];
    let quote = Quote {
        token,
        expires: 100,
        fence,
        content,
    };
    let quotes = Quotes::default();
    let issue = || {
        quotes
            .issue(key, quote.clone(), 1)
            .map_err(|_| "quote store")
    };
    assert!(
        !quotes.consume(key, token, fence, content, 1),
        "missing quote"
    );
    issue()?;
    assert!(quotes.consume(key, token, fence, content, 99));
    assert!(!quotes.consume(key, token, fence, content, 99), "one use");
    issue()?;
    assert!(
        !quotes.consume(key, token, fence, content, 100),
        "deadline is exclusive"
    );
    issue()?;
    assert!(
        !quotes.consume(key, bytes(7), fence, content, 1),
        "wrong authorization"
    );
    issue()?;
    assert!(
        !quotes.consume(key, token, fence, [7; 32], 1),
        "content replacement"
    );
    let changed = [
        CurrentCharacterItemFence {
            character_id: crate::domain::CharacterId::from_bytes(bytes(8))
                .map_err(|e| format!("{e:?}"))?,
            ..fence
        },
        CurrentCharacterItemFence {
            game_session_id: GameSessionId::decode(&bytes(8))?,
            ..fence
        },
        CurrentCharacterItemFence {
            connection_generation: ConnectionGeneration::new(2)?,
            ..fence
        },
        CurrentCharacterItemFence {
            character_lease_generation: 2,
            ..fence
        },
        CurrentCharacterItemFence {
            scope_ownership_generation: ScopeOwnershipGeneration::new(2)?,
            ..fence
        },
        CurrentCharacterItemFence {
            runtime_scope: RuntimeScopeRefV1::channel(
                WorldId::decode(&bytes(8))?,
                ChannelId::decode(&bytes(4))?,
            ),
            ..fence
        },
        CurrentCharacterItemFence {
            runtime_scope: RuntimeScopeRefV1::channel(
                WorldId::decode(&bytes(3))?,
                ChannelId::decode(&bytes(8))?,
            ),
            ..fence
        },
    ];
    for current in changed {
        issue()?;
        assert!(!quotes.consume(key, token, current, content, 1));
    }
    issue()?;
    quotes.revoke(key).map_err(|_| "revoke")?;
    assert!(
        !quotes.consume(key, token, fence, content, 1),
        "unready refresh revokes"
    );
    issue()?;
    let replacement = Quote {
        token: bytes(9),
        ..quote
    };
    quotes
        .issue(key, replacement, 1)
        .map_err(|_| "replacement")?;
    assert!(
        !quotes.consume(key, token, fence, content, 1),
        "refresh replaces authorization"
    );
    Ok(())
}
