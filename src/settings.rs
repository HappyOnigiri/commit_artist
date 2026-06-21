use crate::command;
use num_cpus;
use regex::Regex;

#[derive(Clone)]
pub struct Settings {
    pub path: String,
    pub patterns: Vec<String>,
    pub block_size: usize,
    pub jobs: usize,
}

impl Settings {
    ///
    /// Construct.
    ///
    /// # Panics
    /// patterns is a comma-separated list of hex strings (each 1..=40 chars)
    /// jobs should be more than 0
    /// block size should be less than 64.
    ///
    pub fn new<P: Into<String>, Q: Into<String>>(path: P, patterns: Q, block_size: usize, jobs: usize) -> Self {
        let regx = Regex::new(r"^[0-9a-f]{1,40}$").unwrap();
        let patterns: Vec<String> = patterns
            .into()
            .split(',')
            .map(|p| p.trim().to_owned())
            .collect();
        assert!(!patterns.is_empty());
        for p in &patterns {
            assert!(regx.is_match(p));
        }
        assert!(jobs > 0);
        assert!(block_size < 64);
        Self {
            path: path.into(),
            patterns,
            block_size,
            jobs,
        }
    }

    pub fn patterns<T: Into<String>>(&mut self, patterns: T) {
        let regx = Regex::new(r"^[0-9a-f]{1,40}$").unwrap();
        let parsed: Vec<String> = patterns
            .into()
            .split(',')
            .map(|p| p.trim().to_owned())
            .collect();
        assert!(!parsed.is_empty());
        for p in &parsed {
            assert!(regx.is_match(p));
        }
        self.patterns = parsed;
    }

    pub fn jobs(&mut self, jobs: usize) {
        assert!(jobs > 0);
        self.jobs = jobs;
    }

    pub fn block_size(&mut self, block_size: usize) {
        assert!(block_size < 64);
        self.block_size = block_size;
    }
}

impl Default for Settings {
    fn default() -> Self {
        let path: String = command::current_dir_path();
        let num = num_cpus::get();
        Self::new(path, "0000000", 20, num - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::Settings;

    #[test]
    fn settings_constructor() {
        Settings::new("./", "0000000", 20, 10);
    }

    #[test]
    fn settings_constructor_multi_pattern() {
        let s = Settings::new("./", "1111111,2222222", 20, 10);
        assert_eq!(s.patterns, vec!["1111111", "2222222"]);
    }

    #[test]
    fn settings_constructor_multi_pattern_with_spaces() {
        let s = Settings::new("./", "1111111, 2222222", 20, 10);
        assert_eq!(s.patterns, vec!["1111111", "2222222"]);
    }

    #[test]
    #[should_panic]
    fn nonnominal_settings1() {
        Settings::new("./", "invalidpattern", 20, 10);
    }

    #[test]
    #[should_panic]
    fn nonnominal_settings1_multi() {
        Settings::new("./", "1111111,invalidpattern", 20, 10);
    }

    #[test]
    #[should_panic]
    fn nonnominal_settings2() {
        Settings::new("./", "0000000", 1000, 10);
    }

    #[test]
    #[should_panic]
    fn nonnominal_settings3() {
        Settings::new("./", "0000000", 1000, 0);
    }
}
