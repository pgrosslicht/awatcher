{
  description = "Awatcher - Activity and idle watchers with COSMIC support";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        packages = {
          default = self.packages.${system}.awatcher;

          # Development version with COSMIC support
          awatcher = pkgs.rustPlatform.buildRustPackage {
            pname = "awatcher";
            version = "0.3.2-alpha.3-cosmic";

            src = ./.;

            nativeBuildInputs = with pkgs; [ pkg-config ];
            buildInputs = with pkgs; [ 
              openssl 
              libxkbcommon
            ];
            doCheck = false;

            cargoLock = {
              lockFile = ./Cargo.lock;
              outputHashes = {
                "aw-client-rust-0.1.0" = "sha256-QRJL2yzSkpC76qwJzQ4gVGb9MUSfiUge6yW7/SUBLvY=";
                "cosmic-client-toolkit-0.1.0" = "sha256-rzLust1BKbITEgN7Hwjy1CT+4iOipv+4VIixfUAuCms=";
              };
            };

            meta = with pkgs.lib; {
              description = "Activity and idle watchers with COSMIC support";
              longDescription = ''
                Awatcher is a window activity and idle watcher with an optional tray and UI for statistics. The goal is to compensate
                the fragmentation of desktop environments on Linux by supporting all reportable environments, to add more
                flexibility to reports with filters, and to have better UX with the distribution by a single executable.
                
                This version includes support for the COSMIC desktop environment.
              '';
              downloadPage = "https://github.com/2e3s/awatcher/releases";
              homepage = "https://github.com/2e3s/awatcher";
              license = licenses.mpl20;
              mainProgram = "awatcher";
              maintainers = [ maintainers.aikooo7 ];
              platforms = platforms.linux;
            };
          };

        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            # Rust toolchain
            cargo
            rustc
            rust-analyzer
            clippy
            rustfmt

            # Build dependencies
            pkg-config
            openssl
            libxkbcommon

            # Development tools
            git
          ];

          shellHook = ''
            echo "Awatcher development environment"
            echo "Rust version: $(rustc --version)"
            echo "Cargo version: $(cargo --version)"
            echo ""
            echo "Available commands:"
            echo "  cargo build          - Build the project"
            echo "  cargo run            - Run awatcher"
            echo "  cargo test           - Run tests"
            echo "  cargo clippy         - Run linter"
            echo "  cargo fmt            - Format code"
            echo ""
            echo "Build with Nix:"
            echo "  nix build            - Build with Nix"
          '';
        };

        # For backwards compatibility
        defaultPackage = self.packages.${system}.default;
        devShell = self.devShells.${system}.default;
      }
    );
}
