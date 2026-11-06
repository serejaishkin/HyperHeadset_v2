//! Decides when a "battery low" voice warning is due.
//!
//! The user picks the levels (usually tens of percent) at which to warn and one "critical" level at or
//! below which the warning is repeated. Each level fires once per discharge; it re-arms when the battery
//! climbs back above it or the headset is charging.

use crate::config::VoiceConfig;
use std::collections::BTreeSet;

#[derive(Debug, PartialEq, Eq)]
pub struct LowBatteryAlert {
    pub percent: u8,
    /// How many times the phrase is played (2 at the critical level).
    pub repeat: u8,
}

#[derive(Default)]
pub struct LowBatteryTracker {
    announced: BTreeSet<u8>,
}

impl LowBatteryTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed the latest reading. Returns an alert when a not-yet-announced level has been reached.
    pub fn update(
        &mut self,
        percent: u8,
        charging: bool,
        cfg: &VoiceConfig,
    ) -> Option<LowBatteryAlert> {
        if charging || percent == 0 {
            // 0 = unknown reading; charging = no discharge warnings, everything re-arms.
            if charging {
                self.announced.clear();
            }
            return None;
        }
        // Re-arm levels the battery has risen above.
        self.announced.retain(|l| percent <= *l);
        let crossed: Vec<u8> = cfg
            .low_levels
            .iter()
            .copied()
            .filter(|l| *l > 0 && percent <= *l && !self.announced.contains(l))
            .collect();
        if crossed.is_empty() {
            return None;
        }
        // Several levels can be crossed at once (e.g. the app starts at 8 %): announce once.
        self.announced.extend(crossed);
        let critical = cfg.low_critical_percent > 0 && percent <= cfg.low_critical_percent;
        Some(LowBatteryAlert {
            percent,
            repeat: if critical { 2 } else { 1 },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(levels: &[u8], critical: u8) -> VoiceConfig {
        VoiceConfig {
            low_levels: levels.to_vec(),
            low_critical_percent: critical,
            ..Default::default()
        }
    }

    #[test]
    fn warns_once_per_level_and_repeats_at_critical() {
        let c = cfg(&[30, 20, 10], 10);
        let mut t = LowBatteryTracker::new();
        assert_eq!(t.update(45, false, &c), None);
        assert_eq!(
            t.update(30, false, &c),
            Some(LowBatteryAlert {
                percent: 30,
                repeat: 1
            })
        );
        assert_eq!(t.update(29, false, &c), None);
        assert_eq!(
            t.update(20, false, &c),
            Some(LowBatteryAlert {
                percent: 20,
                repeat: 1
            })
        );
        assert_eq!(
            t.update(10, false, &c),
            Some(LowBatteryAlert {
                percent: 10,
                repeat: 2
            })
        );
        assert_eq!(t.update(9, false, &c), None);
    }

    #[test]
    fn rearms_after_charging_or_recovery() {
        let c = cfg(&[20], 0);
        let mut t = LowBatteryTracker::new();
        assert!(t.update(19, false, &c).is_some());
        assert_eq!(t.update(18, false, &c), None);
        assert_eq!(t.update(40, true, &c), None); // charging
        assert!(t.update(19, false, &c).is_some()); // armed again
        assert_eq!(t.update(35, false, &c), None);
        assert!(t.update(20, false, &c).is_some()); // recovered above, dropped again
    }

    #[test]
    fn start_below_several_levels_announces_once() {
        let c = cfg(&[30, 20, 10], 10);
        let mut t = LowBatteryTracker::new();
        assert_eq!(
            t.update(8, false, &c),
            Some(LowBatteryAlert {
                percent: 8,
                repeat: 2
            })
        );
        assert_eq!(t.update(7, false, &c), None);
    }

    #[test]
    fn unknown_reading_and_empty_levels_do_nothing() {
        let mut t = LowBatteryTracker::new();
        assert_eq!(t.update(0, false, &cfg(&[20], 10)), None);
        assert_eq!(t.update(5, false, &cfg(&[], 10)), None);
    }
}
