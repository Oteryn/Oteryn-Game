#![allow(dead_code)]
#[path = "../src/admission_evidence.rs"]
mod admission_evidence;
#[path = "../src/native_admission_source/mod.rs"]
mod native_admission_source;

use admission_evidence::{Facts, Failure, Observation, Request, Response};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use native_admission_source::{SourceError, TransientCapacity, descriptor::ProducerDescriptor};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use std::{
    env, fs, io,
    sync::{Arc, Barrier},
    thread,
};

const ADMISSION_GAME_MAIN: &str = "cf5c5f35476559450b6bbaf87dce519f7eead9d0";
const PLATFORM_SOURCE: &str = "623435ec1b907d6d9770b767806c90300252a71c";
const EXPECTED_AUTHORITY: &str = "platform";

fn required(name: &str) -> Result<String, io::Error> {
    env::var(name).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, name))
}

fn seeded_key(name: &str) -> Result<[u8; 32], Box<dyn std::error::Error>> {
    Ok([required(name)?.parse::<u8>()?; 32])
}

fn pem_der(path: &str, label: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let text = fs::read_to_string(path)?;
    let begin = format!("-----BEGIN {label}-----");
    let end = format!("-----END {label}-----");
    let body = text
        .split_once(&begin)
        .and_then(|(_, tail)| tail.split_once(&end).map(|(body, _)| body))
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid PEM boundary"))?;
    Ok(STANDARD.decode(body.lines().map(str::trim).collect::<String>())?)
}

fn descriptor() -> Result<ProducerDescriptor, Box<dyn std::error::Error>> {
    let port = required("WP5_S3A_PORT")?.parse::<u16>()?;
    let ca = pem_der(&required("WP5_S3A_CA_CERT")?, "CERTIFICATE")?;
    let client = pem_der(&required("WP5_S3A_CLIENT_CERT")?, "CERTIFICATE")?;
    let key = pem_der(&required("WP5_S3A_CLIENT_KEY")?, "PRIVATE KEY")?;
    Ok(ProducerDescriptor::new(
        EXPECTED_AUTHORITY.into(),
        ("127.0.0.1".into(), port),
        "source.test".into(),
        "source.test".into(),
        vec![CertificateDer::from(ca)],
        vec![CertificateDer::from(client)],
        PrivateKeyDer::try_from(key)?,
    )?)
}

fn assert_observed(
    operation: &str,
    response: Response,
) -> Result<Observation, Box<dyn std::error::Error>> {
    let Response::Observed(observation) = response else {
        return Err(format!("{operation} did not return observed authority").into());
    };
    if observation.source_authority.as_str() != EXPECTED_AUTHORITY
        || observation.source_revision == 0
        || observation.decision_identity.as_str() != observation.source_revision.to_string()
        || observation.clock_uncertainty_seconds > 5
    {
        return Err(format!("{operation} returned invalid provenance").into());
    }
    println!(
        "S3_EVIDENCE operation={operation} class=observed revision={} decision={}",
        observation.source_revision,
        observation.decision_identity.as_str()
    );
    Ok(observation)
}

async fn query_one(
    descriptor: &ProducerDescriptor,
    operation: &str,
    request: Request<'_>,
) -> Result<Observation, Box<dyn std::error::Error>> {
    let capacity = TransientCapacity::new();
    let mut permit = capacity.try_queue()?;
    permit.try_activate()?;
    assert_observed(
        operation,
        native_admission_source::query(descriptor, &request, &mut permit).await?,
    )
}

#[test]
fn qualification_pins_are_closed() {
    assert_eq!(ADMISSION_GAME_MAIN.len(), 40);
    assert_eq!(PLATFORM_SOURCE.len(), 40);
    assert_eq!(native_admission_source::PIPELINE_SLOTS, 2);
    assert_eq!(native_admission_source::QUEUED_REQUESTS, 8);
}

