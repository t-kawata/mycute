//! mycute - Real-time Japanese STT Input Tool
//!
//! - cl: Client/Server (Tauri/Axum)
//! - am: Auto Migration

use mycute::config;
use mycute::enums::Mode;
use mycute::mode::am;
use mycute::mode::cl;
use mycute::mode::og;
use mycute::utils::init::AppInit;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Missing 1st arg as mode to run.");
        eprintln!("{}", Mode::help());
        std::process::exit(1);
    }

    let m = args[1].clone();

    if m.as_str() == "-h" || m.as_str() == "--help" {
        println!("{}", Mode::help());
        std::process::exit(0);
    }

    if m.as_str() == "-v" || m.as_str() == "--version" {
        println!("{}", config::VERSION);
        std::process::exit(0);
    }

    if !Mode::is_valid(&m) {
        eprintln!("Invalid mode: {}", m);
        eprintln!("{}", Mode::help());
        std::process::exit(1);
    }

    let mode = Mode::from_str(&m).expect("Invalid mode");

    let mode_args = std::iter::once(args[0].clone()).chain(args[2..].iter().cloned());

    let result = match mode {
        Mode::AM => {
            let (flgs, _hc) = AppInit::<am::AMFlgs>::from_args(mode_args)
                .with_logger()
                .init()
                .expect("Failed to init am mode.");
            am::main_of_am(flgs)
        }
        Mode::CL => {
            let (flgs, hc) = AppInit::<cl::CLFlgs>::from_args(mode_args)
                .with_logger()
                .init()
                .expect("Failed to init cl mode.");
            cl::main_of_cl(flgs, hc)
        }
        Mode::OG => {
            let (flgs, _hc) = AppInit::<og::OGFlgs>::from_args(mode_args)
                .with_logger()
                .init()
                .expect("Failed to init og mode.");
            og::main_of_og(flgs)
        }
    };

    if let Err(e) = result {
        log::error!("Execution failed: {:?}", e);
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
