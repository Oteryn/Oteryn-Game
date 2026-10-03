//! PREM-1b: the Premium snapshot client against the in-process mutual-TLS test producer
//! (PREMIUM-DELIVERY-0 §3, §3.1; §11 scope items 1, 2 and 5). Every response that is not a 200
//! `application/json` body within the size cap is a failed pull, and a body bound to another
//! request or account fails validation: nothing reaches the fence.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

#[allow(dead_code)]
#[path = "../src/premium/test_producer.rs"]
mod test_producer;

use oteryn_game_server::premium::client::{
    PULL_TIMEOUT, PremiumClientConfig, PremiumSnapshotClient, PullFailure, fresh_nonce,
};
use oteryn_game_server::premium::snapshot::{SnapshotRejection, canonical_uuid, validate};
use std::time::{Duration, Instant};
use test_producer::{ProducerRequest, Reply, TestPki, TestProducer, snapshot};

const ACCOUNT: [u8; 16] = [0x61; 16];

fn block_on<T>(future: impl Future<Output = T>) -> T {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap()
        .block_on(future)
}

fn client(pki: &TestPki, origin: &str) -> PremiumSnapshotClient {
    PremiumSnapshotClient::new(&PremiumClientConfig {
        origin: origin.to_owned(),
        identity_pem: pki.client_identity_pem.clone().into_bytes(),
        platform_ca_pem: pki.ca_pem.clone().into_bytes(),
    })
    .unwrap()
}

/// Pull once with a fresh nonce against a producer answering with `reply`.
async fn pull_with(
    reply: impl Fn(&ProducerRequest) -> Reply + Send + Sync + 'static,
) -> (Result<Vec<u8>, PullFailure>, String, TestProducer) {
    let pki = TestPki::new();
    let producer = TestProducer::start(&pki, reply).await;
    let nonce = fresh_nonce().unwrap();
    let result = client(&pki, &producer.origin).pull(ACCOUNT, &nonce).await;
    (result, nonce, producer)
}

#[test]
fn a_bound_200_snapshot_is_returned_for_validation() {
    block_on(async {
        let (body, nonce, producer) =
            pull_with(|request| Reply::json(snapshot(request, 7, &[]))).await;
        let evidence = validate(&body.unwrap(), ACCOUNT, &nonce).unwrap();
        assert_eq!(evidence.authority_revision, 7);
        // The producer saw exactly the §3.1 request: the canonical AccountId and the nonce.
        assert_eq!(
            producer.requests(),
            vec![ProducerRequest {
                account_id: canonical_uuid(ACCOUNT),
                nonce,
            }]
        );
    });
}

#[test]
fn a_wrong_nonce_or_account_fails_closed() {
    block_on(async {
        let (body, nonce, _producer) = pull_with(|request| {
            Reply::json(snapshot(request, 7, &[("nonce", "0".repeat(32).into())]))
        })
        .await;
        let rejection = validate(&body.unwrap(), ACCOUNT, &nonce).unwrap_err();
        assert_eq!(rejection.rejection(), SnapshotRejection::Malformed);
        let other = canonical_uuid([0x62; 16]);
        let (body, nonce, _producer) = pull_with(move |request| {
            Reply::json(snapshot(
                request,
                7,
                &[("account_id", other.clone().into())],
            ))
        })
        .await;
        let rejection = validate(&body.unwrap(), ACCOUNT, &nonce).unwrap_err();
        assert_eq!(rejection.rejection(), SnapshotRejection::Malformed);
    });
}

#[test]
fn oversize_malformed_or_mistyped_responses_fail_closed() {
    block_on(async {
        let (result, ..) = pull_with(|_| Reply::json(vec![b' '; 1025])).await;
        assert_eq!(result, Err(PullFailure::Oversize));
        let (body, nonce, _producer) = pull_with(|_| Reply::json(b"{\"schema\":".to_vec())).await;
        let rejection = validate(&body.unwrap(), ACCOUNT, &nonce).unwrap_err();
        assert_eq!(rejection.rejection(), SnapshotRejection::Malformed);
        let (result, ..) = pull_with(|request| {
            let mut reply = Reply::json(snapshot(request, 7, &[]));
            reply.headers = vec![("Content-Type".into(), "text/plain".into())];
            reply
        })
        .await;
        assert_eq!(result, Err(PullFailure::ContentType));
        let (result, ..) = pull_with(|request| {
            let mut reply = Reply::json(snapshot(request, 7, &[]));
            reply.headers.clear();
            reply
        })
        .await;
        assert_eq!(result, Err(PullFailure::ContentType));
    });
}

