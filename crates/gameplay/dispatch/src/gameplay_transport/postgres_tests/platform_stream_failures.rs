use super::TestResult;
use super::fixture::Harness;
use crate::durability::platform_streams::PlatformStream;
use crate::platform_streams::{
    LOYALTY_IDENTITY_VAR, LOYALTY_PAGE_SCHEMA, LOYALTY_REQUEST_SCHEMA, LOYALTY_STREAM_PATH,
    PAGE_LIMIT, PlatformStreams, StreamPuller,
};
use crate::premium::client::PremiumClientConfig;
use std::{cell::RefCell, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    sync::oneshot,
};

#[allow(
    dead_code,
    reason = "this case only uses the test producer's mTLS material"
)]
#[path = "../../../../social/src/premium/test_producer.rs"]
mod test_producer;

thread_local! {
    static EVENT: RefCell<Option<oneshot::Sender<String>>> = const { RefCell::new(None) };
}

fn record_event(line: &str) {
    if let Some(out) = EVENT.take() {
        let _ = out.send(line.to_owned());
    }
}

#[test]
fn loyalty_stream_reports_cursor_read_failure() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    check_loyalty_failure(
        admin,
        "CursorRead",
        &["DELETE FROM game_platform_stream_cursors WHERE stream = $1"],
        false,
    )
}

#[test]
fn loyalty_stream_reports_request_encode_failure() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    check_loyalty_failure(
        admin,
        "RequestEncode",
        &[
            "ALTER TABLE game_platform_stream_cursors DROP CONSTRAINT game_platform_stream_cursors_stream_cursor_check",
            "UPDATE game_platform_stream_cursors SET stream_cursor = 'invalid cursor' WHERE stream = $1",
        ],
        false,
    )
}

#[test]
fn loyalty_stream_reports_page_apply_failure() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    check_loyalty_failure(
        admin,
        "PageApply",
        &["ALTER TABLE game_platform_stream_cursors ADD CHECK (pages = 0)"],
        true,
    )
}

fn check_loyalty_failure(
    admin: String,
    reason: &'static str,
    faults: &'static [&'static str],
    serve_page: bool,
) -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| error.to_string())?;
            runtime
                .block_on(check_poll(admin, reason, faults, serve_page))
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "loyalty poll panicked")??;
    Ok(())
}

async fn check_poll(
    admin: String,
    reason: &str,
    faults: &[&'static str],
    serve_page: bool,
) -> TestResult {
    let h = Harness::create_with_admission_seed(admin, "loyalty_failure", true, false).await?;
    for &sql in faults {
        let mut query = sqlx::query(sql);
        if sql.ends_with("$1") {
            query = query.bind(PlatformStream::Loyalty as i16);
        }
        query.execute(&h.pool).await?;
    }
    let pki = test_producer::TestPki::new();
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let puller = StreamPuller::new(
        &PremiumClientConfig {
            origin: format!("https://{}", listener.local_addr()?),
            identity_pem: pki.client_identity_pem.as_bytes().to_vec(),
            platform_ca_pem: pki.ca_pem.as_bytes().to_vec(),
        },
        PlatformStream::Loyalty,
        LOYALTY_IDENTITY_VAR,
    )?;
    let (tx, rx) = oneshot::channel();
    EVENT.set(Some(tx));
    let streams = PlatformStreams::spawn(&h.root, vec![puller], record_event);
    let result = tokio::time::timeout(Duration::from_secs(10), async {
        if serve_page {
            let (_, event) =
                tokio::try_join!(reply_with_loyalty_page(listener, pki.acceptor()), async {
                    Ok::<_, Box<dyn std::error::Error>>(rx.await?)
                })?;
            Ok::<_, Box<dyn std::error::Error>>(event)
        } else {
            Ok(rx.await?)
        }
    })
    .await;
    drop(streams);
    h.cleanup().await?;
    assert_eq!(
        result??,
        format!("event=platform_stream_failed stream=loyalty reason={reason}")
    );
    Ok(())
}

async fn reply_with_loyalty_page(
    listener: TcpListener,
    acceptor: tokio_rustls::TlsAcceptor,
) -> TestResult {
    let (socket, _) = listener.accept().await?;
    let mut stream = BufReader::new(acceptor.accept(socket).await?);
    let mut line = String::new();
    stream.read_line(&mut line).await?;
    assert_eq!(line, format!("POST {LOYALTY_STREAM_PATH} HTTP/1.1\r\n"));
    let mut length = None;
    loop {
        line.clear();
        if stream.read_line(&mut line).await? == 0 {
            return Err("loyalty request ended before its headers".into());
        }
        if line == "\r\n" {
            break;
        }
        if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
            length = Some(value.trim().parse::<usize>()?);
        }
    }
    let mut body = vec![0; length.ok_or("missing loyalty request content length")?];
    stream.read_exact(&mut body).await?;
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body)?,
        serde_json::json!({
            "schema": LOYALTY_REQUEST_SCHEMA, "after": null, "limit": PAGE_LIMIT,
        })
    );
    let page = serde_json::json!({"schema": LOYALTY_PAGE_SCHEMA, "items": [], "next_cursor": null})
        .to_string();
    let reply = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{page}",
        page.len()
    );
    stream.get_mut().write_all(reply.as_bytes()).await?;
    stream.get_mut().shutdown().await?;
    Ok(())
}
