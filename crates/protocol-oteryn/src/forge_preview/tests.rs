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
#[test]
fn forge_resource_quote_is_additive_strict_and_does_not_change_original_wire()
-> Result<(), &'static str> {
    for operation in [
        Operation::DustToSlivers,
        Operation::SliversToCores,
        Operation::DustLimit,
    ] {
        let request = Request {
            nonce: 9,
            selection: Selection::Resource(operation),
        };
        let bytes = request.encode().ok_or("request")?;
        assert_eq!(bytes.len(), 10);
        assert_eq!(Request::decode(&bytes), Some(request));
    }
    let before = Resources {
        dust: 100,
        dust_limit: 100,
        slivers: 50,
        cores: 2,
        ..Default::default()
    };
    let quote = ResourceQuote {
        operation: Operation::DustToSlivers,
        full_resources: true,
        before,
        after_dust: 40,
        after_limit: 100,
        after_slivers: 53,
        after_cores: 2,
    };
    let reply = Reply {
        nonce: 9,
        payload: Payload::Resource(quote),
    };
    let bytes = reply.encode().ok_or("quote")?;
    assert_eq!(Reply::decode(&bytes), Some(reply));
    for n in 0..bytes.len() {
        assert!(Reply::decode(&bytes[..n]).is_none());
    }
    let mut extra = bytes;
    extra.push(0);
    assert!(Reply::decode(&extra).is_none());
    let limit = Reply {
        nonce: 10,
        payload: Payload::Resource(ResourceQuote {
            operation: Operation::DustLimit,
            full_resources: false,
            before: Resources {
                dust: 100,
                dust_limit: 100,
                ..Default::default()
            },
            after_dust: 75,
            after_limit: 120,
            after_slivers: 0,
            after_cores: 0,
        }),
    };
    assert_eq!(Reply::decode(&limit.encode().ok_or("limit")?), Some(limit));
    let browse = Request {
        nonce: 9,
        selection: Selection::Browse { after: 0 },
    };
    let mut expected = 9_u64.to_le_bytes().to_vec();
    expected.push(0);
    expected.extend(0_u64.to_le_bytes());
    assert_eq!(browse.encode(), Some(expected));
    assert!(
        Request {
            nonce: 9,
            selection: Selection::Resource(Operation::Fusion)
        }
        .encode()
        .is_none()
    );
    assert!(Request::decode(&[9, 0, 0, 0, 0, 0, 0, 0, 2, 8]).is_none());
    assert!(
        Reply {
            nonce: 9,
            payload: Payload::Resource(ResourceQuote {
                after_dust: 100,
                ..quote
            })
        }
        .encode()
        .is_none()
    );
    Ok(())
}
