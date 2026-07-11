// Parsing callback data STATELESS. Tidak ada draft in-memory/DB;
// bot restart tidak kehilangan state.
//
// Format:
//   m:4          -> pilih score 4 (belum ada tag)
//   m:4:t:1,5    -> score 4, tag 1 & 5 terpilih (toggle)
//   m:4:t:       -> score 4, belum ada tag
//   ok:4:1,5     -> finalize: score 4, simpan dengan tag 1 & 5

#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    Score(i16),
    Toggle { score: i16, tags: Vec<i64> },
    Finalize { score: i16, tags: Vec<i64> },
}

pub fn parse(data: &str) -> Option<Action> {
    if let Some(rest) = data.strip_prefix("ok:") {
        let (score, csv) = rest.split_once(':')?;
        return Some(Action::Finalize {
            score: score.parse().ok()?,
            tags: parse_csv(csv),
        });
    }
    if let Some(rest) = data.strip_prefix("m:") {
        if let Some((score, tail)) = rest.split_once(":t:") {
            return Some(Action::Toggle {
                score: score.parse().ok()?,
                tags: parse_csv(tail),
            });
        }
        return Some(Action::Score(rest.parse().ok()?));
    }
    None
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
    fn parse_score() {
        assert_eq!(parse("m:4"), Some(Action::Score(4)));
    }

    #[test]
    fn parse_toggle_empty() {
        assert_eq!(
            parse("m:4:t:"),
            Some(Action::Toggle {
                score: 4,
                tags: vec![]
            })
        );
    }

    #[test]
    fn parse_toggle_ids() {
        assert_eq!(
            parse("m:4:t:1,5"),
            Some(Action::Toggle {
                score: 4,
                tags: vec![1, 5]
            })
        );
    }

    #[test]
    fn parse_finalize() {
        assert_eq!(
            parse("ok:3:2,7"),
            Some(Action::Finalize {
                score: 3,
                tags: vec![2, 7]
            })
        );
    }

    #[test]
    fn parse_junk() {
        assert_eq!(parse("garbage"), None);
    }
}
