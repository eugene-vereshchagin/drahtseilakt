// The ball each sector of the original levels begins with (crates/level/data/sector-balls.txt, compiled in): the kind the player
// has when passing the sector's checkpoint.

const TABLE: &str = include_str!("../data/sector-balls.txt");

/// The balls of original level `level` by sector (index 0 is sector 1, which takes the level's start ball: none here).
pub fn original(level: u32) -> Vec<Option<String>> {
    let mut v: Vec<Option<String>> = Vec::new();
    for line in TABLE
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
    {
        let mut w = line.split_whitespace();
        let (Some(l), Some(s), Some(b)) = (w.next(), w.next(), w.next()) else {
            continue;
        };
        let (Ok(l), Ok(s)) = (l.parse::<u32>(), s.parse::<usize>()) else {
            continue;
        };
        if l != level || s == 0 {
            continue;
        }
        if v.len() < s {
            v.resize(s, None);
        }
        v[s - 1] = Some(b.to_owned());
    }
    v
}

#[cfg(test)]
mod tests {
    #[test]
    fn table_reads() {
        let l13 = super::original(13);
        assert_eq!(l13.len(), 5);
        assert_eq!(l13[0], None);
        assert_eq!(l13[2].as_deref(), Some("paper"));
        assert!(super::original(99).is_empty());
        for n in 1..=13 {
            assert!(
                super::original(n).iter().skip(1).all(Option::is_some),
                "level {n}"
            );
        }
    }
}
