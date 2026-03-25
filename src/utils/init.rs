use crate::utils::time;
use clap::{Args, ValueEnum};
use fern::Dispatch;
use log::LevelFilter;
use serde::Serialize;
use std::iter::{Chain, Cloned, Once};
use std::slice::Iter;
use std::sync::Arc;

#[derive(Clone)]
pub struct SharedHttpClients {
    pub async_hc: Arc<reqwest::Client>,
    pub blocking_hc: Arc<reqwest::blocking::Client>,
}
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
    #[arg(
        short = 'l',
        long = "log_level",
        default_value = "debug",
        help = "Log level"
    )]
    pub log_level: LogLevel,
    #[arg(short = 'o', long = "output", default_value_t = String::from("stdout"), help = "Destination of log output (stdout, /path/to/file).")]
    pub output: String,
}

pub trait HasCommonFlgs {
    fn common_flgs(&self) -> &CommonFlgs;
}

pub struct AppInit<'a, T>
where
    T: clap::Parser + HasCommonFlgs,
{
    args: Chain<Once<String>, Cloned<Iter<'a, String>>>,
    use_logger: bool,
    _phantom: std::marker::PhantomData<T>,
}

impl<'a, T> AppInit<'a, T>
where
    T: clap::Parser + HasCommonFlgs,
{
    pub fn from_args(args: Chain<Once<String>, Cloned<Iter<'a, String>>>) -> Self {
        Self {
            args,
            use_logger: false,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn with_logger(mut self) -> Self {
        self.use_logger = true;
        self
    }

    pub fn init(self) -> Result<(T, SharedHttpClients), Box<dyn std::error::Error>> {
        let flgs = parse_args::<T>(self.args)?;
        if self.use_logger {
            setup_logging(flgs.common_flgs())?;
        }

        let timeout = std::time::Duration::from_secs(crate::constants::HTTP_TIMEOUT_SEC);

        let async_hc = Arc::new(reqwest::Client::builder().timeout(timeout).build()?);

        let blocking_hc = Arc::new(
            reqwest::blocking::Client::builder()
                .timeout(timeout)
                .build()?,
        );

        Ok((
            flgs,
            SharedHttpClients {
                async_hc,
                blocking_hc,
            },
        ))
    }
}

pub fn init<T>(
    args: Chain<Once<String>, Cloned<Iter<'_, String>>>,
) -> Result<(T, SharedHttpClients), Box<dyn std::error::Error>>
where
    T: clap::Parser + HasCommonFlgs,
{
    AppInit::<T>::from_args(args).with_logger().init()
}

pub fn parse_args<T>(
    args: Chain<Once<String>, Cloned<Iter<'_, String>>>,
) -> Result<T, Box<dyn std::error::Error>>
where
    T: clap::Parser + HasCommonFlgs,
{
    let flgs = T::parse_from(args);
    Ok(flgs)
}

pub fn setup_logging(common: &CommonFlgs) -> Result<(), Box<dyn std::error::Error>> {
    let level = match common.log_level {
        LogLevel::Error => LevelFilter::Error,
        LogLevel::Warn => LevelFilter::Warn,
        LogLevel::Info => LevelFilter::Info,
        LogLevel::Debug => LevelFilter::Debug,
        LogLevel::Trace => LevelFilter::Trace,
    };

    // YYYY-MM-DDThh:mm:ss 形式の文字列（UTC想定）を
    const TARGET_WIDTH: usize = 25;

    let mut log_dispatch_base = Dispatch::new()
        .level(level)
        .level_for("wgpu_core", LevelFilter::Warn)
        .level_for("wgpu_hal", LevelFilter::Warn)
        .level_for("naga", LevelFilter::Warn)
        .format(|out, message, record| {
            let now = time::now();
            let target = record.target().replace("::", ".");
            let display_target = if target.len() > TARGET_WIDTH {
                &target[target.len() - TARGET_WIDTH..]
            } else {
                &target
            };

            let mut message_content = message.to_string();
            if target.starts_with("sea_orm.driver.") || target.starts_with("sea_orm.database.") {
                message_content = format!("\x1b[36m{}\x1b[0m", message_content);
            }

            out.finish(format_args!(
                "{} {: <width$} [{}] {}",
                now.format("%y-%m-%d_%H:%M:%S"),
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
            log_dispatch_base = log_dispatch_base
                .chain(fern::log_file(path)?)
                .chain(std::io::stdout()); // 親プロセスのリダイレクト(spawn)用にstdoutにも流す
        }
    }

    log_dispatch_base.apply()?;
    Ok(())
}
