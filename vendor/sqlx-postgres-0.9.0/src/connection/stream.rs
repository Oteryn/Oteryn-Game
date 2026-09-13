use std::ops::{ControlFlow, Deref, DerefMut};
use std::str::FromStr;

use futures_channel::mpsc::UnboundedSender;
use futures_util::SinkExt;
use log::Level;
use sqlx_core::bytes::Buf;
use sqlx_core::net::resource_budget::ResourceBudget;
use std::sync::Arc;

use crate::connection::tls::{MaybeUpgradeTls, MaybeUpgradeTlsOwned};
use crate::error::Error;
use crate::message::{
    BackendMessage, BackendMessageFormat, EncodeMessage, FrontendMessage, Notice, Notification,
    ParameterStatus, ReceivedMessage,
};
use crate::net::{self, BufferedSocket};
use crate::{PgConnectOptions, PgDatabaseError, PgSeverity};
use sqlx_core::net::{ConnectionOwner, OwnedSocket};

// the stream is a separate type from the connection to uphold the invariant where an instantiated
// [PgConnection] is a **valid** connection to postgres

// when a new connection is asked for, we work directly on the [PgStream] type until the
// connection is fully established

// in other words, `self` in any PgConnection method is a live connection to postgres that
// is fully prepared to receive queries

pub struct PgStream {
    // A trait object is okay here as the buffering amortizes the overhead of both the dynamic
    // function call as well as the syscall.
    inner: BufferedSocket<OwnedSocket>,

    // buffer of unreceived notification messages from `PUBLISH`
    // this is set when creating a PgListener and only written to if that listener is
    // re-used for query execution in-between receiving messages
    pub(crate) notifications: Option<UnboundedSender<Notification>>,

    pub(crate) parameter_statuses: ParameterStatuses,

    pub(crate) server_version_num: Option<u32>,
    resource_budget: Option<Arc<dyn ResourceBudget>>,
    poisoned: bool,
}

impl PgStream {
    pub(super) async fn connect(options: &PgConnectOptions) -> Result<Self, Error> {
        let socket_result = match options.fetch_socket() {
            Some(ref path) => net::connect_uds(path, MaybeUpgradeTls(options)).await?,
            None => net::connect_tcp(&options.host, options.port, MaybeUpgradeTls(options)).await?,
        };

        let socket = socket_result?;

        Ok(Self {
            inner: BufferedSocket::new(OwnedSocket::unowned(socket)),
            notifications: None,
            parameter_statuses: ParameterStatuses::default(),
            server_version_num: None,
            resource_budget: None,
            poisoned: false,
        })
    }

    pub(super) async fn connect_with_resource_budget(
        options: &PgConnectOptions,
        resource_budget: Arc<dyn ResourceBudget>,
    ) -> Result<Self, Error> {
        let owner = ConnectionOwner::try_new(resource_budget.clone())?;
        let socket_result = match options.fetch_socket() {
            Some(ref path) => {
                net::connect_uds_owned(path, MaybeUpgradeTlsOwned(options, owner.clone()), &owner)
                    .await?
            }
            None => {
                net::connect_tcp_owned(
                    &options.host,
                    options.port,
                    MaybeUpgradeTlsOwned(options, owner.clone()),
                    &owner,
                )
                .await?
            }
        };
        Ok(Self {
            inner: BufferedSocket::new_owned(socket_result?, resource_budget.clone())?,
            notifications: None,
            parameter_statuses: ParameterStatuses::default(),
            server_version_num: None,
            resource_budget: Some(resource_budget),
            poisoned: false,
        })
    }

    pub(crate) fn poison(&mut self) {
        // Keep socket/R/T custody until actual connection destruction. Every
        // later protocol entry refuses this connection so a pool must retire it.
        self.poisoned = true;
    }

    pub(crate) async fn flush(&mut self) -> std::io::Result<()> {
        if self.poisoned {
            return Err(std::io::ErrorKind::ConnectionAborted.into());
        }
        self.inner.flush().await
    }

    pub(crate) async fn shutdown(&mut self) -> std::io::Result<()> {
        if self.poisoned {
            return Err(std::io::ErrorKind::ConnectionAborted.into());
        }
        self.inner.shutdown().await
    }

