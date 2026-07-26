#[rustfmt::skip]
fn main() { match wmw::run_cli_env() { Ok(code) => std::process::exit(code), Err(error) => { eprintln!("{error:#}"); std::process::exit(2); } } }
