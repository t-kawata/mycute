use crate::config;
use std::str::FromStr;

pub const MODE_CL: &str = "cl";
pub const MODE_AM: &str = "am";
pub const MODE_OG: &str = "og";

pub enum Mode {
    AM,
    CL,
    OG,
}

impl Mode {
    pub fn as_str(&self) -> &str {
        match self {
            Mode::AM => MODE_AM,
            Mode::CL => MODE_CL,
            Mode::OG => MODE_OG,
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
            MODE_AM => Some(Mode::AM),
            MODE_CL => Some(Mode::CL),
            MODE_OG => Some(Mode::OG),
            _ => Some(Mode::CL),
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
