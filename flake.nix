{
  description = "The Technomancy game project";
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-22.05";
    flake-utils = {
      url = "github:numtide/flake-utils";
    };
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
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
        src =
          let
            markdownFilter = path: _type: !((pkgs.lib.hasSuffix ".md" path) && builtins.baseNameOf path != "README.md");
            nixFilter = path: _type: !pkgs.lib.hasSuffix ".nix" path;
            extraFiles = path: _type: !(builtins.any (n: pkgs.lib.hasSuffix n path) [ ".github" ".sh" ]);
            filterPath = path: type: builtins.all (f: f path type) [
              markdownFilter
              nixFilter
              extraFiles
              pkgs.lib.cleanSourceFilter
            ];
          in
          pkgs.lib.cleanSourceWith {
            src = ./.;
            filter = filterPath;
          };

        commonArgs = {
          inherit src;
          pname = "technomancy";
        };


        cargoArtifacts = craneLib.buildDepsOnly (commonArgs // { });

        technomancy-engine = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts version;
        });

      in
      rec {
        checks = {
          inherit technomancy-engine;

          technomancy-clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "-- --deny warnings";
          });

          technomancy-fmt = craneLib.cargoFmt (commonArgs // { });
        };

        packages.technomancy-engine = technomancy-engine;
        packages.default = packages.technomancy-engine;

        apps.technomancy-engine = flake-utils.lib.mkApp {
          name = "technomancy-engine";
          drv = technomancy-engine;
        };
        apps.default = apps.technomancy-engine;

        devShells.default = devShells.technomancy-engine;
        devShells.technomancy-engine = pkgs.mkShell {
          inputsFrom = [ technomancy-engine ];

          nativeBuildInputs = [
            rustTarget
          ];
        };
      }
    );
}
