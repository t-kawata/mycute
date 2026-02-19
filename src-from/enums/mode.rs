use crate::config;
use std::str::FromStr;

pub enum Mode {
    RT,
    AM,
}

impl Mode {
    pub fn as_str(&self) -> &str {
        match self {
            Mode::RT => "rt",
            Mode::AM => "am",
        }
    }
    fn as_help(&self) -> &str {
        match self {
            Mode::RT => "Run as REST API server.",
            Mode::AM => "Run auto migration for db.",
        }
    }
    fn all() -> &'static [Mode] {
        &[Mode::RT, Mode::AM]
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "rt" => Some(Mode::RT),
            "am" => Some(Mode::AM),
            _ => None,
        }
    }
    pub fn help() -> String {
        let mut lines = Vec::new();
        lines.push(format!("\n[Available Modes] {}\n", config::VERSION));
        for mode in Self::all() {
            lines.push(format!("> {}\n\t{}", mode.as_str(), mode.as_help()));
        }
        lines.join("\n") + "\n"
    }
    pub fn is_valid(s: &str) -> bool {
        Self::from_str(s).is_some()
    }
}

#[derive(Debug)]
pub struct ModeParseError;

impl FromStr for Mode {
    type Err = ModeParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_str(s).ok_or(ModeParseError)
    }
}
