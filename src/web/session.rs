// Session store in-memory (HashMap token → expiry).
// Personal use — session lost saat restart itu OK, tinggal login lagi.
// Access refresh TTL supaya user aktif tidak tiba-tiba logout.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::RngCore;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub struct Sessions {
    inner: Mutex<HashMap<String, Instant>>,
    ttl: Duration,
}

impl Sessions {
    pub fn new(ttl: Duration) -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
            ttl,
        }
    }

    pub fn ttl(&self) -> Duration {
        self.ttl
    }

    /// Generate token 32 byte random, simpan expiry = now + ttl.
    pub fn create(&self) -> String {
        let mut bytes = [0u8; 32];
        rand::rng().fill_bytes(&mut bytes);
        let token = URL_SAFE_NO_PAD.encode(bytes);
        let expiry = Instant::now() + self.ttl;
        self.inner.lock().unwrap().insert(token.clone(), expiry);
        token
    }

    /// Valid kalau ada + belum expired. Sekalian refresh expiry supaya
    /// user yang aktif gak tiba-tiba logout saat TTL habis.
    pub fn is_valid(&self, token: &str) -> bool {
        let mut store = self.inner.lock().unwrap();
        let now = Instant::now();
        match store.get(token).copied() {
            Some(expiry) if expiry > now => {
                store.insert(token.into(), now + self.ttl);
                true
            }
            Some(_) => {
                store.remove(token);
                false
            }
            None => false,
        }
    }

    pub fn remove(&self, token: &str) {
        self.inner.lock().unwrap().remove(token);
    }
}
