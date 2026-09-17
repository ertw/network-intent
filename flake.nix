{
  description = "Network Intent: native compiler, runtime, and development tools";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/21a67dc470149f337cecafbe965d8d252a390518";
  outputs = { self, nixpkgs }:
    let
      systems = [ "aarch64-darwin" "x86_64-linux" ];
      eachSystem = nixpkgs.lib.genAttrs systems;
      package = system:
        let pkgs = import nixpkgs { inherit system; }; in
        pkgs.stdenv.mkDerivation {
          pname = "network-intent";
          version = "0.3.0";
          # Never reuse host-generated Idris objects, Cargo artifacts, or npm files.
          src = pkgs.lib.cleanSourceWith {
            src = ./.;
            filter = path: type:
              pkgs.lib.cleanSourceFilter path type
              && !(builtins.elem (baseNameOf path) [
                "build" "target" "node_modules" "dist" "test-results" ".DS_Store"
              ]);
          };
          nativeBuildInputs = [ pkgs.idris2 pkgs.python3 pkgs.makeWrapper ]
            ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [ pkgs.zsh ];
          buildPhase = "runHook preBuild; make build; runHook postBuild";
          doCheck = true;
          checkPhase = "runHook preCheck; make test; runHook postCheck";
          installPhase = ''
            runHook preInstall
            mkdir -p $out/libexec/netc $out/bin $out/share/network-intent
            cp -R build/exec/. $out/libexec/netc/
            cp -R docs examples $out/share/network-intent/
            cp scripts/release.py $out/libexec/netc/release.py
            cp netc $out/libexec/netc/launcher
            substituteInPlace $out/libexec/netc/launcher \
              --replace-fail '"$root/build/exec/netc"' '"$root/netc"' \
              --replace-fail '"$root/scripts/release.py"' '"$root/release.py"'
            makeWrapper $out/libexec/netc/launcher $out/bin/netc --prefix PATH : ${pkgs.lib.makeBinPath ([ pkgs.python3 ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [ pkgs.zsh ])}
            runHook postInstall
          '';
          doInstallCheck = true;
          installCheckPhase = ''
            runHook preInstallCheck
            $out/bin/netc schema > schema.json
            $out/bin/netc release-check docs/schema/3.0.json schema.json
            runHook postInstallCheck
          '';
          meta.platforms = systems;
        };
    in {
      packages = eachSystem (system:
        let
          pkgs = import nixpkgs { inherit system; };
          netc = package system;
          runtime = pkgs.rustPlatform.buildRustPackage {
            pname = "network-intent-runtime";
            version = "0.1.0";
            src = netc.src;
            cargoLock.lockFile = ./Cargo.lock;
            nativeBuildInputs = [ pkgs.pkg-config pkgs.cmake ]
              ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [ pkgs.zsh ];
            buildInputs = [ pkgs.openssl ];
            # Admission tests use the repository launcher and compiled Idris tree.
            preCheck = "mkdir -p build/exec; cp -R ${netc}/libexec/netc/. build/exec/";
            meta.platforms = systems;
          };
        in { inherit netc runtime; default = netc; });
      apps = eachSystem (system: {
        default = { type = "app"; program = "${self.packages.${system}.netc}/bin/netc"; };
      });
      checks = eachSystem (system: {
        inherit (self.packages.${system}) netc runtime;
      });
      devShells = eachSystem (system:
        let pkgs = import nixpkgs { inherit system; }; in {
          default = pkgs.mkShell { packages = [
            pkgs.idris2 pkgs.python3 pkgs.gnumake
            pkgs.cargo pkgs.rustc pkgs.rustfmt pkgs.clippy
            pkgs.nodejs_24 pkgs.pkg-config pkgs.cmake pkgs.openssl pkgs.zsh
          ]; };
        });
    };
}
