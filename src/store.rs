
use std::collections::HashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};

use tokio::{
    net::{UnixListener, UnixStream},
    io::{AsyncReadExt, AsyncWriteExt},
    sync::RwLock,
    task,
    time::{sleep, Duration, Instant},
};

use crate::protocol::{Request, Response};


// #[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Debug, Clone)]
struct SecretEntry {
    value: String,
    expires_at: Instant,
}

#[derive(Debug)]
pub struct SecretStore {
    password: String,
    secrets: Arc<RwLock<HashMap<String, SecretEntry>>>,
    ttl: Duration,
}

impl SecretStore {
    pub fn new(password: &str, ttl_secs: u64) -> Arc<Self> {
        Arc::new(Self {
            password: password.to_string(),
            secrets: Arc::new(RwLock::new(HashMap::new())),
            ttl: Duration::from_secs(ttl_secs),
        })
    }

    pub async fn set(&self, key: String, value: String) {
        let entry = SecretEntry {
            value,
            expires_at: Instant::now() + self.ttl,
        };
        self.secrets.write().await.insert(key, entry);
    }

    pub async fn get(&self, key: &str) -> Option<String> {
        let mut secrets = self.secrets.write().await;
        if let Some(entry) = secrets.get(key) {
            if Instant::now() < entry.expires_at {
                return Some(entry.value.clone());
            } else {
                secrets.remove(key);
            }
        }
        None
    }

    pub async fn cleanup_expired(self: Arc<Self>) {
        let secrets = self.secrets.clone();
        task::spawn(async move {
            loop {
                sleep(Duration::from_secs(5)).await;
                let now = Instant::now();
                let mut secrets = secrets.write().await;
                secrets.retain(|_, entry| entry.expires_at > now);
            }
        });
    }
}


