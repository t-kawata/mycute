use mycute::stt_config::Settings;
use std::fs;

#[test]
fn test_verify_settings() {
    let content = fs::read_to_string("settings.json").expect("Failed to read settings.json");
    match serde_json::from_str::<Settings>(&content) {
        Ok(settings) => {
            println!("SUCCESS: Settings loaded correctly.");
            println!("STT Engine: {:?}", settings.stt_engine);
            println!("Replaces count: {}", settings.replaces.len());
            for (i, (k, v)) in settings.replaces.iter().enumerate() {
                println!("  {}: {} -> {:?}", i, k, v);
            }
        },
        Err(e) => {
            panic!("ERROR: Failed to parse settings: {}", e);
        }
    }
}
