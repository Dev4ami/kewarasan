# CLAUDE.md — Kewarasan

Konteks project untuk Claude Code. Semua keputusan desain di bawah ini sudah final hasil diskusi — ikuti, jangan didesain ulang kecuali diminta.

## Apa ini

**Kewarasan** (@kewarasan_bot) — Telegram bot mood tracker + web dashboard. Self-hosted, deploy via Coolify di home server Ubuntu. Repo publik: `Dev4ami/kewarasan`.

Konsep inti: user check-in mood beberapa kali sehari (terjadwal + spontan), sistem mengagregasi jadi insight (trend, heatmap, korelasi tag). Filosofi: "emosi itu riak, mood itu arus" — entry individual = noise, agregasi = sinyal. Nyatat harus <5 detik.

## Stack & Arsitektur

- **Rust, SATU binary** — bukan workspace. Tiga task tokio dalam satu proses:
  1. Dispatcher teloxide (long polling, bukan webhook)
  2. Scheduler check-in (loop `tokio::time::interval` 60 detik)
  3. Axum web server (port 3000, env `WEB_PORT`)
- Semua share satu `PgPool`.
- **PostgreSQL + sqlx** — WAJIB pakai sqlx migrations (`sqlx::migrate!("./migrations")` di startup) dan compile-time checked queries (`query!`/`query_as!`) di mana memungkinkan.
- Dependencies inti: `tokio`, `teloxide 0.13 (macros)`, `axum 0.8`, `sqlx 0.8 (runtime-tokio, postgres, chrono, migrate)`, `chrono`, `chrono-tz`, `serde`, `dotenvy`, `tracing`.

## Struktur direktori

```
src/
├── main.rs          # init pool → run migrations → spawn scheduler + web → dispatch bot
├── config.rs        # env vars: DATABASE_URL, TELOXIDE_TOKEN, WEB_PORT
├── db/
│   ├── models.rs    # MoodEntry, Tag, User, CheckinSchedule (derive FromRow)
│   └── queries.rs   # insert_entry, daily_avg, tag_correlation, heatmap, dst
├── bot/
│   ├── handlers.rs  # /start, /waras, /stats, /jadwal
│   ├── callbacks.rs # parsing callback data + rebuild keyboard
│   ├── keyboards.rs # builder inline keyboard
│   └── scheduler.rs # loop check-in terjadwal
└── web/
    ├── routes.rs    # GET /, /api/trend, /api/tags, /api/heatmap
    └── templates/   # HTML + Chart.js
```

## Database schema

Migration awal `migrations/0001_initial.sql` (SUDAH ADA di repo, jangan diubah — perubahan schema = file migration baru bernomor urut):

- `users` — telegram_id BIGINT UNIQUE, timezone TEXT default 'Asia/Jakarta'
- `checkin_schedules` — user_id, local_time TIME (jam lokal user), enabled
- `mood_entries` — score SMALLINT CHECK 1-5, entry_type ENUM('scheduled','spontaneous'), note TEXT nullable, created_at TIMESTAMPTZ. Index (user_id, created_at)
- `tags` — user_id nullable (NULL = tag bawaan sistem), name. Tag bawaan: coding, kerja, sosial, capek, kurang tidur, olahraga, santai, keluarga
- `entry_tags` — junction (entry_id, tag_id)

Aturan penting:
- Simpan waktu SELALU UTC (TIMESTAMPTZ), konversi ke timezone user hanya saat query analisis/display (`AT TIME ZONE`, chrono-tz).
- `AVG(score)` harus di-cast `::numeric` (pelajaran dari bug SUM BIGINT di project sebelumnya).
- Query korelasi tag pakai `HAVING COUNT(*) >= 5` biar tag minim sampel tidak jadi insight palsu.

## Desain flow bot

**Skala mood 1-5** via inline keyboard emoji: 😩 😕 😐 🙂 😄 (jangan 1-10).

**Check-in terjadwal:** scheduler kirim pesan tanya mood → user tap emoji → pesan DI-EDIT (edit_message, bukan kirim baru) jadi pilihan tags (multi-select toggle, tag terpilih diberi ✅) → tap "Selesai" → simpan entry, edit pesan jadi ringkasan. Check-in yang didiamkan: JANGAN follow-up/reminder ulang — data bolong itu normal, bot cerewet = di-mute.

**Input spontan:** command `/waras`, flow sama, `entry_type = 'spontaneous'`, plus opsi "Tambah catatan" (tunggu satu pesan teks berikutnya sebagai note).

**Callback data: STATELESS.** Semua state pilihan di-encode di callback data, format compact:
- `m:4` = pilih score 4
- `m:4:t:1,5` = score 4 + tag id 1 dan 5 terpilih (toggle: tap tag yang sudah ada = hapus dari list)
- `done` / `skip` = finalize
Limit callback data Telegram 64 byte — dengan format ini aman. Tidak ada draft in-memory/DB; bot restart tidak kehilangan state.

**Scheduler:** tiap menit, query `checkin_schedules` yang `local_time` match jam-menit sekarang di timezone masing-masing user (chrono-tz). Jangan pakai crate cron.

## Web dashboard (Fase 2 — kerjakan SETELAH bot stabil)

- Trend line rata-rata mood harian (Chart.js)
- Heatmap hari-dalam-minggu × jam
- Bar chart korelasi tag (avg score per tag, sorted)
- Filter rentang tanggal
- `/stats` di bot (teks + sparkline emoji `▁▂▄▅▇`) dikerjakan DULUAN sebelum dashboard.

## Urutan build (roadmap)

1. `cargo init`, Cargo.toml, config, main.rs skeleton + koneksi DB + migrations jalan
2. `db/`: models + queries (insert entry, daily avg, tag correlation, heatmap)
3. `bot/`: keyboards + handlers `/start` `/waras` + callbacks (bisa dites langsung di Telegram)
4. `/stats` teks
5. `bot/scheduler.rs` + `/jadwal`
6. `web/`: Axum routes + templates Chart.js
7. Dockerfile untuk Coolify (pastikan folder `migrations/` ikut ter-copy ke build context)

## Konvensi

- Bahasa UI bot & dashboard: **Bahasa Indonesia casual** (bukan formal). Nada boleh sedikit humor sesuai karakter "Kewarasan" (contoh respon mood 5: "Alhamdulillah masih waras ✨").
- Error handling: `anyhow` di main/handlers, jangan `unwrap()` di path runtime.
- Logging pakai `tracing`, bukan `println!`.
- Commit Cargo.lock (ini binary project).
- `.env` JANGAN pernah di-commit; `.env.example` sudah ada.

## Environment

```
DATABASE_URL=postgres://user:pass@host:5432/kewarasan
TELOXIDE_TOKEN=dari @BotFather
WEB_PORT=3000
```

Dev di Windows laptop / Termux Android, deploy ke Ubuntu home server via Coolify (git push → rebuild container).
