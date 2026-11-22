use super::*;

impl MockPlatform {
    pub fn start() -> TestResultWith<Self> {
        let runtime = tokio::runtime::Runtime::new()?;
        let listener = runtime.block_on(TcpListener::bind("127.0.0.1:0"))?;
        let base = format!("http://127.0.0.1:{}/", listener.local_addr()?.port());
        let authority = Arc::new(Mutex::new(Authority {
            coins: Some(coins::CoinAuthority::new(base.clone())?),
            offline: true,
            ..Authority::default()
        }));
        let state = authority.clone();
        runtime.spawn(async move {
            while let Ok((mut stream,_))=listener.accept().await {
                let mut bytes=Vec::new();let mut chunk=[0;4096];
                let text=loop {
                    let Ok(n)=stream.read(&mut chunk).await else {return;}; if n==0 {return;}
                    bytes.extend_from_slice(&chunk[..n]); if bytes.len()>16384 {return;}
                    let text=String::from_utf8_lossy(&bytes).into_owned();
                    if let Some((head,body))=text.split_once("\r\n\r\n") {
                        let length=head.lines().find_map(|line|{let(k,v)=line.split_once(':')?;k.eq_ignore_ascii_case("content-length").then(||v.trim().parse::<usize>().ok()).flatten()}).unwrap_or(0);
                        if body.len()>=length {break text;}
                    }
                };
                let (head,body)=text.split_once("\r\n\r\n").unwrap_or_default();
                let mut line=head.lines().next().unwrap_or_default().split_whitespace();
                let method=line.next().unwrap_or_default();let target=line.next().unwrap_or_default();
                let grant = head.lines().find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    if !name.eq_ignore_ascii_case("authorization") { return None; }
                    value.trim().strip_prefix("Bearer services-test-token-")?.parse::<u32>().ok()
                });
                let signature=head.lines().find_map(|line| {let(key,value)=line.split_once(':')?;key.eq_ignore_ascii_case("x-sandbox-signature").then(||value.trim())});
                let (status,reply)={let Ok(mut state)=state.lock() else {return;};let headers = head.lines().filter(|line| !line.to_ascii_lowercase().starts_with("authorization:"));
                    state.requests.push(format!("{}\r\n\r\n{body}", headers.collect::<Vec<_>>().join("\r\n")));state.respond(method,target,body,grant,signature)};
                if status == 0 { continue; }
                let response = format!("HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{reply}", reply.len());
                let status_gate = (method == "GET" && (target.contains("/bazaar/commands/") || target.contains("/coin-purchases/requests/")))
                    .then(|| state.lock().ok().and_then(|s| s.status_barrier.clone())).flatten();
                if let Some(gate) = status_gate {
                    let state = state.clone();
                    tokio::spawn(async move {
                        if tokio::time::timeout(std::time::Duration::from_secs(2), gate.wait()).await.is_ok() {
                            if let Ok(mut s) = state.lock() { s.last_status = Some(std::time::Instant::now()); }
                            let _ = stream.write_all(response.as_bytes()).await;
                        }
                    });
                } else if method == "POST" && target == "/api/v1/native-account/coin-purchases/orders"
                    && let Some((ready, release)) = state.lock().ok().and_then(|mut s| s.hold_coin_response.take())
                {
                    tokio::spawn(async move {
                        let _ = ready.send(());
                        let _ = release.await;
                        let _ = stream.write_all(response.as_bytes()).await;
                    });
                } else {
                    let _ = stream.write_all(response.as_bytes()).await;
                }
            }
        });
        Ok(Self {
            runtime,
            base,
            authority,
        })
    }
    pub fn client(&self) -> TestResultWith<NativeLoginClient> {
        let config = PlatformClientConfig::new(&self.base)?;
        Ok(NativeLoginClient::new(&config, &config, "native")?)
    }
    pub fn token(&self, client: &NativeLoginClient) -> TestResultWith<AccessToken> {
        Ok(self.runtime.block_on(client.exchange_code(
            "mock-code",
            "mock-verifier",
            "http://127.0.0.1:1/callback",
            CancellationToken::new(),
        ))?)
    }
}

impl Authority {
    fn respond(
        &mut self,
        method: &str,
        target: &str,
        body: &str,
        grant: Option<u32>,
        signature: Option<&str>,
    ) -> (u16, String) {
        if method == "POST" && target == "/oauth/token" {
            self.oauth_exchanges += 1;
            return (
                200,
                serde_json::json!({
                    "token_type": "Bearer",
                    "access_token": format!("services-test-token-{}", self.oauth_exchanges),
                    "expires_in": 1800,
                    "scope": "game:account-services",
                    "refresh_token": format!("refresh-test-token-{}", self.oauth_exchanges)
                })
                .to_string(),
            );
        }
        let grant = grant.filter(|n| (1..=self.oauth_exchanges).contains(n));
        if let Some(wallet) = grant.and_then(|n| self.coin_wallets_by_grant.get_mut(&n))
            && target.starts_with("/api/v1/native-account/coin-purchases/")
        {
            return wallet
                .route(method, target, body)
                .unwrap_or((404, String::new()));
        }
        let authorized = grant.is_some();
        if authorized && method == "POST" {
            let failure = match target {
                "/api/v1/native-account/bazaar/commands" => self.bazaar_post_failure.take(),
                "/api/v1/native-account/coin-purchases/orders" => self.coin_post_failure.take(),
                _ => None,
            };
            if let Some(status) = failure {
                return (status, String::new());
            }
        }
        self.route(method, target, body, authorized, signature)
    }
}

pub(crate) fn test_thread(test: impl FnOnce() -> TestResult + Send + 'static) -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || test().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "test thread failed")?
        .map_err(Into::into)
}
