use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use serde::Deserialize;

const TEMPLATE_REPO: &str = "github:rencire/flake-templates/main";
const DEFAULT_CONFIG_PATH: &str = "wofr.toml";
const DEFAULT_ENTIRE_AGENTS: &[&str] = &["opencode"];
const DEFAULT_CHECKPOINT_REMOTE: &str = "github:rencire/wofr-checkpoints";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Init,
    New,
    EntireInit,
}

#[derive(Debug)]
struct Config {
    action: Action,
    template: Option<String>,
    dest_dir: Option<PathBuf>,
    refresh: bool,
    config_path: Option<PathBuf>,
    entire_init: EntireInitArgs,
}

#[derive(Debug, Default, PartialEq, Eq)]
struct EntireInitArgs {
    config_path: Option<PathBuf>,
    agents: Vec<String>,
    checkpoint_remote: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
struct EntireInitConfig {
    agents: Vec<String>,
    checkpoint_remote: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct WofrConfigFile {
    entire: Option<EntireConfigFile>,
}

#[derive(Debug, Default, Deserialize)]
struct EntireConfigFile {
    agents: Option<Vec<String>>,
    checkpoint_remote: Option<String>,
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
        Action::EntireInit => run_entire_init(&cfg.entire_init)?,
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
    let mut config_path = None;

    let mut action_index = None;
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if arg == "--refresh" {
            refresh = true;
            index += 1;
        } else if arg == "--config" {
            let value = args
                .get(index + 1)
                .ok_or_else(|| "--config requires PATH".to_string())?;
            config_path = Some(PathBuf::from(value));
            index += 2;
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
        "entire-init" => Action::EntireInit,
        other => {
            return Err(format!(
                "action must be 'init', 'new', or 'entire-init', got '{other}'"
            ))
        }
    };

    match action {
        Action::Init => {
            let positional = parse_template_args(action_args, &mut refresh)?;
            let template = positional.first().cloned().ok_or_else(|| "missing TEMPLATE".to_string())?;

            if positional.len() > 1 {
                return Err("unexpected extra arguments".to_string());
            }

            Ok(Config {
                action,
                template: Some(template),
                dest_dir: None,
                refresh,
                config_path,
                entire_init: EntireInitArgs::default(),
            })
        }
        Action::New => {
            let positional = parse_template_args(action_args, &mut refresh)?;
            let template = positional.first().cloned().ok_or_else(|| "missing TEMPLATE".to_string())?;
            let dest_dir = positional.get(1).cloned().ok_or_else(|| "new requires DEST_DIR".to_string())?;

            if positional.len() > 2 {
                return Err("unexpected extra arguments".to_string());
            }

            Ok(Config {
                action,
                template: Some(template),
                dest_dir: Some(PathBuf::from(dest_dir)),
                refresh,
                config_path,
                entire_init: EntireInitArgs::default(),
            })
        }
        Action::EntireInit => {
            if refresh {
                return Err("--refresh is only supported for 'init' and 'new'".to_string());
            }

            let entire_init = parse_entire_init_args(action_args)?;

            Ok(Config {
                action,
                template: None,
                dest_dir: None,
                refresh: false,
                config_path: config_path.clone(),
                entire_init: EntireInitArgs {
                    config_path: entire_init.config_path.or(config_path),
                    agents: entire_init.agents,
                    checkpoint_remote: entire_init.checkpoint_remote,
                },
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

fn parse_entire_init_args(args: &[String]) -> Result<EntireInitArgs, String> {
    let mut parsed = EntireInitArgs::default();
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--config" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--config requires PATH".to_string())?;
                parsed.config_path = Some(PathBuf::from(value));
                index += 2;
            }
            "--agent" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--agent requires NAME".to_string())?;
                parsed.agents.push(value.clone());
                index += 2;
            }
            "--checkpoint-remote" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--checkpoint-remote requires VALUE".to_string())?;
                parsed.checkpoint_remote = Some(value.clone());
                index += 2;
            }
            other => return Err(format!("unexpected argument '{other}'")),
        }
    }

