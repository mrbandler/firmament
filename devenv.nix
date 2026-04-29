{ pkgs, ... }:
{
  name = "firmament-dev";
  packages = with pkgs; [
    # Build essentials
    pkg-config
    openssl

    # Development tools
    cargo-deny
    cargo-watch
    just
    mdbook
    prek
    uv
  ];

  languages.rust = {
    enable = true;
    channel = "stable";
    components = [
      "rustc"
      "cargo"
      "clippy"
      "rustfmt"
      "rust-analyzer"
    ];
    targets = [ "wasm32-unknown-unknown" ];
  };

  env = {
    RUST_BACKTRACE = "1";
  };
}
