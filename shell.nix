{ pkgs ? import <nixpkgs> {} }:

with pkgs;
mkShell {
  nativeBuildInputs = [
    gcc
    nodejs-16_x
    openssl
    pkg-config
  ];
}
