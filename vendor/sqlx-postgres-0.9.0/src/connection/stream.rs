use std::collections::BTreeMap;
use std::ops::{ControlFlow, Deref, DerefMut};

use futures_channel::mpsc::UnboundedSender;
use futures_util::SinkExt;
use log::Level;
use sqlx_core::bytes::Buf;

use crate::connection::tls::MaybeUpgradeTls;
use crate::error::Error;
use crate::message::{
    BackendMessage, BackendMessageFormat, EncodeMessage, FrontendMessage, Notice, Notification,
    ParameterStatus, ReceivedMessage, Startup, validate_wp3_data_row_body,
    validate_wp3_parameter_description_body, validate_wp3_row_description_body,
    wp3_borrowed_server_version,
};
use crate::net::{self, BufferedSocket, Socket};
use crate::{PgConnectOptions, PgDatabaseError, PgSeverity};

// the stream is a separate type from the connection to uphold the invariant where an instantiated
// [PgConnection] is a **valid** connection to postgres

// when a new connection is asked for, we work directly on the [PgStream] type until the
// connection is fully established

// in other words, `self` in any PgConnection method is a live connection to postgres that
// is fully prepared to receive queries

pub struct PgStream {
    // A trait object is okay here as the buffering amortizes the overhead of both the dynamic
    // function call as well as the syscall.
    inner: BufferedSocket<Box<dyn Socket>>,

    // buffer of unreceived notification messages from `PUBLISH`
    // this is set when creating a PgListener and only written to if that listener is
    // re-used for query execution in-between receiving messages
    pub(crate) notifications: Option<UnboundedSender<Notification>>,

    pub(crate) parameter_statuses: BTreeMap<String, String>,

    pub(crate) server_version_num: Option<u32>,
    pub(crate) oteryn_wp3_first_slice_profile: bool,
}

impl PgStream {
    pub(super) async fn connect(options: &PgConnectOptions) -> Result<Self, Error> {
        let socket_result = match options.fetch_socket() {
            Some(ref path) => net::connect_uds(path, MaybeUpgradeTls(options)).await?,
            None => {
                let transport_host = options.host_addr.as_deref().unwrap_or(&options.host);
                net::connect_tcp(transport_host, options.port, MaybeUpgradeTls(options)).await?
            }
        };

        let socket = socket_result?;
        let inner = if options.oteryn_wp3_first_slice_profile {
            let budget = options
                .wp3_resource_budget()
                .ok_or_else(|| Error::Io(std::io::ErrorKind::OutOfMemory.into()))?;
            // Owner-approved WP3 accounting classifies upstream connect/TLS construction as
            // external L_T, transferred to L_R on success and retained through every dependency
            // descendant's finality. Same-root ResourceBudget custody begins at this first
            // Oteryn-controlled buffered-socket retention.
            BufferedSocket::new_owned(socket, budget)?
        } else {
            BufferedSocket::new(socket)
        };

        Ok(Self {
            inner,
            notifications: None,
            parameter_statuses: BTreeMap::default(),
            server_version_num: None,
            oteryn_wp3_first_slice_profile: options.oteryn_wp3_first_slice_profile,
        })
    }

    pub(crate) fn resource_budget(
        &self,
    ) -> Option<&std::sync::Arc<dyn sqlx_core::net::ResourceBudget>> {
        self.inner.resource_budget()
    }

    #[inline(always)]
    pub(crate) fn write_msg(&mut self, message: impl FrontendMessage) -> Result<(), Error> {
        if self.oteryn_wp3_first_slice_profile {
            let mut encoded_size = message.body_size_hint();
            encoded_size += 5;
            self.write_precharged(EncodeMessage(message), encoded_size.0)
        } else {
            self.write(EncodeMessage(message))
        }
    }

    #[inline(always)]
    pub(crate) fn write_startup(&mut self, message: Startup<'_>) -> Result<(), Error> {
        if self.oteryn_wp3_first_slice_profile {
            let encoded_size = message.encoded_size()?;
            self.write_precharged(message, encoded_size)
        } else {
            self.write(message)
        }
    }

    pub(crate) async fn send<T>(&mut self, message: T) -> Result<(), Error>
    where
        T: FrontendMessage,
    {
        self.write_msg(message)?;
        self.flush().await?;
        Ok(())
    }

    // Expect a specific type and format
    pub(crate) async fn recv_expect<B: BackendMessage>(&mut self) -> Result<B, Error> {
        self.recv().await?.decode()
    }

