// What the extended mode adds (docs/drahtseilakt/menu.md): the mode itself, on or off, and each of its additions,
// which the Extended page turns off one by one. The settings keep the mode (`profile extended`) and the additions
// turned off (`extended.<key> off`); everything is on with the mode unless turned off.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Feature {
    /// Every level open, without finishing the one before.
    AllLevels,
    /// Best times beside the levels.
    BestTimes,
    /// Custom levels in the main menu.
    CustomLevels,
    /// Practice tools: R, F2, [ ], F5 / F9, the sector and tick on screen.
    Tools,
    /// The sound volume beside the music's.
    SoundVolume,
}

pub const FEATURES: [Feature; 5] = [
    Feature::AllLevels,
    Feature::BestTimes,
    Feature::CustomLevels,
    Feature::Tools,
    Feature::SoundVolume,
];

impl Feature {
    pub fn key(self) -> &'static str {
        match self {
            Feature::AllLevels => "all-levels",
            Feature::BestTimes => "best-times",
            Feature::CustomLevels => "custom-levels",
            Feature::Tools => "tools",
            Feature::SoundVolume => "sound-volume",
        }
    }

    /// The switch's label on the Extended page.
    pub fn label(self) -> &'static str {
        match self {
            Feature::AllLevels => "All levels open?",
            Feature::BestTimes => "Best times?",
            Feature::CustomLevels => "Custom levels?",
            Feature::Tools => "Practice tools?",
            Feature::SoundVolume => "Sound volume?",
        }
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
}

impl Features {
    #[cfg(test)]
    pub fn new(mode: bool) -> Features {
        Features { mode, off: 0 }
    }

    /// The mode as given and the additions the settings turn off (`setting` reads a key).
    pub fn load(mode: bool, setting: impl Fn(&str) -> Option<String>) -> Features {
        let off = FEATURES
            .iter()
            .filter(|f| setting(&format!("extended.{}", f.key())).as_deref() == Some("off"))
            .fold(0, |m, f| m | f.bit());
        Features { mode, off }
    }

    /// The addition is in use: the mode is on and the addition not turned off.
    pub fn has(self, f: Feature) -> bool {
        self.mode && self.off & f.bit() == 0
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
        assert!(!f.has(Feature::Tools), "the mode is off");
        f.mode = true;
        assert!(FEATURES.iter().all(|&x| f.has(x)), "all on with the mode");
        f.toggle(Feature::Tools);
        assert!(!f.has(Feature::Tools) && f.has(Feature::AllLevels));
        assert_eq!(f.lines(), "extended.tools off\n");
        let back = Features::load(true, |k| (k == "extended.tools").then(|| "off".to_owned()));
        assert_eq!(back, f);
    }
}
