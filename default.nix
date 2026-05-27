{ pkgs ? import <nixpkgs> {} }:

pkgs.rustPlatform.buildRustPackage {
  pname = "de-configurator";
  version = "0.1.0";
  src = ./.;

  cargoLock = if builtins.pathExists ./vendor then null else {
    lockFile = ./Cargo.lock;
  };

  cargoVendorDir = if builtins.pathExists ./vendor then ./vendor else null;

  buildInputs = with pkgs; [
    xorg.libX11
    xorg.libXcursor
    xorg.libXrandr
    xorg.libXi
  ];

  nativeBuildInputs = [ pkgs.makeWrapper ];

  postInstall = ''
    wrapProgram $out/bin/de-configurator \
      --prefix LD_LIBRARY_PATH : "${pkgs.lib.makeLibraryPath (with pkgs; [
        xorg.libX11
        xorg.libXcursor
        xorg.libXrandr
        xorg.libXi
      ])}"
  '';
}
