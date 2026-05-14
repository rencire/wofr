use std::env;
use std::path::{Path, PathBuf};
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
    template: Option<String>,
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
    if wants_help(env::args().skip(1)) {
        print_usage();
        return Ok(());
    }

    let cfg = parse_args(env::args().skip(1))?;

    match cfg.action {
        Action::Init => {
            let template = cfg
                .template
                .as_deref()
                .ok_or_else(|| "init requires TEMPLATE".to_string())?;
            let template_ref = format!("{TEMPLATE_REPO}#{template}");
            let mut args = vec!["flake".to_string(), "init".to_string()];
            if cfg.refresh {
                args.push("--refresh".to_string());
            }
            args.push("-t".to_string());
            args.push(template_ref);
            run_command("nix", &args, None)?;
        }
        Action::New => {
            let template = cfg
                .template
                .as_deref()
                .ok_or_else(|| "new requires TEMPLATE".to_string())?;
            let template_ref = format!("{TEMPLATE_REPO}#{template}");
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

fn wants_help<I>(args: I) -> bool
where
    I: Iterator<Item = String>,
{
    args.into_iter().any(|arg| arg == "--help" || arg == "-h" || arg == "help")
}

fn print_usage() {
    println!("{}", usage());
}

fn parse_args<I>(args: I) -> Result<Config, String>
where
    I: Iterator<Item = String>,
{
    let args: Vec<String> = args.collect();
    let mut refresh = false;

    let mut action_index = None;
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if arg == "--refresh" {
            refresh = true;
            index += 1;
        } else {
            action_index = Some(index);
            break;
        }
    }

    let action_index = action_index.ok_or_else(|| "missing action".to_string())?;
    let action_name = &args[action_index];
    let action_args = &args[action_index + 1..];

    let action = match action_name.as_str() {
        "init" => Action::Init,
        "new" => Action::New,
        other => {
            return Err(format!("action must be 'init' or 'new', got '{other}'"))
        }
    };

    match action {
        Action::Init => {
            let positional = parse_template_args(action_args, &mut refresh)?;
            let template = positional
                .first()
                .cloned()
                .ok_or_else(|| "missing TEMPLATE".to_string())?;

            if positional.len() > 1 {
                return Err("unexpected extra arguments".to_string());
            }

            Ok(Config {
                action,
                template: Some(template),
                dest_dir: None,
                refresh,
            })
        }
        Action::New => {
            let positional = parse_template_args(action_args, &mut refresh)?;
            let template = positional
                .first()
                .cloned()
                .ok_or_else(|| "missing TEMPLATE".to_string())?;
            let dest_dir = positional
                .get(1)
                .cloned()
                .ok_or_else(|| "new requires DEST_DIR".to_string())?;

            if positional.len() > 2 {
                return Err("unexpected extra arguments".to_string());
            }

            Ok(Config {
                action,
                template: Some(template),
                dest_dir: Some(PathBuf::from(dest_dir)),
                refresh,
            })
        }
    }
}

fn parse_template_args(args: &[String], refresh: &mut bool) -> Result<Vec<String>, String> {
    let mut positional = Vec::new();
    for arg in args {
        if arg == "--refresh" {
            *refresh = true;
        } else {
            positional.push(arg.clone());
        }
    }
    Ok(positional)
}

fn run_command(cmd: &str, args: &[String], cwd: Option<&Path>) -> Result<(), String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_init_command() {
        let cfg = parse_args(["init", "rust"].into_iter().map(str::to_string)).unwrap();
        assert_eq!(cfg.action, Action::Init);
        assert_eq!(cfg.template.as_deref(), Some("rust"));
        assert_eq!(cfg.dest_dir, None);
        assert!(!cfg.refresh);
    }

    #[test]
    fn parses_new_command_with_refresh() {
        let cfg =
            parse_args(["--refresh", "new", "rust", "demo"].into_iter().map(str::to_string))
                .unwrap();
        assert_eq!(cfg.action, Action::New);
        assert_eq!(cfg.template.as_deref(), Some("rust"));
        assert_eq!(cfg.dest_dir, Some(PathBuf::from("demo")));
        assert!(cfg.refresh);
    }

    #[test]
    fn detects_help_flags() {
        assert!(wants_help(["--help"].into_iter().map(str::to_string)));
        assert!(wants_help(["help"].into_iter().map(str::to_string)));
        assert!(!wants_help(["init", "rust"].into_iter().map(str::to_string)));
    }
}
