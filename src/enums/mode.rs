use crate::config;
use std::str::FromStr;

pub enum Mode {
    AM,
    CL,
    OG,
}

impl Mode {
    pub fn as_str(&self) -> &str {
        match self {
            Mode::AM => "am",
            Mode::CL => "cl",
            Mode::OG => "og",
        }
    }
    fn as_help(&self) -> &str {
        match self {
            Mode::AM => "Run auto migration for database.",
            Mode::CL => "Run as mycute client/server. Use -r to set role.",
            Mode::OG => "Run owners gen mode.",
        }
    }
    fn all() -> &'static [Mode] {
        &[Mode::AM, Mode::CL, Mode::OG]
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "am" => Some(Mode::AM),
            "cl" => Some(Mode::CL),
            "og" => Some(Mode::OG),
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
