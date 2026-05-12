{ pkgs, ... }:
{
  package = pkgs.wofr;
  settings = {
    entire = {
      agents = [ "opencode" ];
      checkpoint_remote = "github:rencire/wofr-checkpoints";
    };
  };
}
