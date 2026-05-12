use std::env;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

const TEMPLATE_REPO: &str = "github:rencire/flake-templates/main";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Init,
    New,
}

#[derive(Debug)]
struct Config {
    action: Action,
    template: String,
    dest_dir: Option<PathBuf>,
    refresh: bool,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("wofr: {message}");
            eprintln!();
            eprintln!("{}", usage());
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<(), String> {
    let cfg = parse_args(env::args().skip(1))?;
    let template_ref = format!("{TEMPLATE_REPO}#{}", cfg.template);

    match cfg.action {
        Action::Init => {
            let mut args = vec!["flake".to_string(), "init".to_string()];
            if cfg.refresh {
                args.push("--refresh".to_string());
            }
            args.push("-t".to_string());
            args.push(template_ref);
            run_command("nix", &args, None)?;
        }
        Action::New => {
            let dest_dir = cfg
                .dest_dir
                .as_ref()
                .ok_or_else(|| "new requires DEST_DIR".to_string())?;
            let dest_dir = dest_dir
                .to_str()
                .ok_or_else(|| "DEST_DIR must be valid UTF-8".to_string())?;
            let mut args = vec!["flake".to_string(), "new".to_string()];
            if cfg.refresh {
                args.push("--refresh".to_string());
            }
            args.push(dest_dir.to_string());
            args.push("-t".to_string());
            args.push(template_ref);
            run_command("nix", &args, None)?;
        }
    }

    Ok(())
}

fn parse_args<I>(mut args: I) -> Result<Config, String>
where
    I: Iterator<Item = String>,
{
    let mut positional = Vec::new();
    let mut refresh = false;

    for arg in args.by_ref() {
        match arg.as_str() {
            "--refresh" => refresh = true,
            other => positional.push(other.to_string()),
        }
    }

    let mut positional = positional.into_iter();

    let action = match positional.next().as_deref() {
        Some("init") => Action::Init,
        Some("new") => Action::New,
        Some(other) => return Err(format!("action must be 'init' or 'new', got '{other}'")),
        None => return Err("missing action".to_string()),
    };

    let template = positional
        .next()
        .ok_or_else(|| "missing TEMPLATE".to_string())?;

    match action {
        Action::Init => {
            if positional.next().is_some() {
                return Err("unexpected extra arguments".to_string());
            }

            Ok(Config {
                action,
                template,
                dest_dir: None,
                refresh,
            })
        }
        Action::New => {
            let dest_dir = positional
                .next()
                .ok_or_else(|| "new requires DEST_DIR".to_string())?;

            if positional.next().is_some() {
                return Err("unexpected extra arguments".to_string());
            }

            Ok(Config {
                action,
                template,
                dest_dir: Some(PathBuf::from(dest_dir)),
                refresh,
            })
        }
    }
}

fn run_command(cmd: &str, args: &[String], cwd: Option<&PathBuf>) -> Result<(), String> {
    let mut command = Command::new(cmd);
    command.args(args);

    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }

    let status = command.status().map_err(|err| err.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{cmd} exited with status {status}"))
    }
}

fn usage() -> &'static str {
    "Usage:\n  wofr [--refresh] init TEMPLATE\n  wofr [--refresh] new TEMPLATE DEST_DIR"
}
