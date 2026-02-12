#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub struct Level(u8);

impl Level {
    pub const MIN: u8 = 1;
    pub const MAX: u8 = 100;

    pub fn new(value: u8) -> Option<Self> {
        (Self::MIN..=Self::MAX)
            .contains(&value)
            .then_some(Self(value))
    }

    pub fn get(self) -> u8 {
        self.0
    }

    pub fn is_max(self) -> bool {
        self.0 == Self::MAX
    }

    pub fn next(self) -> Option<Self> {
        if self.is_max() {
            None
        } else {
            Self::new(self.0 + 1)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrowthRate {
    Erratic,
    Fast,
    MediumFast,
    MediumSlow,
    Slow,
    Fluctuating,
}

impl GrowthRate {
    pub fn exp_for_level(self, level: Level) -> u32 {
        let l = level.get() as u32;
        match self {
            GrowthRate::Fast => (4 * l.pow(3)) / 5,
            GrowthRate::MediumFast => l.pow(3),
            GrowthRate::MediumSlow => {
                (((6 * l.pow(3)) / 5) as i32 - 15 * (l as i32).pow(2) + 100 * (l as i32) - 140)
                    as u32
                //              let l = level.0 as i32;
                //              ((6 * l.pow(3) - 15 * l.pow(2) + 100 * l - 140) / 5) as u32
            }
            GrowthRate::Slow => (5 * l.pow(3)) / 4,
            GrowthRate::Erratic => match l {
                1..=50 => l.pow(3) * (100 - l) / 50,
                51..=68 => l.pow(3) * (l + 14) / 50,
                69..=98 => l.pow(3) * ((1911 - 10 * l) / 3) / 500,
                99..=100 => l.pow(3) * (160 - l) / 100,
                _ => unreachable!(),
            },
            GrowthRate::Fluctuating => match l {
                1..=15 => l.pow(3) * ((l + 1) / 3 + 24) / 50,
                16..=36 => l.pow(3) * (l + 14) / 50,
                37..=100 => l.pow(3) * (l / 2 + 32) / 50,
                _ => unreachable!(),
            },
        }
    }

    pub fn exp_to_next_level(self, level: Level) -> Option<u32> {
        level
            .next()
            .map(|next| self.exp_for_level(next) - self.exp_for_level(level))
    }

    pub fn level_from_exp(self, exp: u32) -> Level {
        let mut low = Level::MIN;
        let mut high = Level::MAX;
        while low < high {
            let mid = (low + high).div_ceil(2);
            if exp >= self.exp_for_level(Level::new(mid).expect("Mid always valid")) {
                low = mid;
            } else {
                high = mid - 1;
            }
        }
        Level::new(low).expect("Level always valid")
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    fn level(n: u8) -> Level {
        Level::new(n).unwrap()
    }

    struct GrowthRateExpectation {
        rate: GrowthRate,
        level_2: u32,
        level_50: u32,
        level_100: u32,
    }

    const EXPECTATIONS: &[GrowthRateExpectation] = &[
        GrowthRateExpectation {
            rate: GrowthRate::Fast,
            level_2: 6,
            level_50: 100_000,
            level_100: 800_000,
        },
        GrowthRateExpectation {
            rate: GrowthRate::MediumFast,
            level_2: 8,
            level_50: 125_000,
            level_100: 1_000_000,
        },
        GrowthRateExpectation {
            rate: GrowthRate::MediumSlow,
            level_2: 9,
            level_50: 117_360,
            level_100: 1_059_860,
        },
        GrowthRateExpectation {
            rate: GrowthRate::Slow,
            level_2: 10,
            level_50: 156_250,
            level_100: 1_250_000,
        },
        GrowthRateExpectation {
            rate: GrowthRate::Erratic,
            level_2: 15,
            level_50: 125_000,
            level_100: 600_000,
        },
        GrowthRateExpectation {
            rate: GrowthRate::Fluctuating,
            level_2: 4,
            level_50: 142_500,
            level_100: 1_640_000,
        },
    ];

    #[test]
    fn growth_rates_match_expected_values() {
        for exp in EXPECTATIONS {
            assert_eq!(
                exp.rate.exp_for_level(level(2)),
                exp.level_2,
                "{:?} level 2 mismatch",
                exp.rate
            );
            assert_eq!(
                exp.rate.exp_for_level(level(50)),
                exp.level_50,
                "{:?} level 50 mismatch",
                exp.rate
            );
            assert_eq!(
                exp.rate.exp_for_level(level(100)),
                exp.level_100,
                "{:?} level 100 mismatch",
                exp.rate
            );
        }
    }

    #[test]
    fn level_from_exp_matches_known_values() {
        for exp in EXPECTATIONS {
            assert_eq!(exp.rate.level_from_exp(0), level(1));
            assert_eq!(exp.rate.level_from_exp(exp.level_2), level(2));
            assert_eq!(exp.rate.level_from_exp(exp.level_50), level(50));
            assert_eq!(exp.rate.level_from_exp(exp.level_100), level(100));
        }
    }

    #[test]
    fn level_from_exp_between_levels() {
        let rate = GrowthRate::Fast;

        let exp = rate.exp_for_level(level(10)) + 1;
        assert_eq!(rate.level_from_exp(exp), level(10));
    }

    #[test]
    fn exp_to_next_level_behavior() {
        let rate = GrowthRate::MediumFast;

        let exp = rate.exp_for_level(level(10));
        let needed = rate.exp_to_next_level(level(10));

        assert_eq!(needed, Some(rate.exp_for_level(level(11)) - exp));
    }

    #[test]
    fn cannot_level_past_max() {
        let rate = GrowthRate::Slow;
        assert_eq!(rate.exp_to_next_level(level(100)), None);
    }
}
