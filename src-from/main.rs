use mycute::config;
use mycute::enums::Mode;
use mycute::mode::am;
use mycute::mode::rt;
use std::env;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Missing 1st arg as mode to run.");
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
        std::process::exit(1);
    }

    let mode = Mode::from_str(&m).expect("Invalid mode");

    let mode_args = std::iter::once(args[0].clone()).chain(args[2..].iter().cloned());

    match mode {
        Mode::RT => {
            rt::main_of_rt(mode_args).await;
        }
        Mode::AM => {
            am::main_of_am(mode_args).await;
        }
    }
}