#[test]
#[ignore = "requires the disposable S3-A TLS/FastCGI/MariaDB topology"]
fn real_platform_producer_decodes_all_four_operations() -> Result<(), Box<dyn std::error::Error>> {
    let descriptor = descriptor()?;
    let account_id = required("WP5_S3A_ACCOUNT_ID")?;
    let fresh_key = required("WP5_S3A_FRESH_KEY_ID")?;
    let recovery_key = required("WP5_S3A_RECOVERY_KEY_ID")?;
    let fresh_public_key = seeded_key("WP5_S3A_FRESH_KEY_BYTE")?;
    let recovery_public_key = seeded_key("WP5_S3A_RECOVERY_KEY_BYTE")?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async {
        let fresh_account = query_one(
            &descriptor,
            "ReadAccountSecurityV1",
            Request::Account {
                recovery: false,
                account_id: &account_id,
                purpose: "platform_security",
                scope: "fresh_admission",
            },
        )
        .await?;
        if !matches!(fresh_account.facts, Facts::Account { allowed: true, minimum_valid_generation } if minimum_valid_generation >= 1) {
            return Err("fresh account facts are not authoritative".into());
        }

        let fresh_trust = query_one(
            &descriptor,
            "ReadFreshSigningTrustV1",
            Request::Trust {
                recovery: false,
                key_id: &fresh_key,
                key_purpose: "fresh_admission",
            },
        )
        .await?;
        if !matches!(fresh_trust.facts, Facts::Trust { trusted: true, public_key } if public_key == fresh_public_key) {
            return Err("fresh trust facts are not authoritative".into());
        }

        let recovery_account = query_one(
            &descriptor,
            "ReadRecoveryAccountSecurityV2",
            Request::Account {
                recovery: true,
                account_id: &account_id,
                purpose: "platform_security",
                scope: "existing_actor_recovery",
            },
        )
        .await?;
        if !matches!(recovery_account.facts, Facts::Account { allowed: true, minimum_valid_generation } if minimum_valid_generation >= 1) {
            return Err("recovery account facts are not authoritative".into());
        }

        let recovery_trust = query_one(
            &descriptor,
            "ReadRecoverySigningTrustV2",
            Request::Trust {
                recovery: true,
                key_id: &recovery_key,
                key_purpose: "existing_actor_recovery",
            },
        )
        .await?;
        if !matches!(recovery_trust.facts, Facts::Trust { trusted: true, public_key } if public_key == recovery_public_key) {
            return Err("recovery trust facts are not authoritative".into());
        }
        Ok::<(), Box<dyn std::error::Error>>(())
    })?;
    Ok(())
}

#[test]
#[ignore = "requires the disposable S3-A TLS/FastCGI/MariaDB topology"]
fn real_platform_producer_reports_revoked_fresh_key() -> Result<(), Box<dyn std::error::Error>> {
    let descriptor = descriptor()?;
    let fresh_key = required("WP5_S3A_FRESH_KEY_ID")?;
    let fresh_public_key = seeded_key("WP5_S3A_FRESH_KEY_BYTE")?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async {
        let revoked = query_one(
            &descriptor,
            "ReadFreshSigningTrustV1/revoked",
            Request::Trust {
                recovery: false,
                key_id: &fresh_key,
                key_purpose: "fresh_admission",
            },
        )
        .await?;
        if !matches!(revoked.facts, Facts::Trust { trusted: false, public_key } if public_key == fresh_public_key)
        {
            return Err("reconciled fresh signing key is not observed as untrusted".into());
        }
        println!("S3_EVIDENCE revoked_fresh_key=untrusted exact_key_material=true");
        Ok::<(), Box<dyn std::error::Error>>(())
    })
}

#[test]
#[ignore = "requires the disposable S3-A TLS/FastCGI/MariaDB topology"]
fn real_capacity_two_inflight_rejects_third() -> Result<(), Box<dyn std::error::Error>> {
    static CAPACITY: TransientCapacity = TransientCapacity::new();
    let account_id = required("WP5_S3A_ACCOUNT_ID")?;
    let capacity_account_id = required("WP5_S3A_CAPACITY_ACCOUNT_ID")?;
    let barrier = Arc::new(Barrier::new(3));
    let spawn_query = |account_id: String, recovery: bool, operation: &'static str| {
        let barrier = Arc::clone(&barrier);
        thread::spawn(move || -> Result<(), String> {
            let descriptor = descriptor().map_err(|error| error.to_string())?;
            let request = Request::Account {
                recovery,
                account_id: &account_id,
                purpose: "platform_security",
                scope: if recovery {
                    "existing_actor_recovery"
                } else {
                    "fresh_admission"
                },
            };
            let mut permit = CAPACITY.try_queue().map_err(|error| error.to_string())?;
            permit.try_activate().map_err(|error| error.to_string())?;
            barrier.wait();
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| error.to_string())?;
            let response = runtime
                .block_on(native_admission_source::query(
                    &descriptor,
                    &request,
                    &mut permit,
                ))
                .map_err(|error| error.to_string())?;
            assert_observed(operation, response).map_err(|error| error.to_string())?;
            Ok(())
        })
    };
    let first = spawn_query(account_id, false, "ReadAccountSecurityV1/concurrent");
    let second = spawn_query(
        capacity_account_id,
        true,
        "ReadRecoveryAccountSecurityV2/concurrent",
    );
    barrier.wait();
    let mut third_permit = CAPACITY.try_queue()?;
    assert!(matches!(
        third_permit.try_activate(),
        Err(SourceError::CapacityExceeded)
    ));
    first.join().map_err(|_| "first query thread panicked")??;
    second
        .join()
        .map_err(|_| "second query thread panicked")??;
    println!("S3_EVIDENCE capacity=two_inflight third=immediate_reject");
    Ok(())
}

#[test]
fn failure_shape_remains_closed() {
    assert_eq!(
        Response::Failure(Failure::Unavailable),
        Response::Failure(Failure::Unavailable)
    );
}
