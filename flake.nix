{
  description = "Laskugeneraattori";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      crane,
      flake-utils,
      fenix,
      treefmt-nix,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ fenix.overlays.default ];
        };

        toolchain = fenix.packages.${system}.fromToolchainFile {
          file = ./rust-toolchain.toml;
          sha256 = "sha256-zC8E38iDVJ1oPIzCqTk/Ujo9+9kx9dXq7wAwPMpkpg0=";
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
              (lib.fileset.maybeMissing ./testdata)
            ];
          };

          strictDeps = true;

          CARGO_BUILD_TARGET = "x86_64-unknown-linux-musl";

          buildInputs = lib.optionals pkgs.stdenv.isDarwin [ pkgs.libiconv ];
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        laskugeneraattori = craneLib.buildPackage (
          commonArgs
          // {
            inherit cargoArtifacts;
            GIT_COMMIT_SHA = toString (self.rev or self.dirtyRev or self.lastModified or "dirty");
          }
        );

        treefmtEval = treefmt-nix.lib.evalModule pkgs ./treefmt.nix;
      in
      {
        formatter = treefmtEval.config.build.wrapper;

        checks = {
          inherit laskugeneraattori;

          laskugeneraattori-clippy = craneLib.cargoClippy (
            commonArgs
            // {
              inherit cargoArtifacts;
            }
          );

          laskugeneraattori-test = craneLib.cargoTest (
            commonArgs
            // {
              inherit cargoArtifacts;
              GIT_COMMIT_SHA = toString (self.rev or self.dirtyRev or self.lastModified or "dirty");
              MAILGUN_URL = "https://api.eu.mailgun.net/v3/laskutus.tietokilta.fi/messages";
              MAILGUN_USER = "api";
              MAILGUN_PASSWORD = "password";
              MAILGUN_TO = "Rahastonhoitaja <rahastonhoitaja@tietokilta.fi>";
              MAILGUN_FROM = "noreply@laskutus.tietokilta.fi";
            }
          );

          formatting = treefmtEval.config.build.check self;
        };

        packages = {
          default = laskugeneraattori;
          docker = pkgs.dockerTools.buildLayeredImage {
            name = "laskugeneraattori";
            tag = "latest";
            config = {
              Cmd = [ "${laskugeneraattori}/bin/laskugeneraattori" ];
              Env = [ "BIND_ADDR=0.0.0.0" ];
              ExposedPorts."3000/tcp" = { };
            };
          };
        };

        devShells.default = craneLib.devShell {
          checks = self.checks.${system};
          packages = [ ];
        };
      }
    );
}
