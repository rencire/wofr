# wofr

`wofr` is a small Rust CLI that wraps `nix flake init` and `nix flake new`
for templates hosted in `github:rencire/flake-templates/main`.

## Build

```sh
nix build .#wofr
```

## Usage

```sh
wofr [--refresh] init TEMPLATE
wofr [--refresh] new TEMPLATE DEST_DIR
```
