// Parsing callback data STATELESS. Tidak ada draft in-memory/DB;
// bot restart tidak kehilangan state.
//
// Flag sumber di-encode juga: p = spontaneous (/waras), s = scheduled (auto).
//
// Format:
//   m:p:4        -> pilih score 4, sumber spontan (belum ada tag)
//   m:s:4:t:1,5  -> score 4 (terjadwal), tag 1 & 5 terpilih (toggle)
//   m:p:4:t:     -> score 4, belum ada tag
//   ok:p:4:1,5   -> finalize: score 4, simpan dengan tag 1 & 5
//   x            -> batal, buang tanpa simpan

#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    Score { scheduled: bool, score: i16 },
    Toggle { scheduled: bool, score: i16, tags: Vec<i64> },
    Finalize { scheduled: bool, score: i16, tags: Vec<i64> },
    Cancel,
}

pub fn parse(data: &str) -> Option<Action> {
    if data == "x" {
        return Some(Action::Cancel);
    }
    if let Some(rest) = data.strip_prefix("ok:") {
        // rest = "p:4:1,5"
        let (flag, rest) = rest.split_once(':')?;
        let scheduled = parse_flag(flag)?;
        let (score, csv) = rest.split_once(':')?;
        return Some(Action::Finalize {
            scheduled,
            score: score.parse().ok()?,
            tags: parse_csv(csv),
        });
    }
    if let Some(rest) = data.strip_prefix("m:") {
        // rest = "p:4" atau "p:4:t:1,5"
        let (flag, rest) = rest.split_once(':')?;
        let scheduled = parse_flag(flag)?;
        if let Some((score, tail)) = rest.split_once(":t:") {
            return Some(Action::Toggle {
                scheduled,
                score: score.parse().ok()?,
                tags: parse_csv(tail),
            });
        }
        return Some(Action::Score {
            scheduled,
            score: rest.parse().ok()?,
        });
    }
    None
}

fn parse_flag(f: &str) -> Option<bool> {
    match f {
        "s" => Some(true),
        "p" => Some(false),
        _ => None,
    }
}

fn parse_csv(csv: &str) -> Vec<i64> {
    csv.split(',')
        .map(str::trim)
        .filter(|x| !x.is_empty())
        .filter_map(|x| x.parse().ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_score_spontan() {
        assert_eq!(
            parse("m:p:4"),
            Some(Action::Score {
                scheduled: false,
                score: 4
            })
        );
    }

    #[test]
    fn parse_score_scheduled() {
        assert_eq!(
            parse("m:s:4"),
            Some(Action::Score {
                scheduled: true,
                score: 4
            })
        );
    }

    #[test]
    fn parse_toggle_empty() {
        assert_eq!(
            parse("m:p:4:t:"),
            Some(Action::Toggle {
                scheduled: false,
                score: 4,
                tags: vec![]
            })
        );
    }

    #[test]
    fn parse_toggle_ids() {
        assert_eq!(
            parse("m:s:4:t:1,5"),
            Some(Action::Toggle {
                scheduled: true,
                score: 4,
                tags: vec![1, 5]
            })
        );
    }

    #[test]
    fn parse_finalize() {
        assert_eq!(
            parse("ok:p:3:2,7"),
            Some(Action::Finalize {
                scheduled: false,
                score: 3,
                tags: vec![2, 7]
            })
        );
    }

    #[test]
    fn parse_cancel() {
        assert_eq!(parse("x"), Some(Action::Cancel));
    }

    #[test]
    fn parse_junk() {
        assert_eq!(parse("garbage"), None);
        assert_eq!(parse("m:z:4"), None);
    }
}
