{
  description = "TaskTrack is an simple app to manage personal tasks";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
    flake-utils.url = "github:numtide/flake-utils";
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
      flake-utils,
      rust-overlay,
      treefmt-nix,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ rust-overlay.overlays.default ];
        pkgs = import nixpkgs {
          inherit system;
          overlays = overlays;
        };

        rust = pkgs.rust-bin.stable.latest.default;

        treefmt = treefmt-nix.lib.evalModule pkgs ./treefmt.nix;
      in
      {
        # Add a dev shell
        devShells.default = pkgs.mkShell {
          buildInputs = [
            rust

            # editor tools
            pkgs.rust-analyzer

            # other tools
            pkgs.cargo-generate

            # formatter
            treefmt.config.package
          ];

          shellHook = ''
            echo "Hex Viewer written rust"
          '';
        };

        apps.treefmt = {
          type = "app";
          program = "${treefmt.config.package}/bin/treefmt";
        };

        formatter = treefmt.config.build.wrapper;
        checks = {
          formatting = treefmt.config.build.check self;
        };
      }
    );
}
