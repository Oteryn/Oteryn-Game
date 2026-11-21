//! Optional, purpose-separated game-coins identity. Only the node loads this file.
use super::{CoinClient, Error, MtlsTransport};
use crate::descriptor::ProducerDescriptor;
use serde::Deserialize;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    #[serde(default)]
    market_enabled: bool,
    #[serde(default)]
    transfers_enabled: bool,
    source_authority: String,
    endpoint: std::net::SocketAddr,
    peer_name: String,
    trust_roots_der: Vec<String>,
    client_certificate_der: String,
    client_key_der: String,
}
fn bounded(path: &str, max: usize) -> Result<Vec<u8>, Error> {
    let fd = rustix::fs::openat(
        rustix::fs::CWD,
        path,
        rustix::fs::OFlags::RDONLY | rustix::fs::OFlags::CLOEXEC | rustix::fs::OFlags::NONBLOCK,
        rustix::fs::Mode::empty(),
    )
    .map_err(|_| Error::Unavailable)?;
    let file = std::fs::File::from(fd);
    if !file.metadata().map_err(|_| Error::Unavailable)?.is_file() {
        return Err(Error::Invalid);
    }
    use std::io::Read;
    let mut bytes = Vec::new();
    file.take((max + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::Unavailable)?;
    if bytes.is_empty() || bytes.len() > max {
        return Err(Error::Invalid);
    }
    Ok(bytes)
}
impl CoinClient {
    pub fn configured() -> Result<Option<Self>, Error> {
        let Some(path) = std::env::var_os("OTERYN_COIN_SOURCE_CONFIG") else {
            return Ok(None);
        };
        let config: Config =
            serde_json::from_slice(&bounded(path.to_str().ok_or(Error::Invalid)?, 8192)?)?;
        if config.trust_roots_der.is_empty() || config.trust_roots_der.len() > 4 {
            return Err(Error::Invalid);
        }
        let roots = config
            .trust_roots_der
            .iter()
            .map(|p| bounded(p, 4096).map(rustls::pki_types::CertificateDer::from))
            .collect::<Result<Vec<_>, _>>()?;
        let chain = vec![rustls::pki_types::CertificateDer::from(bounded(
            &config.client_certificate_der,
            4096,
        )?)];
        let key =
            rustls::pki_types::PrivateKeyDer::try_from(bounded(&config.client_key_der, 4096)?)
                .map_err(|_| Error::Invalid)?;
        let descriptor = ProducerDescriptor::new(
            config.source_authority,
            (config.endpoint.ip().to_string(), config.endpoint.port()),
            config.peer_name.clone(),
            config.peer_name,
            roots,
            chain,
            key,
        )
        .map_err(|_| Error::Invalid)?;
        Ok(Some(
            Self::new(Box::new(MtlsTransport::new(descriptor)))
                .with_market_enabled(config.market_enabled)
                .with_transfers_enabled(config.transfers_enabled),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coin_configuration_fifo_without_a_writer_is_refused_without_blocking()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = std::env::temp_dir().join(format!(
            "oteryn-coin-config-fifo-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&directory)?;
        let path = directory.join("config");
        rustix::fs::mkfifoat(
            rustix::fs::CWD,
            &path,
            rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
        )?;
        let target = path.to_str().ok_or("path")?.to_owned();
        let (sent, received) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || sent.send(bounded(&target, 8192)));
        let outcome = received.recv_timeout(std::time::Duration::from_secs(1));
        if outcome.is_err() {
            // Release the old blocking open/read before reporting the failure.
            let rescue = rustix::fs::openat(
                rustix::fs::CWD,
                &path,
                rustix::fs::OFlags::WRONLY | rustix::fs::OFlags::NONBLOCK,
                rustix::fs::Mode::empty(),
            )?;
            drop(rescue);
        }
        worker.join().map_err(|_| "worker")??;
        std::fs::remove_dir_all(directory)?;
        assert_eq!(outcome, Ok(Err(Error::Invalid)));
        Ok(())
    }
}
