{
  description = "Rust CLI for creating flakes from rencire templates";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flakelight = {
      url = "github:accelbread/flakelight";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    agent-skills = {
      url = "github:Kyure-A/agent-skills-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rencire-skills = {
      url = "github:rencire/agent-skills";
      flake = false;
    };
    entire-cli-flake = {
      url = "github:rencire/entire-cli-flake";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    llm-agents = {
      url = "github:numtide/llm-agents.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nix-wrapper-modules = {
      url = "github:rencire/nix-wrapper-modules/feat/wofr-wrapper";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    confix = {
      url = "github:rencire/confix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.nix-wrapper-modules.follows = "nix-wrapper-modules";
    };
  };

  outputs =
    { flakelight, ... }@inputs:
    let
      agentSkillsLib = inputs."agent-skills".lib."agent-skills";
      mkAgentBundle =
        pkgs:
        let
          sources = {
            shared = {
              path = inputs."rencire-skills";
              subdir = "skills";
            };
          };
        in
        agentSkillsLib.mkBundle {
          inherit pkgs;
          selection = agentSkillsLib.selectSkills {
            catalog = agentSkillsLib.discoverCatalog sources;
            inherit sources;
            allowlist = [
              "dev-loop"
              "doc-table-of-contents"
              "nix-repo"
              "public-repo-readiness"
              "vcs"
            ];
            skills = { };
          };
        };
      localTargets = {
        agents = agentSkillsLib.defaultLocalTargets.agents // {
          enable = true;
        };
        claude = agentSkillsLib.defaultLocalTargets.claude // {
          enable = false;
        };
      };
    in
    flakelight ./. {
      inherit inputs;
      systems = [
        "aarch64-darwin"
        "aarch64-linux"
        "x86_64-darwin"
        "x86_64-linux"
      ];
      devShell =
        pkgs:
        let
          pkgs' = pkgs.extend inputs."llm-agents".overlays.shared-nixpkgs;
          bundle = mkAgentBundle pkgs';
          configured = inputs.confix.lib.configure {
            pkgs = pkgs';
            configDir = ./nix/confix;
          };
        in
        {
          packages = [
            inputs."entire-cli-flake".packages.${pkgs'.system}.default
            configured.opencode
            configured.wofr
            # pkgs'.llm-agents.claude-code
            # pkgs'.llm-agents.codex
            # pkgs'.llm-agents.gemini-cli
            pkgs'.cargo
            pkgs'.git
            pkgs'.rustc
          ];
          shellHook = agentSkillsLib.mkShellHook {
            pkgs = pkgs';
            inherit bundle;
            targets = localTargets;
          };
        };
    };
}
