<div align="center">

# 🧠 Kewarasan

**Pantau kewarasanmu sebelum orang lain yang memantaunya.**

*Telegram bot untuk mood tracking harian — karena waras itu perlu di-monitoring.*

[![Rust](https://img.shields.io/badge/Rust-🦀-orange?style=flat-square)](https://www.rust-lang.org/)
[![Telegram](https://img.shields.io/badge/Bot-@kewarasan__bot-2CA5E0?style=flat-square&logo=telegram)](https://t.me/kewarasan_bot)
[![PostgreSQL](https://img.shields.io/badge/DB-PostgreSQL-336791?style=flat-square&logo=postgresql&logoColor=white)](https://www.postgresql.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-green?style=flat-square)](LICENSE)

*kewarasan (n., Indonesian): sanity — the state of being sane.*

</div>

---

## Apa ini?

Kewarasan adalah bot Telegram yang nge-*vibe check* kamu beberapa kali sehari, nyatat mood-mu dalam hitungan detik, lalu diam-diam ngumpulin data sampai bisa bilang hal-hal seperti:

> *"Tiap tag `kurang tidur` muncul, rata-rata mood-mu anjlok 1.5 poin."*
>
> *"Senin sore kamu hampir selalu merah. Mungkin bukan kamu masalahnya. Mungkin Senin."*

Emosi itu riak. Mood itu arusnya. Kewarasan nggak peduli sama riak — dia memetakan arus.

## Fitur

- ⏰ **Check-in terjadwal** — bot nanya duluan, kamu tinggal tap emoji. 5 detik, selesai.
- ⚡ **Input spontan** — ada kejadian? `/waras`, catat, lanjut hidup.
- 🏷 **Tags konteks** — `coding`, `capek`, `kurang tidur`, `olahraga`, atau bikin sendiri.
- 📊 **Statistik di chat** — `/stats` buat lihat trend mingguan langsung di Telegram.
- 📈 **Web dashboard** — trend line, heatmap hari × jam, dan korelasi tag. Responsif di HP, bisa di-*Add to Home Screen* (PWA).
- 🔐 **Login opsional** — dashboard bisa dikunci password (cookie session) kalau diekspos ke internet.
- 🔒 **Self-hosted** — data mood-mu tinggal di server-mu. Kewarasan itu privasi.

## Cara pakai

| Command | Fungsi |
|---|---|
| `/start` | Daftar & atur jadwal check-in |
| `/waras` | Catat mood sekarang |
| `/stats` | Seberapa waras kamu minggu ini |
| `/jadwal` | Atur jam check-in harian |

Skala mood: 😩 😕 😐 🙂 😄 — dari *"berat banget"* sampai *"alhamdulillah masih waras ✨"*

## Stack

- **[Rust](https://www.rust-lang.org/)** 🦀 — satu binary, tiga nyawa: bot, scheduler, web server
- **[teloxide](https://github.com/teloxide/teloxide)** — Telegram bot framework
- **[Axum](https://github.com/tokio-rs/axum)** — web dashboard
- **[PostgreSQL](https://www.postgresql.org/)** + **[sqlx](https://github.com/launchbadge/sqlx)** — storage & compile-time checked queries

## Menjalankan sendiri

```bash
# 1. Clone
git clone https://github.com/Dev4ami/kewarasan.git
cd kewarasan

# 2. Siapkan environment
cp .env.example .env
# isi DATABASE_URL & TELOXIDE_TOKEN

# 3. Jalankan (migrations otomatis)
cargo run --release
```

Dashboard tersedia di `http://localhost:3000`. Bot langsung jalan via long polling — nggak perlu webhook, nggak perlu buka port masuk.

### Environment variables

| Variable | Keterangan |
|---|---|
| `DATABASE_URL` | `postgres://user:pass@localhost/kewarasan` |
| `TELOXIDE_TOKEN` | Token dari [@BotFather](https://t.me/BotFather) |
| `WEB_PORT` | Port dashboard (default: `3000`, image Docker: `8775`) |
| `DASHBOARD_PASSWORD` | Opsional — set = dashboard butuh login. Kosong = terbuka (dev lokal) |
| `DASHBOARD_USER` | Opsional — username login (default: `kewarasan`) |

### Deploy via Docker / Coolify

```bash
docker build -t kewarasan .
docker run -d --env-file .env -p 8775:8775 kewarasan
```

Ada `Dockerfile` multi-stage (build sqlx offline, runtime `debian-slim`). Di [Coolify](https://coolify.io/) tinggal `git push` → rebuild container. Bot ini sudah live jalan begini di home server. 🟢

## Filosofi

Mood tracking bukan soal bahagia terus. Itu soal **sadar** — tahu polamu sendiri, tahu apa yang ngangkat dan apa yang ngedrop, sebelum burnout yang ngasih tahu duluan.

Nyatat harus lebih cepat dari mikir. Analisis biar mesin yang kerjakan.

## Roadmap

- [x] Desain schema & arsitektur
- [x] Bot core: check-in, tags, `/stats`
- [x] Scheduler check-in harian (timezone-aware)
- [x] Web dashboard: trend, heatmap, korelasi tag
- [x] Dashboard mobile + PWA + login opsional
- [x] Deploy Docker/Coolify — **live di home server** 🟢

## License

MIT — pakai, fork, modif sesukamu. Yang penting tetap waras. 🧠

---

<div align="center">
<sub>Dibuat dengan 🦀 dan sisa-sisa kewarasan oleh <a href="https://github.com/Dev4ami">Dev4ami</a></sub>
</div>
