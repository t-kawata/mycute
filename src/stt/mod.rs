pub mod recognizer;
pub mod stats;
pub mod openai;

#[cfg(target_os = "windows")]
pub mod win;

#[cfg(target_os = "macos")]
pub mod mac;

pub use recognizer::SpeechRecognizer;
