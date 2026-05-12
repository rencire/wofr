{ rustPlatform, lib }:

rustPlatform.buildRustPackage {
  pname = "wofr";
  version = "0.1.0";

  src = ../.;
  cargoLock.lockFile = ../Cargo.lock;

  meta = {
    description = "Rust CLI for creating flakes from rencire templates";
    homepage = "https://github.com/rencire/flake-templates";
    license = lib.licenses.mit;
    mainProgram = "wofr";
  };
}
