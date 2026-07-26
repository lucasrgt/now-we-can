#[rustfmt::skip]
fn main() { match notyet::run_cli_env() { Ok(code) => std::process::exit(code), Err(error) => { eprintln!("{error:#}"); std::process::exit(2); } } }