#[test]
fn a_redirect_is_never_followed() {
    block_on(async {
        let (result, _, producer) = pull_with(|_| {
            Reply::status(302).header("Location", "https://127.0.0.1:1/v1/premium/snapshot")
        })
        .await;
        assert_eq!(
            result,
            Err(PullFailure::Status {
                status: 302,
                retry_after: None
            })
        );
        assert_eq!(producer.requests().len(), 1);
    });
}

#[test]
fn errors_fail_closed_and_carry_retry_after() {
    block_on(async {
        let (result, ..) = pull_with(|_| Reply::status(500).header("Retry-After", "9")).await;
        assert_eq!(
            result,
            Err(PullFailure::Status {
                status: 500,
                retry_after: None
            })
        );
        for status in [429, 503] {
            let (result, ..) =
                pull_with(move |_| Reply::status(status).header("Retry-After", "7")).await;
            assert_eq!(
                result.unwrap_err().retry_after(),
                Some(Duration::from_secs(7))
            );
        }
    });
}

#[test]
fn a_slow_producer_times_out_after_five_seconds() {
    block_on(async {
        let started = Instant::now();
        let (result, ..) = pull_with(|request| {
            Reply::json(snapshot(request, 7, &[])).delayed(Duration::from_secs(8))
        })
        .await;
        assert_eq!(result, Err(PullFailure::Timeout));
        assert!(started.elapsed() < PULL_TIMEOUT + Duration::from_secs(2));
    });
}

#[test]
fn an_untrusted_server_or_client_gets_no_evidence() {
    block_on(async {
        let (pki, other) = (TestPki::new(), TestPki::new());
        let producer =
            TestProducer::start(&pki, |request| Reply::json(snapshot(request, 7, &[]))).await;
        // The server's certificate is not issued by the configured Platform CA.
        let untrusted_server = PremiumSnapshotClient::new(&PremiumClientConfig {
            origin: producer.origin.clone(),
            identity_pem: pki.client_identity_pem.clone().into_bytes(),
            platform_ca_pem: other.ca_pem.clone().into_bytes(),
        })
        .unwrap();
        let nonce = fresh_nonce().unwrap();
        assert_eq!(
            untrusted_server.pull(ACCOUNT, &nonce).await,
            Err(PullFailure::Transport)
        );
        // The client's identity is not issued by the producer's CA: the handshake fails.
        let untrusted_client = PremiumSnapshotClient::new(&PremiumClientConfig {
            origin: producer.origin.clone(),
            identity_pem: other.client_identity_pem.clone().into_bytes(),
            platform_ca_pem: pki.ca_pem.clone().into_bytes(),
        })
        .unwrap();
        assert_eq!(
            untrusted_client.pull(ACCOUNT, &nonce).await,
            Err(PullFailure::Transport)
        );
        assert!(producer.requests().is_empty());
    });
}

#[test]
fn only_an_https_origin_is_configured() {
    let pki = TestPki::new();
    for origin in [
        "http://127.0.0.1:8443",
        "https://127.0.0.1:8443/other",
        "https://user@127.0.0.1:8443",
        "not a url",
    ] {
        let config = PremiumClientConfig {
            origin: origin.to_owned(),
            identity_pem: pki.client_identity_pem.clone().into_bytes(),
            platform_ca_pem: pki.ca_pem.clone().into_bytes(),
        };
        assert!(PremiumSnapshotClient::new(&config).is_err(), "{origin}");
    }
    let no_ca = PremiumClientConfig {
        origin: "https://127.0.0.1:8443".into(),
        identity_pem: pki.client_identity_pem.clone().into_bytes(),
        platform_ca_pem: Vec::new(),
    };
    assert!(PremiumSnapshotClient::new(&no_ca).is_err());
}
