-- Tambahan tag bawaan sistem: kondisi mental, spiritual & relasi, situasional.
-- (0001 seed: coding, kerja, sosial, capek, kurang tidur, olahraga, santai, keluarga)

INSERT INTO tags (user_id, name) VALUES
    -- kondisi mental
    (NULL, 'overthinking'),
    (NULL, 'cemas'),
    (NULL, 'stres'),
    (NULL, 'kesepian'),
    -- spiritual & relasi
    (NULL, 'ibadah'),
    (NULL, 'pasangan'),
    (NULL, 'me-time'),
    -- situasional
    (NULL, 'uang'),
    (NULL, 'deadline'),
    (NULL, 'konflik');
