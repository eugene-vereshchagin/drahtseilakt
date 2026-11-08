// What the extended mode adds (docs/drahtseilakt/menu.md): the mode itself, on or off, and each of its additions,
// which the Extended page turns off one by one. The settings keep the mode (`profile extended`) and the additions
// turned off (`extended.<key> off`); everything is on with the mode unless turned off. "All levels open" is a debug
// run's only (`--debug`), as god-mode is (mods.md): a player's game unlocks the levels by playing them. The practice
// tools and endless lives were additions here; they are god-mode's now, and their old `extended.tools` and
// `extended.endless-lives` lines are simply not read.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Feature {
    /// Every level open, without finishing the one before.
    AllLevels,
    /// Best times beside the levels.
    BestTimes,
    /// Custom levels in the main menu.
    CustomLevels,
}

// (the sound volume beside the music's was one; it is the original's own setting and always there now: an old
// `extended.sound-volume off` line is simply not read)
pub const FEATURES: [Feature; 3] = [Feature::AllLevels, Feature::BestTimes, Feature::CustomLevels];

impl Feature {
    pub fn key(self) -> &'static str {
        match self {
            Feature::AllLevels => "all-levels",
            Feature::BestTimes => "best-times",
            Feature::CustomLevels => "custom-levels",
        }
    }

    /// Only in a debug run (`--debug`): shown and in use there alone.
    pub fn debug(self) -> bool {
        matches!(self, Feature::AllLevels)
    }

    fn bit(self) -> u8 {
        1 << FEATURES.iter().position(|&f| f == self).unwrap_or(0)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Features {
    /// The extended mode.
    pub mode: bool,
    /// The additions turned off.
    off: u8,
    /// A debug run (`--debug`): the debug-only additions are there.
    pub debug: bool,
}

impl Features {
    #[cfg(test)]
    pub fn new(mode: bool) -> Features {
        Features { mode, off: 0, debug: false }
    }

    /// The mode as given and the additions the settings turn off (`setting` reads a key); `debug` a debug run.
    pub fn load(mode: bool, debug: bool, setting: impl Fn(&str) -> Option<String>) -> Features {
        let off = FEATURES
            .iter()
            .filter(|f| setting(&format!("extended.{}", f.key())).as_deref() == Some("off"))
            .fold(0, |m, f| m | f.bit());
        Features { mode, off, debug }
    }

    /// The addition is there to switch: a debug-only one in a debug run alone.
    pub fn offered(self, f: Feature) -> bool {
        !f.debug() || self.debug
    }

    /// The addition is in use: the mode is on, the addition offered and not turned off.
    pub fn has(self, f: Feature) -> bool {
        self.mode && self.offered(f) && self.off & f.bit() == 0
    }

    /// The addition's own switch, whatever the mode.
    pub fn switched_on(self, f: Feature) -> bool {
        self.off & f.bit() == 0
    }

    pub fn toggle(&mut self, f: Feature) {
        self.off ^= f.bit();
    }

    /// The settings lines of the additions turned off.
    pub fn lines(self) -> String {
        FEATURES
            .iter()
            .filter(|f| !self.switched_on(**f))
            .map(|f| format!("extended.{} off\n", f.key()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn additions_follow_the_mode_and_their_switches() {
        let mut f = Features::new(false);
        assert!(!f.has(Feature::BestTimes), "the mode is off");
        f.mode = true;
        f.debug = true;
        assert!(FEATURES.iter().all(|&x| f.has(x)), "all on with the mode");
        f.toggle(Feature::BestTimes);
        assert!(!f.has(Feature::BestTimes) && f.has(Feature::AllLevels));
        assert_eq!(f.lines(), "extended.best-times off\n");
        let back = Features::load(true, true, |k| (k == "extended.best-times").then(|| "off".to_owned()));
        assert_eq!(back, f);
        // an old practice line is not read
        assert_eq!(Features::load(true, true, |k| (k == "extended.tools").then(|| "off".to_owned())).lines(), "");
    }

    #[test]
    fn all_levels_open_is_a_debug_runs_only() {
        let player = Features::load(true, false, |_| None);
        assert!(!player.has(Feature::AllLevels) && !player.offered(Feature::AllLevels));
        assert!(player.has(Feature::BestTimes));
        assert!(Features::load(true, true, |_| None).has(Feature::AllLevels));
    }
}
