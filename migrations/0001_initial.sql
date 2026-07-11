-- Kewarasan — schema awal.
-- Semua waktu disimpan UTC (TIMESTAMPTZ). Konversi ke timezone user
-- hanya saat query analisis/display.

-- =========================================================
-- users
-- =========================================================
CREATE TABLE users (
    id          BIGSERIAL   PRIMARY KEY,
    telegram_id BIGINT      NOT NULL UNIQUE,
    timezone    TEXT        NOT NULL DEFAULT 'Asia/Jakarta',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- =========================================================
-- checkin_schedules — jadwal check-in terjadwal per user
-- local_time = jam lokal user (timezone dari users.timezone)
-- =========================================================
CREATE TABLE checkin_schedules (
    id         BIGSERIAL PRIMARY KEY,
    user_id    BIGINT    NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    local_time TIME      NOT NULL,
    enabled    BOOLEAN   NOT NULL DEFAULT TRUE
);
CREATE INDEX idx_schedules_user ON checkin_schedules(user_id);

-- =========================================================
-- mood_entries
-- =========================================================
CREATE TYPE entry_type AS ENUM ('scheduled', 'spontaneous');

CREATE TABLE mood_entries (
    id         BIGSERIAL   PRIMARY KEY,
    user_id    BIGINT      NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    score      SMALLINT    NOT NULL CHECK (score BETWEEN 1 AND 5),
    entry_type entry_type  NOT NULL,
    note       TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_entries_user_created ON mood_entries(user_id, created_at);

-- =========================================================
-- tags — user_id NULL = tag bawaan sistem
-- =========================================================
CREATE TABLE tags (
    id      BIGSERIAL PRIMARY KEY,
    user_id BIGINT    REFERENCES users(id) ON DELETE CASCADE,
    name    TEXT      NOT NULL
);
-- Tag bawaan (user_id NULL) unik per nama; tag user unik per (user, nama).
CREATE UNIQUE INDEX idx_tags_default_name ON tags(name)          WHERE user_id IS NULL;
CREATE UNIQUE INDEX idx_tags_user_name    ON tags(user_id, name) WHERE user_id IS NOT NULL;

-- =========================================================
-- entry_tags — junction mood_entries <-> tags
-- =========================================================
CREATE TABLE entry_tags (
    entry_id BIGINT NOT NULL REFERENCES mood_entries(id) ON DELETE CASCADE,
    tag_id   BIGINT NOT NULL REFERENCES tags(id)         ON DELETE CASCADE,
    PRIMARY KEY (entry_id, tag_id)
);

-- =========================================================
-- Seed tag bawaan sistem
-- =========================================================
INSERT INTO tags (user_id, name) VALUES
    (NULL, 'coding'),
    (NULL, 'kerja'),
    (NULL, 'sosial'),
    (NULL, 'capek'),
    (NULL, 'kurang tidur'),
    (NULL, 'olahraga'),
    (NULL, 'santai'),
    (NULL, 'keluarga');
