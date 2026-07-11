// Kewarasan — satu binary, tiga task tokio:
//   1. Dispatcher teloxide (long polling)
//   2. Scheduler check-in (loop interval 60 detik)
//   3. Axum web server
// Semua share satu PgPool.

// TODO(step-1): hapus setelah query & handler beneran mengisi semua modul.
#![allow(dead_code)]

mod bot;
mod config;
mod db;
mod web;

use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use tracing::info;
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    // Stack thread-main Windows cuma 1 MiB — kekecilan buat gabungan future
    // teloxide + axum + sqlx (bikin stack overflow saat startup). Jalankan
    // runtime di thread ber-stack besar. Cross-platform, aman juga di Linux.
    std::thread::Builder::new()
        .name("kewarasan-runtime".into())
        .stack_size(16 * 1024 * 1024)
        .spawn(run)?
        .join()
        .expect("thread runtime panik")
}

fn run() -> Result<()> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async_main())
}

async fn async_main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cfg = config::Config::from_env()?;

    info!("Menghubungkan ke database…");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&cfg.database_url)
        .await?;

    info!("Menjalankan migrations…");
    sqlx::migrate!("./migrations").run(&pool).await?;

    let bot = teloxide::Bot::new(cfg.teloxide_token);

    // Task 2: scheduler check-in.
    let sched_bot = bot.clone();
    let sched_pool = pool.clone();
    tokio::spawn(async move {
        if let Err(e) = bot::scheduler::run(sched_bot, sched_pool).await {
            tracing::error!("scheduler mati: {e:?}");
        }
    });

    // Task 3: web dashboard.
    let web_pool = pool.clone();
    let web_port = cfg.web_port;
    tokio::spawn(async move {
        if let Err(e) = web::serve(web_pool, web_port).await {
            tracing::error!("web server mati: {e:?}");
        }
    });

    // Task 1: dispatcher bot (blocking — jalan sampai proses dimatikan).
    info!("Bot Kewarasan jalan 🧠");
    bot::run(bot, pool).await?;

    Ok(())
}
