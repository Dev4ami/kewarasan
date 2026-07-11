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
    /// Kredensial HTTP Basic Auth dashboard. `None` = akses terbuka
    /// (biar dev lokal tanpa ribet). Set `DASHBOARD_PASSWORD` di prod.
    pub dashboard_auth: Option<DashboardAuth>,
}

#[derive(Debug, Clone)]
pub struct DashboardAuth {
    pub user: String,
    pub password: String,
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
            dashboard_auth: std::env::var("DASHBOARD_PASSWORD").ok().map(|password| {
                DashboardAuth {
                    user: std::env::var("DASHBOARD_USER")
                        .unwrap_or_else(|_| "kewarasan".to_string()),
                    password,
                }
            }),
        })
    }
}
