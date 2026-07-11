-- Cegah jadwal duplikat (user + jam yang sama). Diperlukan buat ON CONFLICT
-- saat nambah jadwal via /jadwal.
CREATE UNIQUE INDEX idx_schedules_user_time ON checkin_schedules(user_id, local_time);
