# wofr

`wofr` is a small Rust CLI that wraps `nix flake init`, `nix flake new`,
and Entire project setup for templates hosted in
`github:rencire/flake-templates/main`.

In this repo, the local `wofr` wrapper configuration is owned by Nix in
`nix/confix/wofr.nix`. Standalone repos can still use `wofr.toml` directly.

## Build

```sh
nix build
```

Build artifacts are linked at `./result/`.

## Run

Show help:

```sh
nix run . -- --help
```

Run commands:

```sh
nix run . -- init TEMPLATE
nix run . -- new TEMPLATE DEST_DIR
nix run . -- entire-init
```

## Usage

```sh
wofr [--config PATH] [--refresh] init TEMPLATE
wofr [--config PATH] [--refresh] new TEMPLATE DEST_DIR
wofr [--config PATH] entire-init [--agent NAME]... [--checkpoint-remote VALUE]
```

## Entire Setup

Run `wofr entire-init` inside a generated Git repo after `nix develop` to
enable Entire and write `.entire/settings.json`.

`wofr` reads `wofr.toml` from the current directory by default when no
`--config` flag is provided.

```toml
[entire]
agents = ["opencode"]

[entire.checkpoint_remote]
provider = "github"
repo = "<owner>/<repo>"
```

Precedence order:

1. CLI flags
2. `wofr.toml`
3. built-in defaults

Supported `entire-init` flags:

- `--config PATH`: read a different config file
- `--agent NAME`: override the agent list, repeat to add multiple agents
- `--checkpoint-remote VALUE`: override the checkpoint remote using `<owner>/<repo>`
