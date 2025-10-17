{
  description = "StylusPort::Solana Handbook & CLI Tool";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    git-hooks-nix.inputs.nixpkgs.follows = "nixpkgs";
    git-hooks-nix.url = "github:cachix/git-hooks.nix";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs =
    inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [
        inputs.git-hooks-nix.flakeModule
      ];
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
        "x86_64-darwin"
      ];
      perSystem =
        {
          pkgs,
          system,
          config,
          ...
        }:
        let
          rust-toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
          cargo-deps = pkgs.rustPlatform.importCargoLock {
            lockFile = ./Cargo.lock;
          };
        in
        {
          _module.args.pkgs = import inputs.nixpkgs {
            inherit system;
            overlays = [ inputs.rust-overlay.overlays.default ];
          };

          devShells.default = pkgs.mkShell {
            buildInputs = [ pkgs.openssl ];
            packages =
              [
                pkgs.cargo-expand
                pkgs.mdbook
                pkgs.rust-analyzer-unwrapped
                pkgs.vale
                rust-toolchain
              ]
              ++ pkgs.lib.optionals pkgs.stdenv.isLinux [
                pkgs.pkg-config
              ];

            shellHook = ''
              ${config.pre-commit.installationScript}
              echo 1>&2 "Welcome to the StylusPort development shell!"
            '';

            RUST_SRC_PATH = "${rust-toolchain}/lib/rustlib/src/rust/library";
          };

          pre-commit.settings = {
            settings.rust = {
              cargoManifestPath = "./Cargo.toml";
              check.cargoDeps = cargo-deps;
            };
            hooks = {
              clippy = {
                enable = true;
                packageOverrides = {
                  cargo = rust-toolchain;
                  clippy = rust-toolchain;
                };
              };
              rustfmt.enable = true;
              vale.enable = true; # spelling & prose lint
            };
          };
        };
    };
}
