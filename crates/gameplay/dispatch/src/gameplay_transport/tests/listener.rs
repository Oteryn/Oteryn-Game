/// Admits after `gate` opens; counts calls.
struct GatedAuthority {
    calls: AtomicUsize,
    gate: Notify,
    open: std::sync::atomic::AtomicBool,
}

impl GatedAuthority {
    fn new(open: bool) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            gate: Notify::new(),
            open: std::sync::atomic::AtomicBool::new(open),
        }
    }
    fn release(&self) {
        self.open.store(true, Ordering::SeqCst);
        self.gate.notify_waiters();
    }
}

impl FreshAdmissionAuthority for GatedAuthority {
    async fn admit(
        &self,
        attempt: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        while !self.open.load(Ordering::SeqCst) {
            let notified = self.gate.notified();
            if self.open.load(Ordering::SeqCst) {
                break;
            }
            notified.await;
        }
        Ok(AdmittedSession {
            game_session_id: attempt.game_session_id,
            world_id: crate::foundation::WorldId::decode(&CHARACTER)
                .map_err(|_| AdmissionRefusal::Unavailable)?,
            channel_id: crate::foundation::ChannelId::decode(&CHARACTER)
                .map_err(|_| AdmissionRefusal::Unavailable)?,
            runtime_actor: None,
            first_entry: FirstEntryOutcome::NotApplicable,
            controller: None,
            continuity: SessionContinuity::FRESH,
            item_fence: None,
        })
    }
}

#[derive(Default)]
struct Ends(Mutex<Vec<ConnectionEnd>>);
impl ConnectionObserver for Ends {
    fn ended(&self, end: ConnectionEnd) {
        self.0.lock().expect("ends").push(end);
    }
}
impl Ends {
    fn snapshot(&self) -> Vec<ConnectionEnd> {
        self.0.lock().expect("ends").clone()
    }
}

struct Material {
    server: Arc<rustls::ServerConfig>,
    client: TlsConnector,
    certificate: CertificateDer<'static>,
}

fn material() -> Result<Material, Box<dyn Error>> {
    let generated = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])?;
    let cert: CertificateDer<'static> = generated.cert.der().clone();
    let key = PrivatePkcs8KeyDer::from(generated.signing_key.serialize_der());
    let server = tcp_tls::tls_config(vec![cert.clone()], key.into())?;
    let mut roots = rustls::RootCertStore::empty();
    roots.add(cert.clone())?;
    let mut config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::aws_lc_rs::default_provider(),
    ))
    .with_protocol_versions(&[&rustls::version::TLS13])?
    .with_root_certificates(roots)
    .with_no_client_auth();
    config.alpn_protocols = vec![b"oteryn-game/1".to_vec()];
    Ok(Material {
        server,
        client: TlsConnector::from(Arc::new(config)),
        certificate: cert,
    })
}

fn bootstrap_frame() -> Vec<u8> {
    fn varint(output: &mut Vec<u8>, mut value: u64) {
        while value >= 0x80 {
            output.push((value as u8 & 0x7f) | 0x80);
            value >>= 7;
        }
        output.push(value as u8);
    }
    fn field(output: &mut Vec<u8>, key: u64, value: &[u8]) {
        varint(output, key);
        varint(output, value.len() as u64);
        output.extend_from_slice(value);
    }
    let mut payload = vec![0x08, 1, 0x10, 1, 0x18, 1];
    field(&mut payload, 0x2a, b"grant");
    field(&mut payload, 0x32, &CHARACTER);
    field(&mut payload, 0x3a, b"seam-test");
    let mut envelope = vec![0x08, 1];
    field(&mut envelope, 0x22, &payload);
    let mut framed = (envelope.len() as u32).to_be_bytes().to_vec();
    framed.extend_from_slice(&envelope);
    framed
}

/// Complete TLS, send a bootstrap, return the first server frame.
async fn admit_client(
    connector: TlsConnector,
    address: SocketAddr,
) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
    let mut stream = connector
        .connect(
            ServerName::try_from("localhost")?,
            TcpStream::connect(address).await?,
        )
        .await?;
    stream.write_all(&bootstrap_frame()).await?;
    let mut prefix = [0u8; 4];
    stream.read_exact(&mut prefix).await?;
    let mut body = vec![0; u32::from_be_bytes(prefix) as usize];
    stream.read_exact(&mut body).await?;
    Ok(body)
}

fn runtime() -> Result<tokio::runtime::Runtime, Box<dyn Error>> {
    Ok(tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?)
}

