#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    pub power_saver: bool,
    pub haptics: bool,
    pub best: u32,
    pub show_fps: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            power_saver: false,
            haptics: true,
            best: 0,
            show_fps: false,
        }
    }
}

impl Settings {
    pub fn encode(self) -> String {
        format!(
            "3 {} {} {} {}",
            u8::from(self.power_saver),
            u8::from(self.haptics),
            self.best,
            u8::from(self.show_fps)
        )
    }

    pub fn decode(value: &str) -> Self {
        let fields: Vec<_> = value.split_whitespace().collect();
        if !matches!(fields.len(), 4 | 5)
            || !matches!(fields[0], "1" | "2" | "3")
            || (fields[0] == "3") != (fields.len() == 5)
            || (fields.len() == 5 && !matches!(fields[4], "0" | "1"))
            || !matches!(fields[1], "0" | "1")
            || !matches!(fields[2], "0" | "1")
        {
            return Self::default();
        }
        let Ok(best @ 0..=397) = fields[3].parse::<u32>() else {
            return Self::default();
        };
        // The original release enabled saver by default. Apply the redesigned
        // defaults once to v1 saves while retaining the player's best score.
        if fields[0] == "1" {
            return Self {
                best,
                ..Self::default()
            };
        }
        Self {
            power_saver: fields[1] == "1",
            haptics: fields[2] == "1",
            best,
            show_fps: fields.get(4) == Some(&"1"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn settings_round_trip() {
        let settings = Settings {
            power_saver: true,
            haptics: false,
            best: 42,
            show_fps: true,
        };
        assert_eq!(Settings::decode(&settings.encode()), settings);
    }
    #[test]
    fn damaged_or_future_storage_uses_safe_defaults() {
        for value in [
            "",
            "4 1 0 4",
            "3 1 0 4",
            "3 1 0 4 x",
            "2 1 0 4 1",
            "1 1 1",
            "1 x 1 3",
            "1 1 0 -1",
            "1 1 0 99999999",
        ] {
            assert_eq!(Settings::decode(value), Settings::default());
        }
    }

    #[test]
    fn fresh_install_is_white_smooth_and_haptic_enabled() {
        let settings = Settings::default();
        assert!(!settings.power_saver);
        assert!(settings.haptics);
        assert!(!settings.show_fps);
    }

    #[test]
    fn v2_preferences_migrate_with_counter_off() {
        let settings = Settings::decode("2 1 0 42");
        assert!(settings.power_saver);
        assert!(!settings.haptics);
        assert_eq!(settings.best, 42);
        assert!(!settings.show_fps);
    }

    #[test]
    fn old_defaults_migrate_without_losing_best_score() {
        assert_eq!(
            Settings::decode("1 1 1 27"),
            Settings {
                best: 27,
                ..Settings::default()
            }
        );
        assert_eq!(Settings::decode("1 0 0 12").best, 12);
    }
}