    pub(crate) fn resource_budget(&self) -> Option<&Arc<dyn ResourceBudget>> {
        self.resource_budget.as_ref()
    }

    #[inline(always)]
    pub(crate) fn write_msg(&mut self, message: impl FrontendMessage) -> Result<(), Error> {
        if self.poisoned {
            return Err(Error::Io(std::io::ErrorKind::ConnectionAborted.into()));
        }
        use sqlx_core::io::ProtocolEncode;
        let size = message
            .body_size_bound()
            .0
            .checked_add(5)
            .filter(|size| *size - 1 <= i32::MAX as usize)
            .ok_or_else(|| Error::Io(std::io::ErrorKind::InvalidData.into()))?;
        self.inner
            .write_precharged(size, |buf| EncodeMessage(message).encode(buf))
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
        if self.poisoned {
            return Err(Error::Io(std::io::ErrorKind::ConnectionAborted.into()));
        }
        if self.resource_budget.is_some() {
            let header = self.inner.peek_owned(5).await?;
            let format = BackendMessageFormat::try_from_u8(header[0])?;
            let length =
                u32::from_be_bytes(header[1..5].try_into().expect("five-byte header")) as usize;
            if !(4..=i32::MAX as usize).contains(&length) {
                return Err(Error::Io(std::io::ErrorKind::InvalidData.into()));
            }
            let total = length
                .checked_add(1)
                .ok_or_else(|| Error::Io(std::io::ErrorKind::InvalidData.into()))?;
            let mut contents = self.inner.read_owned_buffered(total).await?;
            contents.advance(5);
            return Ok(ReceivedMessage { format, contents });
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

                if message_len < 4 {
                    return Err(Error::Io(std::io::ErrorKind::InvalidData.into()));
                }
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
                    contents: sqlx_core::net::OwnedBytes::unowned(contents),
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

                    let ParameterStatus { name, value } = message.decode()?;
                    // TODO: handle `client_encoding`, `DateStyle` change

                    match name.as_str() {
                        "server_version" => {
                            self.server_version_num = parse_server_version(&value);
                        }
                        _ => {
                            self.parameter_statuses.insert(
                                ParameterStatus { name, value },
                                self.resource_budget.clone(),
                            )?;
                        }
                    }

                    continue;
                }

                BackendMessageFormat::NoticeResponse => {
                    // do we need this to be more configurable?
                    // if you are reading this comment and think so, open an issue

                    let notice: Notice = message.decode()?;
                    // Peer notice text is never a logging payload on the owned path.
                    if self.resource_budget.is_some() {
                        continue;
                    }

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
                            message = notice.message()
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

impl Deref for PgStream {
    type Target = BufferedSocket<OwnedSocket>;

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
    let mut count = 0;

    let mut from = 0;
    let mut chs = s.char_indices().peekable();
    while let Some((i, ch)) = chs.next() {
        match ch {
            '.' => {
                if let Ok(num) = u32::from_str(&s[from..i]) {
                    if count == parts.len() {
                        return None;
                    }
                    parts[count] = num;
                    count += 1;
                    from = i + 1;
                } else {
                    break;
                }
            }
            _ if ch.is_ascii_digit() => {
                if chs.peek().is_none() {
                    if let Ok(num) = u32::from_str(&s[from..]) {
                        if count == parts.len() {
                            return None;
                        }
                        parts[count] = num;
                        count += 1;
                    }
                    break;
                }
            }
            _ => {
                if let Ok(num) = u32::from_str(&s[from..i]) {
                    if count == parts.len() {
                        return None;
                    }
                    parts[count] = num;
                    count += 1;
                }
                break;
            }
        };
    }

    let version_num = match &parts[..count] {
        [major, minor, rev] => major
            .checked_mul(100)?
            .checked_add(*minor)?
            .checked_mul(100)?
            .checked_add(*rev)?,
        [major, minor] if *major >= 10 => major.checked_mul(10000)?.checked_add(*minor)?,
        [major, minor] => major
            .checked_mul(100)?
            .checked_add(*minor)?
            .checked_mul(100)?,
        [major] => major.checked_mul(10000)?,
        _ => return None,
    };

    Some(version_num)
}

#[cfg(test)]
mod tests {
    use super::parse_server_version;

    #[test]
    fn test_parse_server_version_num() {
        // old style
        assert_eq!(parse_server_version("9.6.1"), Some(90601));
        // new style
        assert_eq!(parse_server_version("10.1"), Some(100001));
        // old style without minor version
        assert_eq!(parse_server_version("9.6devel"), Some(90600));
        // new style without minor version, e.g.  */
        assert_eq!(parse_server_version("10devel"), Some(100000));
        assert_eq!(parse_server_version("13devel87"), Some(130000));
        // unknown
        assert_eq!(parse_server_version("unknown"), None);
    }
}

#[derive(Default)]
pub(crate) struct ParameterStatuses {
    entries: Vec<ParameterStatus>,
    allocation: Option<sqlx_core::net::resource_budget::ResourceReservation>,
}
impl ParameterStatuses {
    pub(crate) fn contains_key(&self, key: &str) -> bool {
        self.entries.iter().any(|entry| entry.name.as_str() == key)
    }
    fn insert(
        &mut self,
        entry: ParameterStatus,
        budget: Option<Arc<dyn ResourceBudget>>,
    ) -> Result<(), Error> {
        if let Some(old) = self
            .entries
            .iter_mut()
            .find(|old| old.name.as_str() == entry.name.as_str())
        {
            *old = entry;
            return Ok(());
        }
        if self.entries.len() == self.entries.capacity() {
            let capacity = self
                .entries
                .len()
                .checked_add(1)
                .ok_or_else(crate::statement::allocation_denied)?;
            let bytes = capacity
                .checked_mul(std::mem::size_of::<ParameterStatus>())
                .ok_or_else(crate::statement::allocation_denied)?;
            let allocation = budget
                .map(|budget| {
                    sqlx_core::net::resource_budget::ResourceReservation::try_new(budget, bytes)
                })
                .transpose()
                .map_err(|_| crate::statement::allocation_denied())?;
            let mut entries = Vec::with_capacity(capacity);
            entries.append(&mut self.entries);
            drop(std::mem::replace(&mut self.entries, entries));
            self.allocation = allocation;
        }
        self.entries.push(entry);
        Ok(())
    }
}

#[cfg(test)]
mod custody_tests {
    use super::*;
    use crate::statement::custody_test_support::Ledger;
    use sqlx_core::net::OwnedBytes;
    #[test]
    fn status_replacement_and_denied_growth_preserve_custody() {
        let budget = Ledger::new(usize::MAX);
        let mut statuses = ParameterStatuses::default();
        let message = OwnedBytes::try_copy_from_slice(b"a\0first\0", budget.clone()).unwrap();
        statuses
            .insert(
                ParameterStatus::decode_body(message).unwrap(),
                Some(budget.clone()),
            )
            .unwrap();
        let message = OwnedBytes::try_copy_from_slice(b"a\0second\0", budget.clone()).unwrap();
        statuses
            .insert(
                ParameterStatus::decode_body(message).unwrap(),
                Some(budget.clone()),
            )
            .unwrap();
        assert_eq!(statuses.entries.len(), 1);
        assert_eq!(statuses.entries[0].value.as_str(), "second");
        let held = budget.held();
        let message = OwnedBytes::try_copy_from_slice(b"b\0denied\0", budget.clone()).unwrap();
        budget.limit(budget.held());
        assert!(statuses
            .insert(
                ParameterStatus::decode_body(message).unwrap(),
                Some(budget.clone())
            )
            .is_err());
        assert_eq!(budget.held(), held);
        assert_eq!(statuses.entries.len(), 1);
        drop(statuses);
        assert_eq!(budget.held(), 0);
    }
    #[test]
    fn version_parser_does_not_allocate_or_overflow_on_peer_numbers() {
        assert_eq!(parse_server_version("1.2.3.4"), None);
        assert_eq!(parse_server_version("4294967295.1"), None);
        assert_eq!(parse_server_version("17.6"), Some(170006));
    }
}
