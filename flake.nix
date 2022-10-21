{
  description = "The Technomancy game project";
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-22.05";
    flake-utils = {
      url = "github:numtide/flake-utils";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };
  };

  outputs = { self, nixpkgs, crane, flake-utils, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        rustTarget = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        craneLib = (crane.mkLib pkgs).overrideToolchain rustTarget;

        tomlInfo = craneLib.crateNameFromCargoToml { cargoToml = ./Cargo.toml; };
        inherit (tomlInfo) pname version;
        src = ./.;

        cargoArtifacts = craneLib.buildDepsOnly {
          inherit src;
        };

        technomancy = craneLib.buildPackage {
          inherit cargoArtifacts src version;
        };

      in
      rec {
        checks = {
          inherit technomancy;

          technomancy-clippy = craneLib.cargoClippy {
            inherit cargoArtifacts src;
            cargoClippyExtraArgs = "-- --deny warnings";
          };

          technomancy-fmt = craneLib.cargoFmt {
            inherit src;
          };
        };

        packages.technomancy = technomancy;
        packages.default = packages.technomancy;

        apps.technomancy = flake-utils.lib.mkApp {
          name = "technomancy";
          drv = technomancy;
        };
        apps.default = apps.technomancy;

        devShells.default = devShells.technomancy;
        devShells.technomancy = pkgs.mkShell {
          buildInputs = [
          ];

          nativeBuildInputs = [
            rustTarget
          ];
        };
      }
    );
}
