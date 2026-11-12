use super::super::*;
use crate::device_sessions::{DeviceCredential, DeviceSession, SessionKey, now};
use serde::Deserialize;
const DEVICE_BASE: &str = "/api/v1/game-auth/device-sessions";
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ResponseBody {
    protocol_version: u32,
    device_credential: DeviceCredential,
    family_id: String,
    absolute_expires_at: u64,
    idle_expires_at: u64,
    characters: Option<Vec<OwnerCharacter>>,
    ticket: Option<String>,
    expires_in: Option<u64>,
    expires_at: Option<u64>,
}
impl ResponseBody {
    fn parse(bytes: &[u8], suffix: &str) -> Result<Self, NativeLoginError> {
        let body: Self =
            serde_json::from_slice(bytes).map_err(|_| PlatformClientError::InvalidPayload)?;
        let value: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|_| PlatformClientError::InvalidPayload)?;
        let object = value
            .as_object()
            .ok_or(PlatformClientError::InvalidPayload)?;
        let extra: &[&str] = match suffix {
            "" => &[],
            "/native-characters" => &["characters"],
            "/tickets" => &["ticket", "expires_in", "expires_at"],
            _ => return Err(PlatformClientError::InvalidPayload.into()),
        };
        if object.len() != 5 + extra.len() || extra.iter().any(|key| !object.contains_key(*key)) {
            return Err(PlatformClientError::InvalidPayload.into());
        }
        Ok(body)
    }
    fn session(&self) -> Result<DeviceSession, NativeLoginError> {
        let session = DeviceSession {
            protocol_version: self.protocol_version,
            device_credential: self.device_credential.clone(),
            family_id: self.family_id.clone(),
            absolute_expires_at: self.absolute_expires_at,
            idle_expires_at: self.idle_expires_at,
        };
        session.validate(now()?)?;
        Ok(session)
    }
}
impl NativeLoginClient {
    /// No retry on enrollment, rotation or revocation: an ambiguous commit is terminal locally.
    async fn device_post(
        &self,
        suffix: &str,
        authorization: String,
        body: serde_json::Value,
        cancellation: CancellationToken,
    ) -> Result<Vec<u8>, NativeLoginError> {
        if self.platform.scheme() != "https"
            || !crate::device_sessions::canonical_uuid(&self.oauth_client_id)
        {
            return Err(PlatformClientError::InvalidBaseUrl.into());
        }
        let endpoint = self
            .platform
            .join(&(DEVICE_BASE.to_owned() + suffix))
            .map_err(|_| PlatformClientError::InvalidBaseUrl)?;
        let mut header = reqwest::header::HeaderValue::from_str(&authorization)
            .map_err(|_| PlatformClientError::InvalidPayload)?;
        header.set_sensitive(true);
        run(cancellation, async {
            let response = self
                .client
                .post(endpoint)
                .header(reqwest::header::AUTHORIZATION, header)
                .header(reqwest::header::ACCEPT, "application/json")
                .header(reqwest::header::ACCEPT_LANGUAGE, "en")
                .json(&body)
                .send()
                .await
                .map_err(|_| PlatformClientError::Request)?;
            let status = response.status();
            let bytes = read_bounded(response, 16_384).await?;
            if status != StatusCode::OK {
                return Err(NativeLoginError::DeviceRefused(DeviceRefusal::decode(
                    status.as_u16(),
                    &bytes,
                )?));
            }
            Ok(bytes)
        })
        .await
    }
    pub async fn enroll_device(
        &self,
        token: &AccessToken,
        remembered: bool,
        key: Option<&SessionKey>,
        cancel: CancellationToken,
    ) -> Result<DeviceSession, NativeLoginError> {
        let body = if remembered {
            serde_json::json!({"protocol_version":1,"remember_device":true})
        } else {
            key.ok_or(PlatformClientError::InvalidPayload)?
                .enrollment(token.0.expose(), &self.oauth_client_id)?
        };
        let bytes = self
            .device_post("", format!("Bearer {}", token.0.expose()), body, cancel)
            .await?;
        let reply = ResponseBody::parse(&bytes, "")?;
        if reply.characters.is_some()
            || reply.ticket.is_some()
            || reply.expires_in.is_some()
            || reply.expires_at.is_some()
        {
            return Err(PlatformClientError::InvalidPayload.into());
        }
        let session = reply.session()?;
        if !remembered && session.absolute_expires_at.saturating_sub(now()?) > 43_200 {
            return Err(PlatformClientError::InvalidPayload.into());
        }
        Ok(session)
    }
    async fn rotate_device(
        &self,
        session: &DeviceSession,
        suffix: &str,
        cancel: CancellationToken,
    ) -> Result<ResponseBody, NativeLoginError> {
        session.validate(now()?)?;
        let bytes = self
            .device_post(
                suffix,
                format!("OterynDevice {}", session.device_credential.expose()),
                serde_json::json!({"protocol_version":1,"client_id":self.oauth_client_id}),
                cancel,
            )
            .await?;
        let reply = ResponseBody::parse(&bytes, suffix)?;
        session.successor(&reply.session()?, now()?)?;
        Ok(reply)
    }
    pub async fn device_characters(
        &self,
        session: &DeviceSession,
        cancel: CancellationToken,
    ) -> Result<(DeviceSession, Vec<OwnerCharacter>), NativeLoginError> {
        let reply = self
            .rotate_device(session, "/native-characters", cancel)
            .await?;
        if reply.ticket.is_some() || reply.expires_in.is_some() || reply.expires_at.is_some() {
            return Err(PlatformClientError::InvalidPayload.into());
        }
        let successor = reply.session()?;
        let characters = reply
            .characters
            .ok_or(PlatformClientError::InvalidPayload)?;
        if characters.len() > 64
            || characters.iter().any(|c| {
                parse_uuid_v7(&c.character_id).is_none()
                    || parse_uuid_v7(&c.world_id).is_none()
                    || c.name.is_empty()
                    || c.name.len() > 96
                    || c.name.chars().any(char::is_control)
            })
        {
            return Err(PlatformClientError::InvalidPayload.into());
        }
        Ok((successor, characters))
    }
    pub async fn device_ticket(
        &self,
        session: &DeviceSession,
        cancel: CancellationToken,
    ) -> Result<(DeviceSession, NativeTicket), NativeLoginError> {
        let started = Instant::now();
        let utc = now()?;
        let reply = self.rotate_device(session, "/tickets", cancel).await?;
        if reply.characters.is_some() {
            return Err(PlatformClientError::InvalidPayload.into());
        }
        let successor = reply.session()?;
        let ticket = reply.ticket.ok_or(PlatformClientError::InvalidPayload)?;
        let lifetime = reply
            .expires_in
            .ok_or(PlatformClientError::InvalidPayload)?;
        let expiry = reply
            .expires_at
            .ok_or(PlatformClientError::InvalidPayload)?;
        if !(1..=60).contains(&lifetime)
            || expiry <= now()?
            || expiry > utc + 60
            || !is_visible_ascii(&ticket, MAX_TICKET_BYTES)
        {
            return Err(PlatformClientError::InvalidPayload.into());
        }
        Ok((
            successor,
            NativeTicket {
                secret: SecretText(ticket),
                usable_until: started + Duration::from_secs(lifetime.min(expiry - utc)),
                requests_sent: AtomicU32::new(0),
            },
        ))
    }
    pub async fn revoke_device(
        &self,
        credential: &DeviceCredential,
        cancel: CancellationToken,
    ) -> Result<(), NativeLoginError> {
        let bytes = self
            .device_post(
                "/revoke",
                format!("OterynDevice {}", credential.expose()),
                serde_json::json!({"protocol_version":1,"client_id":self.oauth_client_id}),
                cancel,
            )
            .await?;
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Revoked {
            protocol_version: u32,
            revoked: bool,
        }
        let reply: Revoked =
            serde_json::from_slice(&bytes).map_err(|_| PlatformClientError::InvalidPayload)?;
        if reply.protocol_version != 1 || !reply.revoked {
            return Err(PlatformClientError::InvalidPayload.into());
        }
        Ok(())
    }
}

