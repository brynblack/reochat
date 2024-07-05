{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };
      in
      {
        devShells.default = with pkgs; mkShell rec {
          nativeBuildInputs = [
            pkg-config
            openssl
            sqlite
          ];
          buildInputs = [
            libxkbcommon
            libGL
            wayland
            rustup
          ];
          LD_LIBRARY_PATH = "${lib.makeLibraryPath buildInputs}";
        };
      });
}
