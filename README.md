# wofr

`wofr` is a small Rust CLI that wraps `nix flake init` and `nix flake new` for
templates hosted in `github:rencire/flake-templates/main`.

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
```

## Usage

```sh
wofr [--refresh] init TEMPLATE
wofr [--refresh] new TEMPLATE DEST_DIR
```
