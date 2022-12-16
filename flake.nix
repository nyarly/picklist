{
  description = "Picklist - a druid based dmenu-alike";
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/22.11";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs = { self, nixpkgs, flake-utils, fenix }:
  flake-utils.lib.eachDefaultSystem (system:
  let
    pkgs = (import "${nixpkgs}" {
      inherit system;
    });
    fe = fenix.packages.${system}.minimal;
  in {
    devShells.default = pkgs.mkShell {
      buildInputs = with pkgs; with fe; [
        cargo
        rustc
        rust-analyzer
        pkg-config
        glib
        cairo
        pango
        gdk-pixbuf
        atk
        gtk3
        inferno
      ];
    };
  });
}
