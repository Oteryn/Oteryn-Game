use super::*;
impl MockPlatform {
    pub fn signed_event(
        &self,
        order: &CoinOrder,
        kind: EventKind,
    ) -> TestResultWith<(Vec<u8>, String)> {
        let event = Event {
            event_id: AttemptRef::generate()?.as_str().into(),
            order_id: order.order_id.clone(),
            price: order.price.clone(),
            kind,
        };
        let body = serde_json::to_vec(&event)?;
        let state = self.authority.lock().map_err(|_| "authority lock")?;
        let signature = state
            .coins
            .as_ref()
            .ok_or("coin provider")?
            .provider
            .sign(&body);
        Ok((body, signature))
    }
    pub fn webhook(&self, body: &[u8], signature: &str) -> TestResultWith<u16> {
        self.runtime.block_on(async {
            let address=self.base.strip_prefix("http://").and_then(|s|s.strip_suffix('/')).ok_or("local fixture URL")?;
            let mut stream=tokio::net::TcpStream::connect(address).await?;
            let head=format!("POST /mock-provider/webhook HTTP/1.1\r\nhost: {address}\r\ncontent-type: application/json\r\nx-sandbox-signature: {signature}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",body.len());
            stream.write_all(head.as_bytes()).await?;stream.write_all(body).await?;
            let mut reply=Vec::new();stream.read_to_end(&mut reply).await?;
            let text=std::str::from_utf8(&reply)?;
            Ok(text.split_whitespace().nth(1).ok_or("HTTP status")?.parse()?)
        })
    }
}
