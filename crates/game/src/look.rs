// The Look: one choice of how levels are drawn (owner's decision 2026-11-11, decisions.md "One Look, and beauty comes
// through the toolbox"), in place of separate texture and shape switches.
//
// - Original: the player's copy's textures and the 2004 shapes.
// - Remaster: the toolbox's enlarged original textures on rounded shapes.
// - Drahtseilakt: our textures on rounded shapes.
// - Player's pack: a texture pack of the player's own, on rounded shapes.
//
// The rounded shapes of an original level are the toolbox's export of it (levels/remastered/Level_NN, the rounding
// cache); a level that has none plays as it is, with the look's textures. A look is offered only where it can be
// shown: the copy's needs the copy, the remaster's the copy and an export, the player's a pack of his.

use gamedata::packs::{self, Kind, ORIGINAL, OWN, REMASTERED};
use std::sync::Mutex;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Look {
    Original,
    Remaster,
    Drahtseilakt,
    Player,
}

/// In the menu's order.
pub const ALL: [Look; 4] = [
    Look::Original,
    Look::Remaster,
    Look::Drahtseilakt,
    Look::Player,
];

static CHOSEN: Mutex<Look> = Mutex::new(Look::Drahtseilakt);

impl Look {
    /// The settings value and the menu words' key (`look <key>`).
    pub fn key(self) -> &'static str {
        match self {
            Look::Original => "original",
            Look::Remaster => "remaster",
            Look::Drahtseilakt => "drahtseilakt",
            Look::Player => "player",
        }
    }

    /// Whether the look has the rounded shapes (every look but the original's).
    pub fn rounded(self) -> bool {
        self != Look::Original
    }

    /// Whether the look can be shown here.
    pub fn available(self) -> bool {
        match self {
            Look::Original => packs::packs(Kind::Textures)
                .iter()
                .any(|p| p.key == ORIGINAL),
            Look::Remaster => packs::packs(Kind::Textures)
                .iter()
                .any(|p| p.key == REMASTERED),
            Look::Drahtseilakt => true,
            Look::Player => player_pack(None).is_some(),
        }
    }

    /// The texture pack the look draws with; `kept` is the player's pack of the settings.
    fn textures(self, kept: Option<&str>) -> Option<String> {
        match self {
            Look::Original => Some(ORIGINAL.to_owned()),
            Look::Remaster => Some(REMASTERED.to_owned()),
            Look::Drahtseilakt => Some(OWN.to_owned()),
            Look::Player => player_pack(kept),
        }
    }
}

/// A texture pack of the player's: `kept` when it is one, else the first found.
fn player_pack(kept: Option<&str>) -> Option<String> {
    let found: Vec<String> = packs::packs(Kind::Textures)
        .into_iter()
        .filter(|p| !p.is_builtin())
        .map(|p| p.key)
        .collect();
    kept.and_then(|k| found.iter().find(|f| *f == k))
        .or(found.first())
        .cloned()
}

/// The look the settings ask for: `look`, and for settings written before the Look existed (`look original |
/// remastered` for the original levels, `textures` for the pictures) what they meant. Not available: ours.
pub fn from_settings(look: Option<&str>, textures: Option<&str>) -> Look {
    let by_textures = || match textures {
        Some(ORIGINAL) => Look::Original,
        Some(REMASTERED) => Look::Remaster,
        Some(OWN) | None => Look::Drahtseilakt,
        Some(_) => Look::Player,
    };
    let wanted = match look {
        Some("remaster" | "remastered") => Look::Remaster,
        Some("drahtseilakt") => Look::Drahtseilakt,
        Some("player") => Look::Player,
        // an old `look original` meant "as they are", with whatever textures were chosen: the textures decide
        Some("original") if textures == Some(ORIGINAL) => Look::Original,
        _ => by_textures(),
    };
    if wanted.available() {
        wanted
    } else {
        Look::Drahtseilakt
    }
}

/// The look in force.
pub fn chosen() -> Look {
    CHOSEN.lock().map_or(Look::Drahtseilakt, |l| *l)
}

/// Starts from the settings (after the packs are found): the look, and the texture pack it draws with.
pub fn start(look: Option<&str>, textures: Option<&str>) {
    let l = from_settings(look, textures);
    choose(l, textures);
}

/// Chooses a look; its textures apply from the next level loaded.
pub fn choose(look: Look, kept: Option<&str>) {
    if let Ok(mut c) = CHOSEN.lock() {
        *c = look;
    }
    if let Some(k) = look.textures(kept) {
        packs::choose(Kind::Textures, &k);
    }
}

/// The look `by` steps after the chosen one among the available (the arrows go round them).
pub fn next(by: i32) -> Look {
    let ok: Vec<Look> = ALL.into_iter().filter(|l| l.available()).collect();
    let now = chosen();
    let at = i64::try_from(ok.iter().position(|&l| l == now).unwrap_or(0)).unwrap_or(0);
    let n = i64::try_from(ok.len().max(1)).unwrap_or(1);
    usize::try_from(at.saturating_add(i64::from(by)).rem_euclid(n))
        .ok()
        .and_then(|i| ok.get(i).copied())
        .unwrap_or(Look::Drahtseilakt)
}

/// The settings lines a look is kept as: the look, and the textures pack it draws with (so that an older game
/// reading the file gets the same pictures).
pub fn settings(look: Look) -> Vec<(String, String)> {
    let mut v = vec![("look".to_owned(), look.key().to_owned())];
    if let Some(t) = look.textures(None) {
        v.push(("textures".to_owned(), t));
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_settings_keep_their_meaning() {
        // nothing set: ours; the old textures keys say what they said; an old `look remastered` is the Remaster
        assert_eq!(from_settings(None, None), Look::Drahtseilakt);
        assert_eq!(
            from_settings(Some("original"), Some("own")),
            Look::Drahtseilakt
        );
        assert_eq!(
            from_settings(Some("drahtseilakt"), None),
            Look::Drahtseilakt
        );
        // looks that cannot be shown here fall back to ours (the test has no remaster export, no player's pack)
        assert_eq!(from_settings(Some("player"), None), Look::Drahtseilakt);
        assert!(Look::Drahtseilakt.rounded() && !Look::Original.rounded());
    }

    #[test]
    fn keys_are_distinct() {
        let keys: Vec<_> = ALL.iter().map(|l| l.key()).collect();
        assert_eq!(keys, ["original", "remaster", "drahtseilakt", "player"]);
    }
}
