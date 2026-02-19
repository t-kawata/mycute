use std::env;

pub fn get_env_or<T: std::str::FromStr>(key: &str, default: T) -> T {
    match env::var(key) {
        Ok(value) => value.parse::<T>().unwrap_or(default),
        Err(_) => default,
    }
}