    pub(crate) async fn recv_unchecked(&mut self) -> Result<ReceivedMessage, Error> {
        if self.oteryn_wp3_first_slice_profile {
            let header = self.inner.read_owned_header().await?;
            let format = BackendMessageFormat::try_from_u8(header[0])?;
            let message_len =
                u32::from_be_bytes([header[1], header[2], header[3], header[4]]) as usize;
            let expected_len = wp3_backend_expected_len(message_len)?;
            debug_assert_eq!(expected_len, message_len + 1);
            let body_len = message_len
                .checked_sub(4)
                .ok_or_else(|| err_protocol!("WP3 backend message body underflow"))?;
            let (contents, allocation) = self.inner.read_owned_body(body_len).await?;

            match format {
                BackendMessageFormat::DataRow => validate_wp3_data_row_body(&contents)?,
                BackendMessageFormat::ParameterDescription => {
                    validate_wp3_parameter_description_body(&contents)?
                }
                BackendMessageFormat::RowDescription => {
                    validate_wp3_row_description_body(&contents)?
                }
                BackendMessageFormat::NotificationResponse
                | BackendMessageFormat::CopyData
                | BackendMessageFormat::CopyDone
                | BackendMessageFormat::CopyInResponse
                | BackendMessageFormat::CopyOutResponse => {
                    return Err(err_protocol!(
                        "backend message is outside the WP3 first-slice profile"
                    ));
                }
                _ => {}
            }

            return Ok(ReceivedMessage {
                format,
                contents,
                allocation: Some(allocation),
            });
        }

        // NOTE: to not break everything, this should be cancel-safe;
        // DO NOT modify `buf` unless a full message has been read
        self.inner
            .try_read(|buf| {
                // all packets in postgres start with a 5-byte header
                // this header contains the message type and the total length of the message
                let Some(mut header) = buf.get(..5) else {
                    return Ok(ControlFlow::Continue(5));
                };

                let format = BackendMessageFormat::try_from_u8(header.get_u8())?;

                let message_len = header.get_u32() as usize;

                let expected_len = message_len
                    .checked_add(1)
                    // this shouldn't really happen but is mostly a sanity check
                    .ok_or_else(|| {
                        err_protocol!("message_len + 1 overflows usize: {message_len}")
                    })?;

                if buf.len() < expected_len {
                    return Ok(ControlFlow::Continue(expected_len));
                }

                // `buf` SHOULD NOT be modified ABOVE this line

                // pop off the format code since it's not counted in `message_len`
                buf.advance(1);

                // consume the message, including the length prefix
                let mut contents = buf.split_to(message_len).freeze();

                // cut off the length prefix
                contents.advance(4);

                Ok(ControlFlow::Break(ReceivedMessage {
                    format,
                    contents,
                    allocation: None,
                }))
            })
            .await
    }

    // Get the next message from the server
    // May wait for more data from the server
    pub(crate) async fn recv(&mut self) -> Result<ReceivedMessage, Error> {
        loop {
            let message = self.recv_unchecked().await?;

            match message.format {
                BackendMessageFormat::ErrorResponse => {
                    // An error returned from the database server.
                    return Err(message.decode::<PgDatabaseError>()?.into());
                }

                BackendMessageFormat::NotificationResponse => {
                    if let Some(buffer) = &mut self.notifications {
                        let notification: Notification = message.decode()?;
                        let _ = buffer.send(notification).await;

                        continue;
                    }
                }

                BackendMessageFormat::ParameterStatus => {
                    // informs the frontend about the current (initial)
                    // setting of backend parameters

                    if self.oteryn_wp3_first_slice_profile {
                        if let Some(version) = wp3_borrowed_server_version(&message.contents)? {
                            self.server_version_num = parse_server_version(version);
                        }
                    } else {
                        let ParameterStatus { name, value } = message.decode()?;
                        // TODO: handle `client_encoding`, `DateStyle` change

                        match name.as_str() {
                            "server_version" => {
                                self.server_version_num = parse_server_version(&value);
                            }
                            _ => {
                                self.parameter_statuses.insert(name, value);
                            }
                        }
                    }

                    continue;
                }

                BackendMessageFormat::NoticeResponse => {
                    // do we need this to be more configurable?
                    // if you are reading this comment and think so, open an issue

                    let notice: Notice = message.decode()?;

                    let (log_level, tracing_level) = match notice.severity() {
                        PgSeverity::Fatal | PgSeverity::Panic | PgSeverity::Error => {
                            (Level::Error, tracing::Level::ERROR)
                        }
                        PgSeverity::Warning => (Level::Warn, tracing::Level::WARN),
                        PgSeverity::Notice => (Level::Info, tracing::Level::INFO),
                        PgSeverity::Debug => (Level::Debug, tracing::Level::DEBUG),
                        PgSeverity::Info | PgSeverity::Log => (Level::Trace, tracing::Level::TRACE),
                    };

                    let log_is_enabled = log::log_enabled!(
                        target: "sqlx::postgres::notice",
                        log_level
                    ) || sqlx_core::private_tracing_dynamic_enabled!(
                        target: "sqlx::postgres::notice",
                        tracing_level
                    );
                    if log_is_enabled {
                        sqlx_core::private_tracing_dynamic_event!(
                            target: "sqlx::postgres::notice",
                            tracing_level,
                            severity = ?notice.severity(),
                            sqlstate_class = notice_sqlstate_class(notice.code())
                        );
                    }

                    continue;
                }

                _ => {}
            }

            return Ok(message);
        }
    }
}