/// Both outputs; the workspace Tokio has no macros.
async fn join<A, B>(a: impl Future<Output = A>, b: impl Future<Output = B>) -> (A, B) {
    let mut a = pin!(a);
    let mut b = pin!(b);
    let (mut left, mut right) = (None, None);
    poll_fn(|context| {
        if left.is_none()
            && let Poll::Ready(value) = a.as_mut().poll(context)
        {
            left = Some(value);
        }
        if right.is_none()
            && let Poll::Ready(value) = b.as_mut().poll(context)
        {
            right = Some(value);
        }
        if left.is_some() && right.is_some() {
            Poll::Ready((left.take().expect("left"), right.take().expect("right")))
        } else {
            Poll::Pending
        }
    })
    .await
}

async fn settle(ms: u64) {
    tokio::time::sleep(Duration::from_millis(ms)).await;
}

#[test]
fn resource_limits_reject_values_above_registered_maxima() {
    let deadline = Duration::from_secs(1);
    assert!(ListenerLimits::new(MAX_CONNECTIONS, MAX_HANDSHAKE_UNITS, deadline).is_some());
    assert!(ListenerLimits::new(MAX_CONNECTIONS + 1, MAX_HANDSHAKE_UNITS, deadline).is_none());
    assert!(ListenerLimits::new(MAX_CONNECTIONS, MAX_HANDSHAKE_UNITS + 1, deadline).is_none());
    assert!(ListenerLimits::new(0, 1, deadline).is_none());
    assert!(ListenerLimits::new(1, 0, deadline).is_none());
    assert!(ListenerLimits::new(1, 1, Duration::ZERO).is_none());
}

#[test]
fn resource_connection_budget_backpressures_at_limit() -> Result<(), Box<dyn Error>> {
    runtime()?.block_on(async {
        let material = material()?;
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let limits = ListenerLimits::new(2, 2, Duration::from_millis(600)).ok_or("limits")?;
        let authority = GatedAuthority::new(true);
        let ends = Ends::default();
        let shutdown = CancellationToken::new();
        // Three silent clients: only two fit the budget; the third is not
        // accepted until an entry deadline frees a slot.
        let mut clients = Vec::new();
        for _ in 0..3 {
            clients.push(TcpStream::connect(address).await?);
        }
        let serve = serve_listener(
            &listener,
            &material.server,
            limits,
            &authority,
            &SecureIdentifiers,
            &ends,
            &shutdown,
        );
        let check = async {
            settle(900).await;
            let midway = ends.snapshot().len();
            settle(900).await;
            let finished = ends.snapshot().len();
            shutdown.cancel();
            (midway, finished)
        };
        let ((), (midway, finished)) = join(serve, check).await;
        assert_eq!(midway, 2, "third client must wait for a free slot");
        assert_eq!(finished, 3);
        assert!(
            ends.snapshot()
                .iter()
                .all(|end| *end == ConnectionEnd::TransportFailed)
        );
        assert_eq!(authority.calls.load(Ordering::SeqCst), 0);
        drop(clients);
        Ok(())
    })
}

/// Admits at once; every ended connection's loss is recorded and its
/// grace expiry then waits until shutdown.
struct LifecycleAuthority {
    inner: GatedAuthority,
    losses: AtomicUsize,
    expiring: AtomicUsize,
}

impl FreshAdmissionAuthority for LifecycleAuthority {
    async fn admit(
        &self,
        attempt: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        self.inner.admit(attempt).await
    }
    async fn lose_control(
        &self,
        _: AdmittedSession,
        _: Duration,
        _: Option<&crate::durability::world_reset::RetiringWorldReset>,
    ) -> ControlLossResult {
        self.losses.fetch_add(1, Ordering::SeqCst);
        ControlLossResult::Recorded
    }
    async fn expire_control_loss(&self, _: AdmittedSession) -> GraceExpiryResult {
        self.expiring.fetch_add(1, Ordering::SeqCst);
        std::future::pending().await
    }
}

/// KILL-REWARD-COMP-1 §1.3 (#1908 Codex P1 4202825538): an abandoned release, whose kill
/// drain may stay deferred or whose kill mark an unknown outcome kept on every attempt, runs
/// past the attempt bound until it ends; a mismatch release stops at it.
#[test]
fn an_abandoned_release_retries_past_the_bound() {
    assert!(retries_release(0, true));
    assert!(retries_release(0, false));
    assert!(!retries_release(EXPIRY_ATTEMPTS, false));
    assert!(retries_release(EXPIRY_ATTEMPTS, true));
    assert!(retries_release(u32::MAX, true));
}

/// Records the loss; the first two grace expiries end unknown.
struct UnknownExpiryAuthority {
    inner: GatedAuthority,
    expiries: AtomicUsize,
}

