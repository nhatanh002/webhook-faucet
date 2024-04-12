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
            toml.package.name; # make this what ever your cargo.toml package.name is
          version = "v${toml.package.version}";
          src = ./.; # the folder with the cargo.toml

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
              python311Packages.supervisor
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
            #LIBCLANG_PATH = pkgs.lib.makeLibraryPath [ pkgs.llvmPackages_latest.libclang.lib ];
            #shellHook = ''
            #  export PATH=$PATH:''${CARGO_HOME:-~/.cargo}/bin
            #  export PATH=$PATH:''${RUSTUP_HOME:-~/.rustup}/toolchains/$RUSTC_VERSION-x86_64-unknown-linux-gnu/bin/
            #  '';
            ## Add precompiled library to rustc search path
            #RUSTFLAGS = (builtins.map (a: ''-L ${a}/lib'') [
            #  # add libraries here (e.g. pkgs.libvmi)
            #]);
            ## Add glibc, clang, glib and other headers to bindgen search path
            #BINDGEN_EXTRA_CLANG_ARGS = 
            ## Includes with normal include path
            #(builtins.map (a: ''-I"${a}/include"'') [
            #  # add dev libraries here (e.g. pkgs.libvmi.dev)
            #  pkgs.glibc.dev 
            #])
            ## Includes with special directory paths
            #++ [
            #  ''-I"${pkgs.llvmPackages_latest.libclang.lib}/lib/clang/${pkgs.llvmPackages_latest.libclang.version}/include"''
            #  ''-I"${pkgs.glib.dev}/include/glib-2.0"''
            #  ''-I${pkgs.glib.out}/lib/glib-2.0/include/''
            #];
          };
        };
      });
}
