use sqlx::PgPool;
use std::time::Duration;
use tokio::time;
use tracing::debug;

/// Loop check-in terjadwal: tiap 60 detik cek jadwal yang match jam-menit
/// sekarang di timezone masing-masing user. Logika diisi di Step 5.
pub async fn run(_pool: PgPool) -> anyhow::Result<()> {
    let mut ticker = time::interval(Duration::from_secs(60));
    loop {
        ticker.tick().await;
        debug!("scheduler tick — belum ada jadwal yang diproses");
    }
}
