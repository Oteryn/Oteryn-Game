use oteryn_platform_client::updates::{UpdateClient, UpdateConfig};
#[test]
fn update_origin_requires_a_build_pinned_root() {
    assert!(UpdateConfig::new("stable", b"", "windows", "x86_64").is_err());
    let _ = std::mem::size_of::<UpdateClient>();
}

use async_trait::async_trait;
use bytes::Bytes;
use std::{path::PathBuf, pin::Pin, sync::Arc};
use tough::{Transport, TransportError, TransportErrorKind};
use url::Url;
#[derive(Clone, Debug)]
struct Fixture {
    corrupted: bool,
    requests: Arc<std::sync::Mutex<Vec<String>>>,
}
#[async_trait]
impl Transport for Fixture {
    async fn fetch(
        &self,
        url: Url,
    ) -> Result<
        Pin<Box<dyn futures::Stream<Item = Result<Bytes, TransportError>> + Send + Sync>>,
        TransportError,
    > {
        self.requests
            .lock()
            .map_err(|_| TransportError::new(TransportErrorKind::Other, &url))?
            .push(url.to_string());
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/contracts/fixtures/client-update-origin-v1/origin-v1/stable");
        let name = url
            .path()
            .rsplit('/')
            .next()
            .ok_or_else(|| TransportError::new(TransportErrorKind::Other, &url))?;
        let mut bytes = if url.path().contains("/metadata/") {
            std::fs::read(base.join("metadata").join(name))
                .map_err(|_| TransportError::new(TransportErrorKind::FileNotFound, &url))?
        } else if name.ends_with("policy-v1.json") {
            std::fs::read(base.join("policy-v1.json"))
                .map_err(|_| TransportError::new(TransportErrorKind::Other, &url))?
        } else {
            b"package".to_vec()
        };
        if self.corrupted && name.ends_with("client.zip") {
            bytes[0] ^= 1;
        }
        Ok(Box::pin(futures::stream::once(std::future::ready(Ok(
            Bytes::from(bytes),
        )))))
    }
}
#[test]
fn signed_origin_checks_versions_downloads_and_refuses_corruption()
-> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Runtime::new()?.block_on(async {
  let root=include_bytes!("../../../docs/contracts/fixtures/client-update-origin-v1/origin-v1/stable/metadata/1.root.json");
  let folder=std::env::temp_dir().join(format!("oteryn-update-fixture-{}",std::process::id()));
  let _=std::fs::remove_dir_all(&folder);std::fs::create_dir_all(&folder)?;
  let requests=Arc::new(std::sync::Mutex::new(Vec::new()));
  let client=UpdateClient::check_with_transport(UpdateConfig::new("stable",root,"windows","x86_64")?,&folder,Fixture{corrupted:false,requests:requests.clone()}).await?;
  assert_eq!(client.policy.current_release_id,"rel-a");assert_eq!(client.length,7);
  client.download(&folder.join("package.zip")).await?;
  assert_eq!(std::fs::read(folder.join("package.zip"))?,b"package");
  let bad=UpdateClient::check_with_transport(UpdateConfig::new("stable",root,"windows","x86_64")?,&folder,Fixture{corrupted:true,requests}).await?;
  assert!(bad.download(&folder.join("bad.zip")).await.is_err());
  assert!(!folder.join("bad.zip").exists());assert!(!folder.join("bad.partial").exists());
  let mut untrusted=root.to_vec(); let pos=untrusted.iter().position(|b|*b==b'a').ok_or("fixture")?;untrusted[pos]=b'b';
  assert!(UpdateClient::check_with_transport(UpdateConfig::new("stable",&untrusted,"windows","x86_64")?,&folder.join("wrong"),Fixture{corrupted:false,requests:Default::default()}).await.is_err());
  std::fs::remove_dir_all(folder)?;
  Ok::<(),Box<dyn std::error::Error>>(())
 })
}
