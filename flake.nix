{
  description = "Laskugeneraattori";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
    # Vendors the manifest of every Rust release, so rust-toolchain.toml can be bumped (by hand or
    # by Renovate) without also updating a hash here
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
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
      rust-overlay,
      treefmt-nix,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        lib = pkgs.lib;

        craneLib = (crane.mkLib pkgs).overrideToolchain (
          p: p.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml
        );

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

        # Dependencies of the shipped binary (release profile, fat LTO). Only
        # `cargo build` is needed here: clippy and tests use the `ci` profile.
        cargoArtifacts = craneLib.buildDepsOnly (
          commonArgs
          // {
            doCheck = false;
            buildPhaseCargoCommand = "cargoWithProfile build --locked";
          }
        );

        # Clippy and tests use the `ci` profile (release without LTO) so that
        # every test binary doesn't go through a slow fat-LTO link.
        ciArgs = commonArgs // {
          CARGO_PROFILE = "ci";
        };

        ciCargoArtifacts = craneLib.buildDepsOnly ciArgs;

        laskugeneraattori = craneLib.buildPackage (
          commonArgs
          // {
            inherit cargoArtifacts;
            # Tests run in the `laskugeneraattori-test` check instead
            doCheck = false;
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
            ciArgs
            // {
              cargoArtifacts = ciCargoArtifacts;
              doInstallCargoArtifacts = false;
            }
          );

          laskugeneraattori-test = craneLib.cargoTest (
            ciArgs
            // {
              cargoArtifacts = ciCargoArtifacts;
              doInstallCargoArtifacts = false;
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
          # Compiled dependencies. CI keeps a GC root to this so that they are
          # the only thing saved to (and restored from) the Actions cache.
          deps = pkgs.linkFarm "laskugeneraattori-deps" [
            {
              name = "release";
              path = cargoArtifacts;
            }
            {
              name = "ci";
              path = ciCargoArtifacts;
            }
          ];
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
