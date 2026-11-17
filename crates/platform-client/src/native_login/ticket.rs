//! Native ticket issuance, with one bounded retry when Platform rate limits it.
use super::*;

pub(super) async fn issue(
    client: &NativeLoginClient,
    access_token: AccessToken,
    cancellation: CancellationToken,
) -> Result<NativeTicket, NativeLoginError> {
    let endpoint = client
        .platform
        .join("/api/v1/game-auth/tickets")
        .map_err(|_error| PlatformClientError::InvalidBaseUrl)?;
    let request = async {
        // Rate limiting happens before issuance, so one identical retry is safe.
        let mut retried = false;
        let response = loop {
            let response = client
                .client
                .post(endpoint.clone())
                .bearer_auth(access_token.0.expose())
                .json(&serde_json::json!({ "protocol_version": 1 }))
                .send()
                .await
                .map_err(|_error| PlatformClientError::Request)?;
            if response.status() != StatusCode::TOO_MANY_REQUESTS {
                break response;
            }
            if retried {
                return Err(NativeLoginError::TicketRefused(
                    PublicClass::TemporarilyUnavailable,
                ));
            }
            let delay = response
                .headers()
                .get(RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .map(Duration::from_secs)
                .filter(|delay| *delay <= MAX_RETRY_AFTER)
                .ok_or(NativeLoginError::TicketRefused(
                    PublicClass::TemporarilyUnavailable,
                ))?;
            #[cfg(test)]
            super::tests::ticket_wait(false);
            tokio::time::sleep(delay).await;
            #[cfg(test)]
            super::tests::ticket_wait(true);
            retried = true;
        };
        let received = Instant::now();
        match response.status() {
            status if status.is_success() => {}
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                return Err(NativeLoginError::TicketRefused(
                    PublicClass::AuthenticationRequired,
                ));
            }
            _ => {
                return Err(NativeLoginError::TicketRefused(
                    PublicClass::TemporarilyUnavailable,
                ));
            }
        }
        let body = read_bounded(response, MAX_TICKET_RESPONSE_BYTES).await?;
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct TicketResponse {
            protocol_version: u64,
            ticket: String,
            expires_in: u64,
        }
        let ticket: TicketResponse =
            serde_json::from_slice(&body).map_err(|_error| PlatformClientError::InvalidPayload)?;
        if ticket.protocol_version != 1
            || ticket.expires_in == 0
            || !is_visible_ascii(&ticket.ticket, MAX_TICKET_BYTES)
        {
            return Err(PlatformClientError::InvalidPayload.into());
        }
        let lifetime = Duration::from_secs(ticket.expires_in).min(MAX_TICKET_LIFETIME);
        Ok(NativeTicket {
            secret: SecretText(ticket.ticket),
            usable_until: received + lifetime,
            requests_sent: AtomicU32::new(0),
        })
    };
    run(cancellation, request).await
}