fn wp3_backend_expected_len(message_len: usize) -> Result<usize, Error> {
    if !(4..=131_206).contains(&message_len) {
        return Err(err_protocol!(
            "backend message length is outside the WP3 first-slice profile"
        ));
    }
    message_len
        .checked_add(1)
        .ok_or_else(|| err_protocol!("WP3 backend message length overflow"))
}

fn notice_sqlstate_class(code: &str) -> &'static str {
    if code.len() != 5 || !code.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
        return "invalid";
    }
    match &code.as_bytes()[..2] {
        b"00" => "success",
        b"01" => "warning",
        b"02" => "no_data",
        b"08" => "connection",
        b"22" => "data_exception",
        b"23" => "integrity_constraint",
        b"25" => "transaction_state",
        b"28" => "authorization",
        b"40" => "rollback",
        b"42" => "syntax_or_access",
        b"53" => "insufficient_resources",
        b"54" => "program_limit",
        b"55" => "object_state",
        b"57" => "operator_intervention",
        b"58" => "system_error",
        b"XX" => "internal",
        _ => "other",
    }
}

impl Deref for PgStream {
    type Target = BufferedSocket<Box<dyn Socket>>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for PgStream {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

// reference:
// https://github.com/postgres/postgres/blob/6feebcb6b44631c3dc435e971bd80c2dd218a5ab/src/interfaces/libpq/fe-exec.c#L1030-L1065
fn parse_server_version(s: &str) -> Option<u32> {
    let mut parts = [0u32; 3];
    let mut parts_len = 0usize;
    let mut current = 0u32;
    let mut has_digit = false;

    for byte in s.bytes() {
        if byte.is_ascii_digit() {
            current = current
                .checked_mul(10)?
                .checked_add(u32::from(byte - b'0'))?;
            has_digit = true;
            continue;
        }
        if byte == b'.' && has_digit && parts_len < parts.len() {
            parts[parts_len] = current;
            parts_len += 1;
            current = 0;
            has_digit = false;
            continue;
        }
        break;
    }
    if has_digit {
        if parts_len >= parts.len() {
            return None;
        }
        parts[parts_len] = current;
        parts_len += 1;
    }

    match parts_len {
        3 => parts[0]
            .checked_mul(100)?
            .checked_add(parts[1])?
            .checked_mul(100)?
            .checked_add(parts[2]),
        2 if parts[0] >= 10 => parts[0].checked_mul(10_000)?.checked_add(parts[1]),
        2 => parts[0]
            .checked_mul(100)?
            .checked_add(parts[1])?
            .checked_mul(100),
        1 => parts[0].checked_mul(10_000),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{notice_sqlstate_class, parse_server_version, wp3_backend_expected_len};

    #[test]
    fn wp3_backend_message_max_is_checked_before_peer_driven_read_target() {
        assert_eq!(wp3_backend_expected_len(131_206).unwrap(), 131_207);
        assert!(wp3_backend_expected_len(131_207).is_err());
        assert!(wp3_backend_expected_len(3).is_err());
    }

    #[test]
    fn wp3_notice_logging_uses_only_bounded_sqlstate_classification() {
        assert_eq!(notice_sqlstate_class("23505"), "integrity_constraint");
        assert_eq!(notice_sqlstate_class("XX000"), "internal");
        assert_eq!(notice_sqlstate_class("ZZ999"), "other");
        assert_eq!(notice_sqlstate_class("peer controlled prose"), "invalid");
    }

    #[test]
    fn test_parse_server_version_num() {
        // old style
        assert_eq!(parse_server_version("9.6.1"), Some(90601));
        // new style
        assert_eq!(parse_server_version("10.1"), Some(100001));
        assert_eq!(parse_server_version("17.6"), Some(170006));
        // old style without minor version
        assert_eq!(parse_server_version("9.6devel"), Some(90600));
        // new style without minor version, e.g.  */
        assert_eq!(parse_server_version("10devel"), Some(100000));
        assert_eq!(parse_server_version("13devel87"), Some(130000));
        // unknown
        assert_eq!(parse_server_version("unknown"), None);
    }

    #[test]
    fn parse_server_version_rejects_composition_overflow() {
        assert_eq!(parse_server_version("42949673.0.0"), None);
        assert_eq!(parse_server_version("42949672.96.0"), None);
        assert_eq!(parse_server_version("9.42949672"), None);
    }
}
