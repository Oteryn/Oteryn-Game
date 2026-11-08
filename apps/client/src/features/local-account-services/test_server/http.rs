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
                let authorized=head.lines().any(|line|line.eq_ignore_ascii_case("authorization: Bearer services-test-token"));
                let signature=head.lines().find_map(|line| {let(key,value)=line.split_once(':')?;key.eq_ignore_ascii_case("x-sandbox-signature").then(||value.trim())});
                let (status,reply)={let Ok(mut state)=state.lock() else {return;};state.requests.push(text.clone());state.route(method,target,body,authorized,signature)};
                let _=stream.write_all(format!("HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{reply}",reply.len()).as_bytes()).await;
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
