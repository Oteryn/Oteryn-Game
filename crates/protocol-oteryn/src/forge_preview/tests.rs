use super::*;
#[test]
fn forge_preview_strict_readonly_roundtrip_and_bounds() -> Result<(), &'static str> {
    let first = ItemHandle::new(1).ok_or("first")?;
    let second = ItemHandle::new(2).ok_or("second")?;
    let request = Request {
        nonce: 1,
        selection: Selection::Pair {
            operation: Operation::Fusion,
            first,
            second,
            boost: true,
            protect: true,
        },
    };
    let bytes = request.encode().ok_or("request")?;
    assert_eq!(bytes.len(), MAX_REQUEST_BYTES);
    assert_eq!(Request::decode(&bytes), Some(request));
    let mut invalid = bytes.clone();
    invalid[10] = 1;
    assert!(
        Request::decode(&invalid).is_none(),
        "readonly occurrence stays zero"
    );
    assert!(
        Request {
            nonce: 1,
            selection: Selection::Pair {
                operation: Operation::Fusion,
                first,
                second: first,
                boost: false,
                protect: false
            }
        }
        .encode()
        .is_none()
    );
    let item = Item {
        handle: first,
        definition_ref: 7,
        appearance: 3264,
        name: "Sword".into(),
        classification: 4,
        tier: 1,
        maximum_tier: 10,
        container: false,
    };
    let reply = Reply {
        nonce: 1,
        payload: Payload::Page(Page {
            after: 0,
            next: None,
            resources: Resources {
                dust: 20,
                dust_limit: 100,
                ..Default::default()
            },
            items: vec![item],
        }),
    };
    let bytes = reply.encode().ok_or("reply")?;
    assert_eq!(Reply::decode(&bytes), Some(reply));
    for n in 0..bytes.len() {
        assert!(Reply::decode(&bytes[..n]).is_none());
    }
    let mut trailing = bytes;
    trailing.push(0);
    assert!(Reply::decode(&trailing).is_none());
    assert!(Reply::decode(&[0; MAX_REPLY_BYTES + 1]).is_none());
    Ok(())
}

#[test]
fn forge_preview_quote_correlates_pair_and_rejects_noncanonical_options() -> Result<(), &'static str>
{
    let first = ItemHandle::new(1).ok_or("first")?;
    let second = ItemHandle::new(2).ok_or("second")?;
    let selection = Selection::Pair {
        operation: Operation::Fusion,
        first,
        second,
        boost: true,
        protect: true,
    };
    let item = |handle| Item {
        handle,
        definition_ref: 7,
        appearance: 3264,
        name: "Sword".into(),
        classification: 1,
        tier: 0,
        maximum_tier: 1,
        container: false,
    };
    let quote = Quote {
        selection,
        first: item(first),
        second: item(second),
        resources: Resources {
            dust: 100,
            dust_limit: 100,
            cores: 2,
            ..Default::default()
        },
        cost: Cost {
            gold: 25000,
            dust: 100,
            cores: 2,
        },
        target_tier: 1,
        success_percent: 65,
        protection_percent: 50,
    };
    let reply = Reply {
        nonce: 7,
        payload: Payload::Quote(quote.clone()),
    };
    let bytes = reply.encode().ok_or("quote")?;
    assert_eq!(Reply::decode(&bytes), Some(reply));
    let mut wrong = quote;
    wrong.second.handle = first;
    assert!(
        Reply {
            nonce: 7,
            payload: Payload::Quote(wrong)
        }
        .encode()
        .is_none()
    );
    assert!(
        Request {
            nonce: 7,
            selection: Selection::Pair {
                operation: Operation::Transfer,
                first,
                second,
                boost: true,
                protect: false
            }
        }
        .encode()
        .is_none()
    );
    Ok(())
}
