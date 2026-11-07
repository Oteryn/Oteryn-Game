use crate::HashMap;

use crate::connection::{sasl, stream::PgStream};
use crate::error::Error;
use crate::io::StatementId;
use crate::message::{
    Authentication, BackendKeyData, BackendMessageFormat, Password, ReadyForQuery, Startup,
};
use crate::{PgAuthenticationPolicy, PgConnectOptions, PgConnection};

use super::{PgConnectionInner, PgStatementCache};

// https://www.postgresql.org/docs/current/protocol-flow.html#id-1.10.5.7.3
// https://www.postgresql.org/docs/current/protocol-flow.html#id-1.10.5.7.11

impl PgConnection {
    pub(crate) async fn establish(options: &PgConnectOptions) -> Result<Self, Error> {
        // Upgrade to TLS if we were asked to and the server supports it
        let mut stream = PgStream::connect(options).await?;

        // To begin a session, a frontend opens a connection to the server
        // and sends a startup message.

        let mut params = [("", ""); 6];
        params[..3].copy_from_slice(&[
            ("DateStyle", "ISO, MDY"),
            ("client_encoding", "UTF8"),
            ("TimeZone", "UTC"),
        ]);
        let mut count = 3;
        for (key, value) in [
            ("extra_float_digits", options.extra_float_digits.as_deref()),
            ("application_name", options.application_name.as_deref()),
            ("options", options.options.as_deref()),
        ] {
            if let Some(value) = value {
                params[count] = (key, value);
                count += 1;
            }
        }

        stream.write_startup(Startup {
            username: Some(&options.username),
            database: options.database.as_deref(),
            params: &params[..count],
        })?;

        stream.flush().await?;

        // The server then uses this information and the contents of
        // its configuration files (such as pg_hba.conf) to determine whether the connection is
        // provisionally acceptable, and what additional
        // authentication is required (if any).

        let mut process_id = 0;
        let mut secret_key = 0;
        let transaction_status;
        let strict_scram = options.authentication_policy == PgAuthenticationPolicy::ScramSha256;
        let mut scram_authenticated = false;

        loop {
            let message = stream.recv().await?;
            match message.format {
                BackendMessageFormat::Authentication => match message.decode()? {
                    Authentication::Ok => {
                        if strict_scram && !scram_authenticated {
                            return Err(err_protocol!(
                                "SCRAM-SHA-256 authentication required but server accepted the connection without SCRAM"
                            ));
                        }
                    }

                    Authentication::CleartextPassword => {
                        if strict_scram {
                            return Err(err_protocol!(
                                "SCRAM-SHA-256 authentication required; cleartext password rejected"
                            ));
                        }

                        stream
                            .send(Password::Cleartext(
                                options.password.as_deref().unwrap_or_default(),
                            ))
                            .await?;
                    }

                    Authentication::Md5Password(body) => {
                        if strict_scram {
                            return Err(err_protocol!(
                                "SCRAM-SHA-256 authentication required; MD5 password rejected"
                            ));
                        }

                        stream
                            .send(Password::Md5 {
                                username: &options.username,
                                password: options.password.as_deref().unwrap_or_default(),
                                salt: body.salt,
                            })
                            .await?;
                    }

                    Authentication::Sasl(body) => {
                        if strict_scram
                            && !body.mechanisms().any(|mechanism| mechanism == "SCRAM-SHA-256")
                        {
                            return Err(err_protocol!(
                                "SCRAM-SHA-256 authentication required but not offered by server"
                            ));
                        }
                        sasl::authenticate(&mut stream, options, body).await?;
                        if strict_scram {
                            scram_authenticated = true;
                        }
                    }

                    method => {
                        return Err(err_protocol!(
                            "unsupported authentication method: {:?}",
                            method
                        ));
                    }
                },

                BackendMessageFormat::BackendKeyData => {
                    // provides secret-key data that the frontend must save if it wants to be
                    // able to issue cancel requests later

                    let data: BackendKeyData = message.decode()?;

                    process_id = data.process_id;
                    secret_key = data.secret_key;
                }

                BackendMessageFormat::ReadyForQuery => {
                    if strict_scram && !scram_authenticated {
                        return Err(err_protocol!(
                            "SCRAM-SHA-256 authentication required before ReadyForQuery"
                        ));
                    }
                    transaction_status = message.decode::<ReadyForQuery>()?.transaction_status;

                    break;
                }

                _ => {
                    return Err(err_protocol!(
                        "establish: unexpected message: {:?}",
                        message.format
                    ))
                }
            }
        }

        let cache_statement = PgStatementCache::new(
            options.statement_cache_capacity,
            options.oteryn_wp3_first_slice_profile,
            stream.resource_budget().cloned(),
        )?;

        Ok(PgConnection {
            inner: Box::new(PgConnectionInner {
                stream,
                process_id,
                secret_key,
                transaction_status,
                transaction_depth: 0,
                pending_ready_for_query_count: 0,
                next_statement_id: StatementId::NAMED_START,
                cache_statement,
                cache_type_oid: HashMap::new(),
                cache_type_info: HashMap::new(),
                cache_elem_type_to_array: HashMap::new(),
                cache_table_data: HashMap::new(),
                log_settings: options.log_settings.clone(),
            }),
        })
    }
}
