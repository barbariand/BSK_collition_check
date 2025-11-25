{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";
  };
  outputs = {
    flake-utils,
    nixpkgs,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {inherit system;};
    in {
      devShells.default = with pkgs;
        mkShell {
          nativeBuildInput = with pkgs; [openssl];
          buildInputs = with pkgs; [
            openssl
          ];
          packages = with pkgs; [pkg-config];
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [pkgs.openssl];
          PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
          shellHook = ''
          '';
        };
    });
}
