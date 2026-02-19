use crate::config::settings::{Env, get_env};
use chrono::{FixedOffset, Utc};
use clap::{Args, Parser, ValueEnum};
use fern::Dispatch;
use log::LevelFilter;
use serde::Serialize;
use std::iter::{Chain, Cloned, Once};
use std::slice::Iter;

#[derive(Debug, ValueEnum, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

#[derive(Debug, Args, Serialize, Clone)]
pub struct CommonFlgs {
    #[arg(short = 'e', long = "env", default_value_t = String::from("local"), help = "Environment")]
    pub env: String,
    #[arg(short = 'l', long = "log_level", default_value = "debug", help = "Log level")]
    pub log_level: LogLevel,
    #[arg(short = 'o', long = "output", default_value_t = String::from("stdout"), help = "Destination of log output (stdout, /path/to/file).")]
    pub output: String,
}

pub trait HasCommonFlgs {
    fn common_flgs(&self) -> &CommonFlgs;
}

pub fn init<T>(
    args: Chain<Once<String>, Cloned<Iter<'_, String>>>,
) -> Result<(T, Env), Box<dyn std::error::Error>>
where
    T: Parser + HasCommonFlgs,
{
    let flgs = T::parse_from(args);
    let common = flgs.common_flgs();

    let env = get_env(&common.env);
    if env.empty {
        return Err(format!("Invalid environment: {}", common.env).into());
    }

    let level = match common.log_level {
        LogLevel::Error => LevelFilter::Error,
        LogLevel::Warn => LevelFilter::Warn,
        LogLevel::Info => LevelFilter::Info,
        LogLevel::Debug => LevelFilter::Debug,
        LogLevel::Trace => LevelFilter::Trace,
    };

    const TARGET_WIDTH: usize = 25;

    let mut log_dispatch_base = Dispatch::new().level(level).format(|out, message, record| {
        let jst = Utc::now().with_timezone(&FixedOffset::east_opt(9 * 3600).unwrap());
        let target = record.target().replace("::", ".");
        let display_target = if target.len() > TARGET_WIDTH {
            &target[target.len() - TARGET_WIDTH..]
        } else {
            &target
        };

        let mut message_content = message.to_string();
        if target.starts_with("sea_orm.driver.") || target.starts_with("sea_orm.database.") {
            // Cyan (36)
            message_content = format!("\x1b[36m{}\x1b[0m", message_content);
        }

        out.finish(format_args!(
            "{} {: <width$} [{}] {}", // width$ で引数の値を参照
            jst.format("%y-%m-%d_%H:%M:%S"),
            display_target,
            record.level(),
            message_content,
            width = TARGET_WIDTH
        ))
    });

    match common.output.as_str() {
        "stdout" => {
            log_dispatch_base = log_dispatch_base.chain(std::io::stdout());
        }
        path => {
            log_dispatch_base = log_dispatch_base.chain(fern::log_file(path)?);
        }
    }

    log_dispatch_base.apply()?;
    Ok((flgs, env))
}
