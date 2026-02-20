use anyhow::{Context, Result};
use lazy_static::lazy_static;
use regex::Regex;
use uuid::Uuid;

lazy_static! {
    static ref UUID_REGEX: Regex =
        Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$")
            .unwrap();
}

pub fn gen_uuid() -> String {
    Uuid::new_v4().to_string()
}

pub fn parse_uuid(s: &str) -> Result<Uuid> {
    Uuid::parse_str(s).context("Failed to parse UUID string")
}

pub fn is_valid_uuid(s: &str) -> bool {
    // Check regex for V4 format strictness
    // Standard Uuid::parse_str might allow other versions/variants, but regex enforces lowercase v4.
    if !UUID_REGEX.is_match(s) {
        return false;
    }
    // Double check with parser just in case
    Uuid::parse_str(s).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gen_uuid() {
        let id = gen_uuid();
        assert!(is_valid_uuid(&id));
    }

    #[test]
    fn test_parse_uuid() {
        let _valid = "550e8400-e29b-41d4-a716-446655440000";
        // Note: regex enforces '4' at 13th char (version 4) and '89ab' at variant.
        // The above UUID is v4 (41d4).
        // Let's generate a real v4 to be sure of test data validity with our strict regex.
        let v4 = Uuid::new_v4().to_string();

        let parsed = parse_uuid(&v4).unwrap();
        assert_eq!(parsed.to_string(), v4);
    }

    #[test]
    fn test_is_valid_uuid() {
        let valid = Uuid::new_v4().to_string();
        assert!(is_valid_uuid(&valid));

        let invalid_ver = "00000000-0000-1000-8000-000000000000"; // v1
        assert!(
            !is_valid_uuid(invalid_ver),
            "Should reject non-v4 if the regex enforces v4 logic 4...-... "
        );
        // Our regex: -4[0-9a-f]{3}-

        let uppercase = valid.to_uppercase();
        // Regex expects lowercase [0-9a-f]
        assert!(
            !is_valid_uuid(&uppercase),
            "Should reject uppercase per regex"
        );

        let garbage = "not-a-uuid";
        assert!(!is_valid_uuid(garbage));
    }
}
