// Builder inline keyboard. Skala mood 1-5: 😩 😕 😐 🙂 😄
// Callback STATELESS: state pilihan + sumber (p/s) di-encode di callback data.

use crate::db::models::Tag;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

/// (emoji, score) urut 1..5.
pub const MOODS: [(&str, i16); 5] = [
    ("😩", 1),
    ("😕", 2),
    ("😐", 3),
    ("🙂", 4),
    ("😄", 5),
];

pub fn mood_emoji(score: i16) -> &'static str {
    MOODS
        .iter()
        .find(|(_, s)| *s == score)
        .map(|(e, _)| *e)
        .unwrap_or("❓")
}

/// Flag sumber di callback: scheduled=s, spontaneous=p.
fn flag(scheduled: bool) -> &'static str {
    if scheduled {
        "s"
    } else {
        "p"
    }
}

/// Baris 5 emoji + baris Batal. Callback `m:<flag>:<score>`, batal = `x`.
pub fn mood_keyboard(scheduled: bool) -> InlineKeyboardMarkup {
    let f = flag(scheduled);
    let emojis: Vec<InlineKeyboardButton> = MOODS
        .iter()
        .map(|(emoji, score)| {
            InlineKeyboardButton::callback(emoji.to_string(), format!("m:{f}:{score}"))
        })
        .collect();
    InlineKeyboardMarkup::new(vec![
        emojis,
        vec![InlineKeyboardButton::callback("✕ Batal".to_string(), "x".to_string())],
    ])
}

/// Grid tag (3 per baris) + baris Selesai/Batal.
/// Tag terpilih diberi ✅. Callback tiap tag = state HASIL kalau tombol ditekan
/// (toggle), format `m:<flag>:<score>:t:<csv>`. Selesai = `ok:<flag>:<score>:<csv>`.
pub fn tags_keyboard(
    scheduled: bool,
    score: i16,
    selected: &[i64],
    all_tags: &[Tag],
) -> InlineKeyboardMarkup {
    let f = flag(scheduled);
    let mut rows: Vec<Vec<InlineKeyboardButton>> = Vec::new();

    for chunk in all_tags.chunks(3) {
        let row = chunk
            .iter()
            .map(|tag| {
                let label = if selected.contains(&tag.id) {
                    format!("✅ {}", tag.name)
                } else {
                    tag.name.clone()
                };
                InlineKeyboardButton::callback(
                    label,
                    format!("m:{f}:{score}:t:{}", toggle_csv(selected, tag.id)),
                )
            })
            .collect();
        rows.push(row);
    }

    rows.push(vec![
        InlineKeyboardButton::callback(
            "✔️ Selesai".to_string(),
            format!("ok:{f}:{score}:{}", csv(selected)),
        ),
        InlineKeyboardButton::callback("✕ Batal".to_string(), "x".to_string()),
    ]);

    InlineKeyboardMarkup::new(rows)
}

fn csv(ids: &[i64]) -> String {
    ids.iter()
        .map(|i| i.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

/// CSV hasil toggle `id` pada `selected` (sorted, biar stabil).
fn toggle_csv(selected: &[i64], id: i64) -> String {
    let mut v: Vec<i64> = if selected.contains(&id) {
        selected.iter().copied().filter(|x| *x != id).collect()
    } else {
        let mut s = selected.to_vec();
        s.push(id);
        s
    };
    v.sort_unstable();
    csv(&v)
}
