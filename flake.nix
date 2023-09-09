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

        rustfmt' = pkgs.writeShellScriptBin "rustfmt" ''
          exec "${fmtRustTarget}/bin/rustfmt" "$@"
        '';

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
          pname = commonArgs.pname;
        };

        commonArgs = {
          inherit src cargoArtifacts;
          pname = "technomancy";
        };

        technomancy = craneLib.buildPackage (commonArgs // { });
      in
      rec {
        checks = {
          inherit technomancy;

          technomancy-clippy = craneLib.cargoClippy (commonArgs // {
            cargoClippyExtraArgs = "-- --deny warnings";
          });

          technomancy-fmt = fmtCraneLib.cargoFmt (commonArgs // { });
        };

        packages.technomancy = technomancy;
        packages.default = packages.technomancy;

        apps.technomancy = inputs.flake-utils.lib.mkApp {
          name = "technomancy";
          drv = technomancy;
        };
        apps.default = apps.technomancy;

        devShells.default = devShells.technomancy;
        devShells.technomancy = pkgs.mkShell {
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
