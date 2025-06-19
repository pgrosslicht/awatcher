{ pkgs, lib, config, inputs, ... }:

{
  packages = [
    pkgs.git
    pkgs.libiconv
    pkgs.openssl
    pkgs.pkg-config
    pkgs.libxkbcommon
  ];

  languages.rust = {
    enable = true;
    components = [ "rustc" "cargo" "clippy" "rustfmt" "rust-analyzer" ];
  };

  enterShell = ''
    echo "Rust version: $(rustc --version)"
    echo "Cargo version: $(cargo --version)"
    echo "RUST_SRC_PATH: $RUST_SRC_PATH"
  '';
}