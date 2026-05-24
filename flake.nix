{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      nixpkgs,
      flake-utils,
      fenix,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        rustToolchain = fenix.packages.${system}.latest.toolchain;
        devToolchain = fenix.packages.${system}.combine [
          rustToolchain
          fenix.packages.${system}.latest.rust-analyzer
        ];
        runtimeLibs = with pkgs; [
          libxkbcommon
          sqlite
          vulkan-loader
          wayland
        ];
      in
      with pkgs;
      {
        devShells.default = mkShell {
          packages = [
            bacon
            devToolchain
          ];
          nativeBuildInputs = [
            pkg-config
          ];
          buildInputs = runtimeLibs;
          shellHook = ''
            export LD_LIBRARY_PATH="${lib.makeLibraryPath runtimeLibs}''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
          '';
        };

        packages.default =
          (makeRustPlatform {
            cargo = rustToolchain;
            rustc = rustToolchain;
          }).buildRustPackage
            {
              pname = "reochat";
              version = "0.1.0";
              src = ./.;

              cargoLock.lockFile = ./Cargo.lock;

              nativeBuildInputs = [
                pkg-config
                cmake
                makeWrapper
              ];

              buildInputs = runtimeLibs;

              postFixup = ''
                wrapProgram $out/bin/reochat \
                  --prefix LD_LIBRARY_PATH : ${lib.makeLibraryPath runtimeLibs}
              '';
            };
      }
    );
}
