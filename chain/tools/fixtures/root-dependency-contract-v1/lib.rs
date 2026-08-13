use futures_util::future;
use parity_scale_codec::{Decode, Encode};
use rusqlite::hooks::{Authorization, TransactionOperation};
use rusqlite::limits::Limit;
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::Duration;
use tokio::sync::Mutex;
use url::Url;
use uuid::Uuid;

/// Compile-time witnesses for both Subxt crates in the future root dependency set.
pub type RpcClient = subxt::OnlineClient<subxt::config::PolkadotConfig>;
pub type Sr25519Signer = subxt_signer::sr25519::Keypair;

/// A small SCALE/type-metadata payload used to prove both derive dependencies.
#[derive(Clone, Debug, Decode, Deserialize, Encode, PartialEq, Eq, Serialize, TypeInfo)]
pub struct DependencyProbe {
    pub id: [u8; 16],
    pub canonical_url: String,
    pub digest: [u8; 32],
    pub process_id: u32,
    pub directory_mode: u32,
}

/// Exercises the non-Subxt dependencies without performing any network access.
pub async fn dependency_probe(input: &str) -> Result<DependencyProbe, url::ParseError> {
    let canonical_url = Url::parse(input)?.to_string();
    let guarded = Mutex::new(canonical_url.as_bytes().to_vec());

    tokio::time::sleep(Duration::ZERO).await;
    let bytes = future::ready(guarded.into_inner()).await;
    let digest: [u8; 32] = Sha256::digest(bytes).into();

    Ok(DependencyProbe {
        id: *Uuid::new_v4().as_bytes(),
        canonical_url,
        digest,
        process_id: rustix::process::getpid()
            .as_raw_nonzero()
            .get()
            .unsigned_abs(),
        directory_mode: rustix::fs::Mode::RWXU.bits(),
    })
}

/// Exercises every selected rusqlite feature and the existing JSON contract.
pub fn root_graph_probe(value: &DependencyProbe) -> rusqlite::Result<String> {
    fn allow_all(_: rusqlite::hooks::AuthContext<'_>) -> Authorization {
        Authorization::Allow
    }

    let connection = rusqlite::Connection::open_in_memory()?;
    connection.load_extension_disable()?;
    let _sql_limit = connection.limit(Limit::SQLITE_LIMIT_SQL_LENGTH)?;
    connection.authorizer(Some(allow_all))?;

    let _patched_commit_operation = TransactionOperation::Commit;
    serde_json::to_string(value)
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))
}