impl FreshAdmissionAuthority for UnknownExpiryAuthority {
    async fn admit(
        &self,
        attempt: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        self.inner.admit(attempt).await
    }
    async fn lose_control(
        &self,
        _: AdmittedSession,
        _: Duration,
        _: Option<&crate::durability::world_reset::RetiringWorldReset>,
    ) -> ControlLossResult {
        ControlLossResult::Recorded
    }
    async fn expire_control_loss(&self, _: AdmittedSession) -> GraceExpiryResult {
        if self.expiries.fetch_add(1, Ordering::SeqCst) < 2 {
            GraceExpiryResult::Unknown
        } else {
            GraceExpiryResult::Released
        }
    }
}

/// KILL-REWARD-COMP-1 §1.3: an unknown grace expiry runs again behind the backoff until it
/// reaches a final result.
#[test]
fn an_unknown_grace_expiry_is_retried_until_final() -> Result<(), Box<dyn Error>> {
    runtime()?.block_on(async {
        let authority = UnknownExpiryAuthority {
            inner: GatedAuthority::new(true),
            expiries: AtomicUsize::new(0),
        };
        let session = AdmittedSession {
            game_session_id: GameSessionId::decode(&CHARACTER)?,
            world_id: crate::foundation::WorldId::decode(&CHARACTER)?,
            channel_id: crate::foundation::ChannelId::decode(&CHARACTER)?,
            runtime_actor: None,
            first_entry: FirstEntryOutcome::NotApplicable,
            controller: None,
            continuity: SessionContinuity::FRESH,
            item_fence: None,
        };
        let lost = LostControl {
            reset: None,
            session,
            wait: Duration::ZERO,
        };
        control_loss_lifecycle(&authority, lost, &CancellationToken::new()).await;
        assert_eq!(authority.expiries.load(Ordering::SeqCst), 3);
        Ok(())
    })
}

#[test]
fn resource_loss_lifecycle_holds_no_connection_slot() -> Result<(), Box<dyn Error>> {
    runtime()?.block_on(async {
        let material = material()?;
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let limits = ListenerLimits::new(1, 1, Duration::from_millis(600)).ok_or("limits")?;
        let authority = LifecycleAuthority {
            inner: GatedAuthority::new(true),
            losses: AtomicUsize::new(0),
            expiring: AtomicUsize::new(0),
        };
        let ends = Ends::default();
        let shutdown = CancellationToken::new();
        let serve = serve_listener(
            &listener,
            &material.server,
            limits,
            &authority,
            &SecureIdentifiers,
            &ends,
            &shutdown,
        );
        let check = async {
            // The first admitted client leaves; its grace lifecycle stays pending.
            let first = admit_client(material.client.clone(), address).await;
            settle(200).await;
            let pending = authority.expiring.load(Ordering::SeqCst);
            // With a budget of one connection, a second admission still fits.
            let second = tokio::time::timeout(
                Duration::from_secs(2),
                admit_client(material.client.clone(), address),
            )
            .await;
            // The second client has left too; wait (bounded) until its end is served.
            for _ in 0..40 {
                if authority.losses.load(Ordering::SeqCst) == 2 {
                    break;
                }
                settle(50).await;
            }
            shutdown.cancel();
            (first.is_ok(), pending, matches!(second, Ok(Ok(_))))
        };
        let ((), (first, pending, second)) = join(serve, check).await;
        assert!(first);
        assert_eq!(pending, 1, "first loss must be in its grace lifecycle");
        assert!(
            second,
            "a pending grace lifecycle must not hold the only slot"
        );
        assert_eq!(authority.losses.load(Ordering::SeqCst), 2);
        Ok(())
    })
}

#[test]
fn resource_handshake_units_bound_concurrent_admission() -> Result<(), Box<dyn Error>> {
    runtime()?.block_on(async {
        let material = material()?;
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let limits = ListenerLimits::new(4, 1, Duration::from_millis(700)).ok_or("limits")?;
        let authority = GatedAuthority::new(false);
        let ends = Ends::default();
        let shutdown = CancellationToken::new();
        let first = tokio::spawn(admit_client(material.client.clone(), address));
        let serve = serve_listener(
            &listener,
            &material.server,
            limits,
            &authority,
            &SecureIdentifiers,
            &ends,
            &shutdown,
        );
        let check = async {
            settle(300).await;
            // The first admission holds the only unit; the second client
            // cannot even start TLS and reaches its entry deadline.
            let second = admit_client(material.client.clone(), address).await;
            let calls_while_held = authority.calls.load(Ordering::SeqCst);
            authority.release();
            let first = first.await;
            shutdown.cancel();
            (second.is_err(), calls_while_held, first)
        };
        let ((), (second_failed, calls_while_held, first)) = join(serve, check).await;
        assert!(second_failed);
        assert_eq!(calls_while_held, 1);
        let accepted: Vec<u8> = first?.map_err(|error| error.to_string())?;
        assert_eq!(
            decode_wire_envelope(&accepted)?.message_type(),
            MessageType::ServerAccepted
        );
        assert_eq!(authority.calls.load(Ordering::SeqCst), 1);
        Ok(())
    })
}

