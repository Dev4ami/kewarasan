// Query database. Diisi di Step 2 (insert_entry, daily_avg, tag_correlation,
// heatmap, dst). Semua pakai compile-time checked queries (query!/query_as!)
// + offline cache (.sqlx) supaya Docker build tidak butuh DB hidup.