    Ok(parsed)
}

fn run_entire_init(args: &EntireInitArgs) -> Result<(), String> {
    let config = resolve_entire_init_config(args)?;
    let first_agent = config
        .agents
        .first()
        .ok_or_else(|| "entire agents must contain at least one agent".to_string())?;

    if let Some(checkpoint_remote) = config.checkpoint_remote {
        write_checkpoint_config(&checkpoint_remote)?;
    }

    run_command(
        "entire",
        &[
            "enable".to_string(),
            "--project".to_string(),
            "--agent".to_string(),
            first_agent.to_string(),
        ],
        None,
    )?;

    for agent in &config.agents[1..] {
        run_command(
            "entire",
            &["agent".to_string(), "add".to_string(), agent.clone()],
            None,
        )?;
    }

    Ok(())
}

fn resolve_entire_init_config(args: &EntireInitArgs) -> Result<EntireInitConfig, String> {
    let file = load_config_file(args.config_path.as_deref())?;
    let file_entire = file.entire.unwrap_or_default();

    let agents = if args.agents.is_empty() {
        file_entire.agents.unwrap_or_else(default_entire_agents)
    } else {
        args.agents.clone()
    };

    if agents.is_empty() {
        return Err("entire agents must contain at least one agent".to_string());
    }

    let checkpoint_remote = args
        .checkpoint_remote
        .clone()
        .or(file_entire.checkpoint_remote)
        .or_else(|| Some(DEFAULT_CHECKPOINT_REMOTE.to_string()));

    Ok(EntireInitConfig {
        agents,
        checkpoint_remote,
    })
}

fn load_config_file(config_path: Option<&Path>) -> Result<WofrConfigFile, String> {
    match config_path {
        Some(path) => parse_config_file(path).map(Some),
        None => {
            let path = Path::new(DEFAULT_CONFIG_PATH);
            if path.exists() {
                parse_config_file(path).map(Some)
            } else {
                Ok(None)
            }
        }
    }
    .map(|config| config.unwrap_or_default())
}

fn parse_config_file(path: &Path) -> Result<WofrConfigFile, String> {
    let content = fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    toml::from_str(&content).map_err(|err| format!("failed to parse {}: {err}", path.display()))
}

fn default_entire_agents() -> Vec<String> {
    DEFAULT_ENTIRE_AGENTS.iter().map(|agent| agent.to_string()).collect()
}

fn write_checkpoint_config(checkpoint_remote: &str) -> Result<(), String> {
    let settings_dir = Path::new(".entire");
    fs::create_dir_all(settings_dir).map_err(|err| err.to_string())?;
    let settings_path = settings_dir.join("settings.json");
    fs::write(settings_path, checkpoint_settings_json(checkpoint_remote)).map_err(|err| err.to_string())
}

fn checkpoint_settings_json(checkpoint_remote: &str) -> String {
    format!(
        concat!(
            "{{\n",
            "  \"enabled\": true,\n",
            "  \"telemetry\": false,\n",
            "  \"strategy_options\": {{\n",
            "    \"checkpoint_remote\": \"{}\"\n",
            "  }}\n",
            "}}\n"
        ),
        escape_json(checkpoint_remote)
    )
}

fn escape_json(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            _ => escaped.push(ch),
        }
    }
    escaped
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
    "Usage:\n  wofr [--config PATH] [--refresh] init TEMPLATE\n  wofr [--config PATH] [--refresh] new TEMPLATE DEST_DIR\n  wofr [--config PATH] entire-init [--agent NAME]... [--checkpoint-remote VALUE]"
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
        assert_eq!(cfg.config_path, None);
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
        assert_eq!(cfg.config_path, None);
    }

    #[test]
    fn parses_entire_init_command() {
        let cfg = parse_args(["entire-init"].into_iter().map(str::to_string)).unwrap();
        assert_eq!(cfg.action, Action::EntireInit);
        assert_eq!(cfg.template, None);
        assert_eq!(cfg.dest_dir, None);
        assert!(!cfg.refresh);
        assert_eq!(cfg.config_path, None);
        assert_eq!(cfg.entire_init, EntireInitArgs::default());
    }

    #[test]
    fn parses_global_config_path() {
        let cfg = parse_args(
            ["--config", "from-wrapper.toml", "entire-init"]
                .into_iter()
                .map(str::to_string),
        )
        .unwrap();

        assert_eq!(cfg.config_path, Some(PathBuf::from("from-wrapper.toml")));
        assert_eq!(cfg.entire_init.config_path, Some(PathBuf::from("from-wrapper.toml")));
    }

    #[test]
    fn rejects_refresh_for_entire_init() {
        let err = parse_args(["--refresh", "entire-init"].into_iter().map(str::to_string))
            .unwrap_err();
        assert!(err.contains("--refresh"));
    }

    #[test]
    fn parses_entire_init_flags() {
        let cfg = parse_args(
            [
                "entire-init",
                "--config",
                "custom.toml",
                "--agent",
                "opencode",
                "--agent",
                "codex",
                "--checkpoint-remote",
                "github:rencire/custom",
            ]
            .into_iter()
            .map(str::to_string),
        )
        .unwrap();

        assert_eq!(
            cfg.entire_init,
            EntireInitArgs {
                config_path: Some(PathBuf::from("custom.toml")),
                agents: vec!["opencode".to_string(), "codex".to_string()],
                checkpoint_remote: Some("github:rencire/custom".to_string()),
            }
        );
    }

    #[test]
    fn resolves_defaults_without_config_file() {
        let config = resolve_entire_init_config(&EntireInitArgs::default()).unwrap();
        assert_eq!(config.agents, vec!["opencode".to_string()]);
        assert_eq!(
            config.checkpoint_remote,
            Some("github:rencire/wofr-checkpoints".to_string())
        );
    }

    #[test]
    fn resolves_config_file_values() {
        let temp_dir = make_temp_dir("config-file");
        let config_path = temp_dir.join("wofr.toml");
        fs::write(
            &config_path,
            "[entire]\nagents = [\"codex\", \"opencode\"]\ncheckpoint_remote = \"github:rencire/from-file\"\n",
        )
        .unwrap();

        let config = resolve_entire_init_config(&EntireInitArgs {
            config_path: Some(config_path.clone()),
            agents: Vec::new(),
            checkpoint_remote: None,
        })
        .unwrap();

        assert_eq!(config.agents, vec!["codex".to_string(), "opencode".to_string()]);
        assert_eq!(config.checkpoint_remote, Some("github:rencire/from-file".to_string()));

        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn cli_flags_override_config_file() {
        let temp_dir = make_temp_dir("cli-overrides");
        let config_path = temp_dir.join("wofr.toml");
        fs::write(
            &config_path,
            "[entire]\nagents = [\"codex\"]\ncheckpoint_remote = \"github:rencire/from-file\"\n",
        )
        .unwrap();

        let config = resolve_entire_init_config(&EntireInitArgs {
            config_path: Some(config_path.clone()),
            agents: vec!["opencode".to_string()],
            checkpoint_remote: Some("github:rencire/from-cli".to_string()),
        })
        .unwrap();

        assert_eq!(config.agents, vec!["opencode".to_string()]);
        assert_eq!(config.checkpoint_remote, Some("github:rencire/from-cli".to_string()));

        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn escapes_checkpoint_remote_in_json() {
        let json = checkpoint_settings_json("github:rencire/wofr\"checkpoints");
        assert!(json.contains("\\\"checkpoints"));
    }

    #[test]
    fn detects_help_flags() {
        assert!(wants_help(["--help"].into_iter().map(str::to_string)));
        assert!(wants_help(["help"].into_iter().map(str::to_string)));
        assert!(!wants_help(["init", "rust"].into_iter().map(str::to_string)));
    }

    fn make_temp_dir(name: &str) -> PathBuf {
        let unique = format!(
            "wofr-test-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let path = env::temp_dir().join(unique);
        fs::create_dir_all(&path).unwrap();
        path
    }
}
