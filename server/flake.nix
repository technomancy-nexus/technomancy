{
  description = "The Technomancy game server";
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


        cargoArtifacts = craneLib.buildDepsOnly {
          inherit src;
        };

        server = craneLib.buildPackage {
          inherit cargoArtifacts src version;
        };

      in
      rec {
        checks = {
          inherit server;

          server-clippy = craneLib.cargoClippy {
            inherit cargoArtifacts src;
            cargoClippyExtraArgs = "-- --deny warnings";
          };

          server-fmt = craneLib.cargoFmt {
            inherit src;
          };
        };

        packages.server = server;
        packages.default = packages.server;

        apps.server = flake-utils.lib.mkApp {
          name = "server";
          drv = server;
        };
        apps.default = apps.server;

        devShells.default = devShells.server;
        devShells.server = pkgs.mkShell {
          inputsFrom = [ ];

          nativeBuildInputs = [
            rustTarget
          ];
        };
      }
    );
}
