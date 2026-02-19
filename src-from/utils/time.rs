use chrono::NaiveDateTime;

// For JST (NaiveDateTime), we output as is.
pub fn datetime_to_str(dt: &NaiveDateTime) -> String {
    dt.format("%Y-%m-%dT%H:%M:%S").to_string()
}

pub fn naive_to_str(dt: &NaiveDateTime) -> String {
    dt.format("%Y-%m-%dT%H:%M:%S").to_string()
}
