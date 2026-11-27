#![allow(unexpected_cfgs)]
use vstd::contrib::exec_spec::ExecSpecEq;
use vstd::prelude::*;

verus! {
pub enum Command { Version, Age, Usage, Evaluate, Delegate }

/// Unrecognized arguments, including platform-native non-UTF-8 values, go to the runner.
pub fn select_command(first: Option<&str>, count: usize) -> (command: Command)
    ensures command == match first {
        Some(s) if s@ == "--version"@ && count == 1 => Command::Version,
        Some(s) if s@ == "check-observation-age"@ => if count == 1 { Command::Age } else { Command::Usage },
        Some(s) if s@ == "evaluate-workflow-v1"@ && count == 1 => Command::Evaluate,
        _ => Command::Delegate,
    },
{
    match first {
        Some(s) if <&str>::exec_eq(s, "--version") && count == 1 => Command::Version,
        Some(s) if <&str>::exec_eq(s, "check-observation-age") => {
            if count == 1 { Command::Age } else { Command::Usage }
        },
        Some(s) if <&str>::exec_eq(s, "evaluate-workflow-v1") && count == 1 => Command::Evaluate,
        _ => Command::Delegate,
    }
}
}

#[cfg(not(verus_keep_ghost))]
fn main() -> std::process::ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    match select_command(args.first().and_then(|value| value.to_str()), args.len()) {
        Command::Version => {
            println!("chrono-judge-workflow {}", env!("CARGO_PKG_VERSION"));
            return std::process::ExitCode::SUCCESS;
        }
        Command::Usage => {
            eprintln!("E_USAGE: check-observation-age reads JSON from stdin");
            return 2.into();
        }
        Command::Age => {
            return match chrono_judge_workflow::check_observation_age(std::io::stdin().lock()) {
                Ok(()) => 0.into(),
                Err((code, message)) => {
                    eprintln!("{message}");
                    code.into()
                }
            };
        }
        Command::Evaluate => {
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
        Command::Delegate => {}
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
