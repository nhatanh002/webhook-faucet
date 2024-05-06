{
  description = "rust env";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        toml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
        rustVersion = pkgs.rust-bin.nightly.latest.default;
        toolchain = (rustVersion.override { extensions = [ "cargo" "rustc" "rust-std" "rust-src" "rustfmt" "clippy" ]; });
        rustPlatform = pkgs.makeRustPlatform {
          cargo = toolchain;
          rustc = toolchain;
        };

        inherit (pkgs) inotify-tools terminal-notifier;
        inherit (pkgs.lib) optionals;
        inherit (pkgs.stdenv) isDarwin isLinux;

        linuxDeps = optionals isLinux [ inotify-tools ];
        defaultPkg = rustPlatform.buildRustPackage {
          pname =
            toml.package.name;
          version = "v${toml.package.version}";
          src = ./.;

          cargoLock.lockFile = ./Cargo.lock;
          cargoBuildFlags = [
            "--release"
          ];

          buildInputs = with pkgs; [
            openssl
          ];

          nativeBuildInputs = with pkgs; [
            pkg-config
          ];
        };
      in
      {
        packages = { default = defaultPkg; };
        devShells = {
          default = pkgs.mkShell {
            packages = with pkgs;  [
              toolchain
              rust-analyzer-unwrapped
              cargo-watch
              pkg-config
              openssl.dev
              cmake
              zlib
              redis
              python311Packages.supervisor
              docker-compose
              vegeta
            ] ++ linuxDeps;
            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [ pkgs.openssl ];
            shellHook = ''
              mkdir -p .nix-cargo-home
              export CARGO_HOME=$PWD/.nix-cargo-home
              export RUST_SRC_PATH="${toolchain}/lib/rustlib/src/rust/library"
              # export RUSTFLAGS=""
              export PATH=$CARGO_HOME/bin:$PATH
              export LANG=C.UTF-8
            '';
          };
        };
      });
}
