{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
  };

  outputs = {
    self,
    nixpkgs,
  }: let
    system = "x86_64-linux";
    pkgs = import nixpkgs {inherit system;};
    customBuildRustCrateForPkgs = pkgs:
      pkgs.buildRustCrate.override {
        defaultCrateOverrides =
          pkgs.defaultCrateOverrides
          // {
            fuser = attrs: {
              buildInputs = [pkgs.pkg-config pkgs.fuse3];
            };
          };
      };
    cargoNix = pkgs.callPackage ./Cargo.nix {
      buildRustCrateForPkgs = customBuildRustCrateForPkgs;
    };
  in {
    checks.${system} = {
      bwfs = cargoNix.rootCrate.build.override {
        runTests = true;
      };
    };

    packages.${system} = {
      bwfs = cargoNix.rootCrate.build;
      bwfs-bw = pkgs.writeShellScriptBin "bwfs-bw" ''
        ${pkgs.lib.getExe self.packages.${system}.bwfs} --bw-bin ${pkgs.lib.getExe pkgs.bitwarden-cli} $@
      '';
    };

    devShells.${system}.default = pkgs.mkShell {
      packages = [
        pkgs.rustc
        pkgs.cargo
        pkgs.rustfmt
        pkgs.clippy

        pkgs.crate2nix

        pkgs.pkg-config
        pkgs.fuse3
        pkgs.openssl

        pkgs.bitwarden-cli
      ];
    };
  };
}
