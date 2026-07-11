use anyhow::{Context, Result};

/// Telegram id pemilik bot — dapet sapaan spesial di bot & jadi user default
/// dashboard web. Dipakai lintas modul (bot + web) biar magic number-nya tunggal.
pub const OWNER_ID: i64 = 1069319412;

/// Konfigurasi dari environment variables.
#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub teloxide_token: String,
    pub web_port: u16,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            database_url: std::env::var("DATABASE_URL")
                .context("DATABASE_URL belum diset")?,
            teloxide_token: std::env::var("TELOXIDE_TOKEN")
                .context("TELOXIDE_TOKEN belum diset")?,
            web_port: std::env::var("WEB_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3000),
        })
    }
}
