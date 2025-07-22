{ pkgs, ... }:
let
  fs = pkgs.lib.fileset;
  srcFiles = fs.unions [
    ../src
    ../Cargo.lock
    ../Cargo.toml
  ];
in
pkgs.rustPlatform.buildRustPackage {
  name = "one-block-army";
  src = fs.toSource {
    root = ../.;
    fileset = srcFiles;
  };
  cargoLock = {
    lockFile = ../Cargo.lock;
  };

  nativeBuildInputs = [ pkgs.pkg-config ];
  PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";

  meta = {
    mainProgram = "one-block-army";
  };
}
