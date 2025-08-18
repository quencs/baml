let
  # Pin nixpkgs to the exact version from flake.lock
  nixpkgs = builtins.fetchTarball {
    url = "https://github.com/NixOS/nixpkgs/archive/b134951a4c9f3c995fd7be05f3243f8ecd65d798.tar.gz";
    sha256 = "sha256-OnSAY7XDSx7CtDoqNh8jwVwh4xNL/2HaJxGjryLWzX8=";
  };
  
  # Pin nixpkgs-unstable to the exact version from flake.lock
  nixpkgs-unstable = builtins.fetchTarball {
    url = "https://github.com/NixOS/nixpkgs/archive/8a2f738d9d1f1d986b5a4cd2fd2061a7127237d7.tar.gz";
    sha256 = "sha256-sPwcCYuiEopaafePqlG826tBhctuJsLx/mhKKM5Fmjo=";
  };
  
  # Pin fenix to the exact version from flake.lock
  fenixSrc = builtins.fetchTarball {
    url = "https://github.com/nix-community/fenix/archive/c36306dbcc4ad8128e659ea072ad35e02936b03e.tar.gz";
    sha256 = "sha256-zgHp8rxIbJFeF2DuEMAhKqfdUnclcjaVfdhLNgX5nUM=";
  };
  
  pkgs = import nixpkgs {};
  pkgs-unstable = import nixpkgs-unstable {};
  fenix = import fenixSrc { };
  
  pythonEnv = pkgs.python39.withPackages (ps: []);
  
  toolchain = with fenix; combine [
    complete.cargo
    complete.clippy
    complete.rustc
    complete.rust-std
    complete.rustfmt
    complete.rust-analyzer
    targets.wasm32-unknown-unknown.latest.rust-std
    targets.x86_64-unknown-linux-musl.latest.rust-std
  ];

  appleDeps = with pkgs.darwin.apple_sdk.frameworks; [
    CoreServices
    System
    SystemConfiguration
    pkgs.libiconv-darwin
  ];

in
pkgs.mkShell {
  buildInputs = (with pkgs; [
    cmake
    git
    go
    gotools
    openssl
    pkg-config
    lld_17
    pythonEnv
    ruby
    ruby.devEnv
    maturin
    pnpm
    protoc-gen-go
    vsce # VSCode extension packaging tool
    toolchain
    nodejs
    nodePackages.typescript
    pkgs-unstable.uv
    pkgs-unstable.flatbuffers
    wasm-pack
    gcc
    napi-rs-cli
    wasm-bindgen-cli

    # For building the typescript client.
    pixman
    cairo
    pango
    libjpeg
    giflib
    librsvg
  ]) ++ (if pkgs.stdenv.isDarwin then appleDeps else []);

  shellHook = ''
    export PATH="${pkgs.llvmPackages_17.clang}/bin:$PATH"
    export RUST_SRC_PATH=${pkgs.rustPlatform.rustLibSrc}
    export LIBCLANG_PATH=${pkgs.libclang.lib}/lib/
    export UV_PYTHON="${pythonEnv}/bin/python3"
    export BINDGEN_EXTRA_CLANG_ARGS="${if pkgs.stdenv.isDarwin then
      "" # Rely on default includes provided by stdenv.cc + libclang
    else
      "-isystem ${pkgs.llvmPackages_17.libclang.lib}/lib/clang/17/include -isystem ${pkgs.llvmPackages_17.libclang.lib}/include -isystem ${pkgs.glibc.dev}/include"
    }"
  '';
}