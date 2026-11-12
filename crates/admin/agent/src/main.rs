use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use oteryn_admin_agent::{
    channel::{Agent, HttpsTransport, SystemClock},
    config::Config,
    policy::Policy,
    store::PostgresStore,
};
use oteryn_admin_contracts::WireError;
#[tokio::main]
async fn main() -> Result<(), WireError> {
    let path = std::env::args_os()
        .nth(1)
        .ok_or("usage: oteryn-admin-agent CONFIG.toml")?;
    let config = Config::parse(&std::fs::read_to_string(path)?)?;
    let policy = Policy::parse(&std::fs::read_to_string(&config.policy_file_path)?)?;
    let secret = std::fs::read_to_string(&config.signing_key_path)?;
    let bytes: [u8; 32] = URL_SAFE_NO_PAD
        .decode(secret.trim())?
        .try_into()
        .map_err(|_| "agent signing seed must be 32 bytes")?;
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&bytes);
    let url = std::env::var(&config.db_url_env_name)
        .map_err(|_| "configured DB URL environment variable missing")?;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .map_err(|_| "admin DB connection failed")?;
    let transport = HttpsTransport::new(&config)?;
    let mut agent = Agent {
        config,
        policy,
        transport,
        store: PostgresStore { pool },
        clock: SystemClock,
        signing_key,
    };
    agent
        .run()
        .await
        .map_err(|code| format!("admin agent stopped: {code:?}").into())
}