#[test]
fn shutdown_completes_admission_in_flight_and_cancels_entry_work() -> Result<(), Box<dyn Error>>
{
    runtime()?.block_on(async {
        let material = material()?;
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let limits = ListenerLimits::new(4, 4, Duration::from_secs(30)).ok_or("limits")?;
        let authority = GatedAuthority::new(false);
        let ends = Ends::default();
        let shutdown = CancellationToken::new();
        let admitting = tokio::spawn(admit_client(material.client.clone(), address));
        let silent = TcpStream::connect(address).await?;
        let serve = serve_listener(
            &listener,
            &material.server,
            limits,
            &authority,
            &SecureIdentifiers,
            &ends,
            &shutdown,
        );
        let check = async {
            settle(300).await;
            shutdown.cancel();
            settle(300).await;
            // Entry work was cancelled; the admission is still in flight.
            let before_release = ends.snapshot();
            authority.release();
            (before_release, admitting.await)
        };
        let ((), (before_release, admitted)) = join(serve, check).await;
        assert_eq!(before_release, vec![ConnectionEnd::TransportFailed]);
        let accepted: Vec<u8> = admitted?.map_err(|error| error.to_string())?;
        assert_eq!(
            decode_wire_envelope(&accepted)?.message_type(),
            MessageType::ServerAccepted
        );
        let ends = ends.snapshot();
        assert_eq!(ends.len(), 2);
        assert!(matches!(
            ends[1],
            ConnectionEnd::AdmittedThenDisconnected(_)
        ));
        drop(silent);
        Ok(())
    })
}

#[test]
fn transport_negatives_close_without_admission() -> Result<(), Box<dyn Error>> {
    runtime()?.block_on(async {
        let material = material()?;
        let generated = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])?;
        let _ = generated;
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let limits = ListenerLimits::new(8, 8, Duration::from_secs(5)).ok_or("limits")?;
        let authority = GatedAuthority::new(true);
        let ends = Ends::default();
        let shutdown = CancellationToken::new();
        let serve = serve_listener(
            &listener,
            &material.server,
            limits,
            &authority,
            &SecureIdentifiers,
            &ends,
            &shutdown,
        );
        let check = async {
            let mut outcomes = Vec::new();
            for (tls12, alpn) in [
                (true, Some(b"oteryn-game/1".as_slice())),
                (false, Some(b"wrong".as_slice())),
                (false, None),
            ] {
                let started = std::time::Instant::now();
                let outcome = tokio::time::timeout(
                    Duration::from_secs(10),
                    negative_client(&material, address, tls12, alpn),
                )
                .await;
                outcomes.push((outcome.is_ok(), started.elapsed()));
            }
            shutdown.cancel();
            outcomes
        };
        let ((), outcomes) = join(serve, check).await;
        for (finished, elapsed) in &outcomes {
            assert!(*finished, "client hung after {elapsed:?}");
        }
        assert_eq!(authority.calls.load(Ordering::SeqCst), 0);
        Ok(())
    })
}

async fn negative_client(
    material: &Material,
    address: SocketAddr,
    tls12: bool,
    alpn: Option<&[u8]>,
) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
    let server_cert = material.certificate.clone();
    let mut roots = rustls::RootCertStore::empty();
    roots.add(server_cert)?;
    let version = if tls12 {
        &rustls::version::TLS12
    } else {
        &rustls::version::TLS13
    };
    let mut config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::aws_lc_rs::default_provider(),
    ))
    .with_protocol_versions(&[version])?
    .with_root_certificates(roots)
    .with_no_client_auth();
    config.alpn_protocols = alpn.map(<[u8]>::to_vec).into_iter().collect();
    let tcp = TcpStream::connect(address).await?;
    let Ok(mut stream) = TlsConnector::from(Arc::new(config))
        .connect(ServerName::try_from("localhost")?, tcp)
        .await
    else {
        return Ok(Vec::new());
    };
    let _ = stream.write_all(&bootstrap_frame()).await;
    let mut output = Vec::new();
    let _ = stream.read_to_end(&mut output).await;
    Ok(output)
}

fn uuid_v7(seed: u8) -> [u8; 16] {
    let mut bytes = [seed; 16];
    bytes[6] = 0x70 | (seed & 0x0f);
    bytes[8] = 0x80 | (seed & 0x3f);
    bytes
}
