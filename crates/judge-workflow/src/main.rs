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
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    if args.as_slice() == [std::ffi::OsString::from("evaluate-workflow-v1")] {
        return match chrono_judge_workflow::evaluate_original(
            std::io::stdin().lock(),
            std::io::stdout().lock(),
        ) {
            Ok(()) => 0.into(),
            Err(error) => {
                eprintln!("{error}");
                2.into()
            }
        };
    }
    use std::os::unix::process::CommandExt;
    let program = match std::env::current_exe() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("{error}");
            return 2.into();
        }
    };
    let error = std::process::Command::new("chrono-harness")
        .args([
            "judge-command-v1",
            "chrono-judge-workflow",
            env!("CARGO_PKG_VERSION"),
        ])
        .arg(program)
        .args([
            "evaluate-workflow-v1",
            "E_WORKFLOW_INPUT",
            "--admission-result",
        ])
        .args(args)
        .exec();
    eprintln!("chrono-harness judge-command-v1: {error}");
    2.into()
}
