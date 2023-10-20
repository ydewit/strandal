# save this as shell.nix
{ pkgs ? import <nixpkgs> {}}:

pkgs.mkShell {
  buildInputs = with pkgs; [
    openssl
  ];
  nativeBuildInputs = with pkgs; [
    pkg-config
    pkgs.llvmPackages_latest.lldb
  ];
  packages = [ 
    pkgs.rustup
    pkgs.linuxKernel.packages.linux_6_1.perf
    pkgs.hotspot
    pkgs.rust-cbindgen
  ];
  RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
}
