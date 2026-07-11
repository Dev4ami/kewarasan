-- Tandai kapan user pertama kali onboarding. Dipakai buat pasang jadwal
-- default SEKALI (bukan tiap /start) — biar penghapusan jadwal user tetap
-- dihormati, gak ke-re-add.
ALTER TABLE users ADD COLUMN onboarded_at TIMESTAMPTZ;