impl NativeLoginClient {
    /// Adds an explicitly supplied out-of-band CA for private-PKI qualification.
    /// Hostname verification, HTTPS enforcement and redirect refusal remain enabled.
    pub fn with_additional_tls_root(mut self, der: &[u8]) -> Result<Self, PlatformClientError> {
        let certificate =
            reqwest::Certificate::from_der(der).map_err(|_| PlatformClientError::ClientCreation)?;
        self.client = Client::builder()
            .redirect(Policy::none())
            .no_proxy()
            .add_root_certificate(certificate)
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|_| PlatformClientError::ClientCreation)?;
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pinned_site_error_envelope_is_exact() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            DeviceRefusal::decode(400, br#"{"status":400,"error":"Invalid request"}"#)?,
            DeviceRefusal {
                status: 400,
                error: "Invalid request"
            }
        );
        assert_eq!(
            DeviceRefusal::decode(503, br#"{"status":503,"error":"Temporarily unavailable"}"#)?,
            DeviceRefusal {
                status: 503,
                error: "Temporarily unavailable"
            }
        );
        assert!(
            DeviceRefusal::decode(
                400,
                br#"{"status":400,"error":"Invalid request","extra":true}"#
            )
            .is_err()
        );
        assert!(
            DeviceRefusal::decode(401, br#"{"status":400,"error":"Invalid request"}"#).is_err()
        );
        assert!(
            DeviceRefusal::decode(400, br#"{"status":400,"error":"invalid_request"}"#).is_err()
        );
        assert!(DeviceRefusal::decode(400, br#"{"error":"Invalid request"}"#).is_err());
        Ok(())
    }
    #[test]
    fn refusal_retains_exact_status_and_only_contract_error_values()
    -> Result<(), Box<dyn std::error::Error>> {
        for status in [400, 401, 429, 503] {
            for error in [
                "invalid_request",
                "device_authorization_unavailable",
                "too_many_requests",
            ] {
                let bytes = serde_json::to_vec(&serde_json::json!({"error":error}))?;
                assert_eq!(
                    DeviceRefusal::decode(status, &bytes)?,
                    DeviceRefusal { status, error }
                );
            }
        }
        for bytes in [
            br#"{"error":"unrecognized"}"#.as_slice(),
            br#"{"error":"invalid_request","extra":true}"#,
            br#"{"error":"invalid_request","error":"invalid_request"}"#,
        ] {
            assert_eq!(
                DeviceRefusal::decode(401, bytes),
                Err(PlatformClientError::InvalidPayload)
            );
        }
        Ok(())
    }
    #[test]
    fn exact_device_response_shapes_reject_null_extras_and_duplicate_members()
    -> Result<(), Box<dyn std::error::Error>> {
        let bytes = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/contracts/fixtures/remembered-device-v1/enroll-response.json"
        ));
        assert!(ResponseBody::parse(bytes, "").is_ok());
        let mut value: serde_json::Value = serde_json::from_slice(bytes)?;
        value["characters"] = serde_json::Value::Null;
        assert!(ResponseBody::parse(&serde_json::to_vec(&value)?, "").is_err());
        let duplicate = std::str::from_utf8(bytes)?.replace(
            "\"protocol_version\":1",
            "\"protocol_version\":1,\"protocol_version\":1",
        );
        assert!(ResponseBody::parse(duplicate.as_bytes(), "").is_err());
        Ok(())
    }
}
