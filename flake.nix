{
  description = "huginn: yet another fetch tool written in Rust";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    rust-overlay,
    flake-utils,
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        overlays = [(import rust-overlay)];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
      in {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "huginn";
          version = "0.1.0";
          src = ./.;
          cargoLock = {
            lockFile = ./Cargo.lock;
          };
          nativeBuildInputs = with pkgs; [
            pkg-config
          ];
          buildInputs = with pkgs; [
            librsvg
            libsixel
          ];
        };

        apps.default = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/huginn";
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            (rust-bin.stable.latest.default.override {
              extensions = ["rust-src" "rust-analyzer"];
            })
            rustfmt
            pkg-config
            imagemagick
            librsvg
            libsixel
          ];

          shellHook = ''
            mkdir -p $HOME/.local/share/huginn/logos

            if [ -d "${self}/logos" ]; then
               echo "Installing distro logos..."
               cp -r ${self}/logos/* $HOME/.local/share/huginn/logos/ 2>/dev/null || true
            fi

            echo "Huginn dev environment ready!"
          '';
        };
      }
    );
}
