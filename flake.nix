{
  description = "Laskugeneraattori";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    devenv.url = "github:cachix/devenv";
  };

  nixConfig = {
    extra-trusted-public-keys = "devenv.cachix.org-1:w1cLUi8dv3hnoSPGAuibQv+f9TZLr6cv/Hm9XgU50cw=";
    extra-substituters = "https://devenv.cachix.org";
  };

  outputs = {
    self,
    nixpkgs,
    crane,
    flake-utils,
    fenix,
    devenv,
    ...
  } @ inputs:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [fenix.overlays.default];
      };

      toolchain = fenix.packages.${system}.fromToolchainFile {
        file = ./rust-toolchain;
        sha256 = "sha256-yMuSb5eQPO/bHv+Bcf/US8LVMbf/G/0MSfiPwBhiPpk=";
      };

      lib = pkgs.lib;

      craneLib = (crane.mkLib pkgs).overrideToolchain toolchain;

      unfilteredRoot = ./.;

      commonArgs = {
        src = lib.fileset.toSource {
          root = unfilteredRoot;
          fileset = lib.fileset.unions [
            (craneLib.fileset.commonCargoSources unfilteredRoot)
            (lib.fileset.maybeMissing ./templates)
          ];
        };

        strictDeps = true;

        GIT_COMMIT_SHA = toString (self.rev or self.dirtyRev or self.lastModified or "dirty");

        buildInputs = lib.optionals pkgs.stdenv.isDarwin [pkgs.libiconv];

        doCheck = false;
      };

      cargoArtifacts = craneLib.buildDepsOnly commonArgs;

      laskugeneraattori = craneLib.buildPackage (
        commonArgs // {inherit cargoArtifacts;}
      );
    in {
      checks = {
        inherit laskugeneraattori;
      };

      packages = {
        default = laskugeneraattori;
        docker = pkgs.dockerTools.buildLayeredImage {
          name = "laskugeneraattori";
          config.Cmd = ["${laskugeneraattori}/bin/laskugeneraattori"];
        };
      };

      devShells.default = devenv.lib.mkShell {
        inherit inputs pkgs;
        modules = [
          {
            languages.rust = {
              enable = true;
              channel = "stable";
              inherit toolchain;
            };

            devcontainer.enable = true;
          }
        ];
      };
    });
}
