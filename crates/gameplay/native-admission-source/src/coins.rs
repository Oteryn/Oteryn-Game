//! Platform coin wire v1. Wallet state is never reconstructed in Game.
use crate::{TransientCapacity, descriptor::ProducerDescriptor};
use std::{future::Future, pin::Pin};
#[path = "coins/config.rs"]
mod config;
#[path = "coins/request.rs"]
mod request;
#[path = "coins/response.rs"]
mod response;
pub use request::Request;
pub use response::{Balance, Entry, History, Mutation, Receipt, Reply, decode};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Invalid,
    Unavailable,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "coin service {:?}", self)
    }
}
impl std::error::Error for Error {}
impl From<serde_json::Error> for Error {
    fn from(_: serde_json::Error) -> Self {
        Self::Invalid
    }
}
type FutureReply<'a> = Pin<Box<dyn Future<Output = Result<(u16, Vec<u8>), Error>> + Send + 'a>>;
pub trait Transport: Send {
    fn exchange<'a>(&'a mut self, method: &'a str, path: &'a str, body: &'a str)
    -> FutureReply<'a>;
}
pub struct MtlsTransport {
    descriptor: ProducerDescriptor,
    capacity: TransientCapacity,
}
impl MtlsTransport {
    pub fn new(descriptor: ProducerDescriptor) -> Self {
        Self {
            descriptor,
            capacity: TransientCapacity::new(),
        }
    }
}
impl Transport for MtlsTransport {
    fn exchange<'a>(
        &'a mut self,
        method: &'a str,
        path: &'a str,
        body: &'a str,
    ) -> FutureReply<'a> {
        Box::pin(async move {
            let mut permit = self.capacity.try_queue().map_err(|_| Error::Unavailable)?;
            permit.try_activate().map_err(|_| Error::Unavailable)?;
            crate::http1_mtls::exchange_coins(&self.descriptor, method, path, body, &mut permit)
                .await
                .map_err(|_| Error::Unavailable)
        })
    }
}
pub struct ReceiptProof {
    request: Request,
    mutation: Mutation,
}
impl ReceiptProof {
    pub fn request(&self) -> &Request {
        &self.request
    }
    pub fn receipt(&self) -> &Receipt {
        &self.mutation.receipt
    }
    pub fn canonical(&self) -> Result<String, Error> {
        Ok(serde_json::to_string(&self.mutation)?)
    }
}
pub struct CoinClient {
    market_enabled: bool,
    transfers_enabled: bool,
    transport: Box<dyn Transport>,
}
impl CoinClient {
    pub fn with_market_enabled(mut self, enabled: bool) -> Self {
        self.market_enabled = enabled;
        self
    }
    pub fn with_transfers_enabled(mut self, enabled: bool) -> Self {
        self.transfers_enabled = enabled;
        self
    }
    pub fn transfers_enabled(&self) -> bool {
        self.transfers_enabled
    }
    pub fn market_enabled(&self) -> bool {
        self.market_enabled
    }
    pub fn new(transport: Box<dyn Transport>) -> Self {
        Self {
            transport,
            market_enabled: false,
            transfers_enabled: false,
        }
    }
    pub async fn receipt(
        &mut self,
        request: &Request,
    ) -> Result<Result<ReceiptProof, String>, Error> {
        match self.execute(request).await? {
            Reply::Mutation(mutation) => Ok(Ok(ReceiptProof {
                request: request.clone(),
                mutation,
            })),
            Reply::Refused(code) => Ok(Err(code)),
            _ => Err(Error::Invalid),
        }
    }
    pub async fn execute(&mut self, request: &Request) -> Result<Reply, Error> {
        let (method, path, body) = request.wire()?;
        let (status, bytes) = self.transport.exchange(method, &path, &body).await?;
        decode(request, status, &bytes)
    }
}
#[cfg(test)]
#[path = "coins/tests.rs"]
mod tests;
