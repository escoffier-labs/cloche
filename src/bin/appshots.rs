use std::process::ExitCode;

fn main() -> ExitCode {
    match cloche::run() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("appshots: {err}");
            ExitCode::from(1)
        }
    }
}
