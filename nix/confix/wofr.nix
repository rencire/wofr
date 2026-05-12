{ pkgs, ... }:
{
  package = pkgs.callPackage ../package.nix { };
  settings = {
    entire = {
      agents = [ "opencode" ];
      checkpoint_remote = "github:rencire/wofr-checkpoints";
    };
  };
}
