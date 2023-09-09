{
  description = "The Technomancy engine game project";
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

  outputs = inputs:
    inputs.flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import inputs.nixpkgs {
          inherit system;
          overlays = [ (import inputs.rust-overlay) ];
        };

        rustTarget = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        craneLib = (inputs.crane.mkLib pkgs).overrideToolchain rustTarget;

        fmtRustTarget = pkgs.rust-bin.selectLatestNightlyWith (toolchain: pkgs.rust-bin.fromRustupToolchain { channel = "nightly"; components = [ "rustfmt" ]; });
        fmtCraneLib = (inputs.crane.mkLib pkgs).overrideToolchain fmtRustTarget;

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
          pname = "technomancy_engine";
        };


        cargoArtifacts = craneLib.buildDepsOnly (commonArgs // { });

        technomancy-engine = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts version;
        });

        rustfmt' = pkgs.writeShellScriptBin "rustfmt" ''
          exec "${fmtRustTarget}/bin/rustfmt" "$@"
        '';

      in
      rec {
        checks = {
          inherit technomancy-engine;

          technomancy-engine-clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "-- --deny warnings";
          });

          technomancy-engine-fmt = fmtCraneLib.cargoFmt (commonArgs // { });
        };

        packages.technomancy-engine = technomancy-engine;
        packages.default = packages.technomancy-engine;

        apps.technomancy-engine = inputs.flake-utils.lib.mkApp {
          name = "technomancy-engine";
          drv = technomancy-engine;
        };
        apps.default = apps.technomancy-engine;

        devShells.default = devShells.technomancy-engine;
        devShells.technomancy-engine = pkgs.mkShell {
          inputsFrom = [ technomancy-engine ];

          nativeBuildInputs = [
            rustfmt'
            rustTarget
            pkgs.bacon
            pkgs.nodePackages.mermaid-cli
          ];
        };
      }
    );
}
