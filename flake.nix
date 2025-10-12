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
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            (rust-bin.stable.latest.default.override {
              extensions = ["rust-src" "rust-analyzer"];
            })
            rustfmt
            imagemagick
            librsvg
            libsixel
          ];

          shellHook = ''
            mkdir -p $HOME/.config/huginn/logos

            if [ -d "${self}/logos" ]; then
               echo "Installing distro logos..."
               cp -r ${self}/logos/* $HOME/.config/huginn/logos/
            fi
          '';
        };
      }
    );
}
