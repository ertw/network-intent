{
  description = "Network Intent DSL: Idris 2 compiler and acceptance checks";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/21a67dc470149f337cecafbe965d8d252a390518";
  outputs = { self, nixpkgs }:
    let
      systems = [ "aarch64-darwin" "x86_64-darwin" "aarch64-linux" "x86_64-linux" ];
      eachSystem = nixpkgs.lib.genAttrs systems;
      package = system:
        let pkgs = import nixpkgs { inherit system; }; in
        pkgs.stdenv.mkDerivation {
          pname = "network-intent";
          version = "0.2.0";
          src = pkgs.lib.cleanSource ./.;
          nativeBuildInputs = [ pkgs.idris2 pkgs.python3 pkgs.makeWrapper ];
          buildPhase = "make build";
          doCheck = true;
          checkPhase = "make test";
          installPhase = ''
            mkdir -p $out/libexec/netc $out/bin $out/share/network-intent
            cp -R build/exec/. $out/libexec/netc/
            cp -R docs examples $out/share/network-intent/
            cp scripts/release.py $out/libexec/netc/release.py
            cp netc $out/libexec/netc/launcher
            substituteInPlace $out/libexec/netc/launcher \
              --replace-fail '"$root/build/exec/netc"' '"$root/netc"' \
              --replace-fail '"$root/scripts/release.py"' '"$root/release.py"'
            makeWrapper $out/libexec/netc/launcher $out/bin/netc --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.python3 ]}
          '';
        };
    in {
      packages = eachSystem (system: { netc = package system; default = package system; });
      checks = eachSystem (system: { netc = package system; });
      devShells = eachSystem (system:
        let pkgs = import nixpkgs { inherit system; }; in {
          default = pkgs.mkShell { packages = [ pkgs.idris2 pkgs.python3 pkgs.gnumake ]; };
        });
    };
}
