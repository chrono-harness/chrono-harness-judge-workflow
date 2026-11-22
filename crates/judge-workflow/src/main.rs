fn main() -> std::process::ExitCode {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() == Some(std::ffi::OsStr::new("check-observation-age")) {
        if args.next().is_some() {
            eprintln!("E_USAGE: check-observation-age reads JSON from stdin");
            return 2.into();
        }
        return match chrono_judge_workflow::check_observation_age(std::io::stdin().lock()) {
            Ok(()) => 0.into(),
            Err((code, message)) => {
                eprintln!("{message}");
                code.into()
            }
        };
    }
    std::process::ExitCode::from(chrono_harness::wire::judge_main(
        "chrono-judge-workflow",
        env!("CARGO_PKG_VERSION"),
        chrono_judge_workflow::judge,
    ))
}
