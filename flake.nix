{
  description = "hyper minimal acme-dns replacement.";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    crate2nix = {
      url = "github:kolloch/crate2nix";
      flake = false;
    };
    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, utils, rust-overlay, crate2nix, ... }:
    let name = "acmecrab";
    in utils.lib.eachSystem [ utils.lib.system.x86_64-linux ] (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [
            rust-overlay.overlays.default
            (self: super: {
              rustc = self.rust-bin.stable.latest.default;
              cargo = self.rust-bin.stable.latest.default;
            })
          ];
        };
        inherit (import "${crate2nix}/tools.nix" { inherit pkgs; })
          generatedCargoNix;

        project = pkgs.callPackage (generatedCargoNix {
          inherit name;
          src = ./.;
        }) {
          # Individual crate overrides go here
          # Example: https://github.com/balsoft/simple-osd-daemons/blob/6f85144934c0c1382c7a4d3a2bbb80106776e270/flake.nix#L28-L50
          defaultCrateOverrides = pkgs.defaultCrateOverrides // {
            ${name} = oldAttrs: { inherit nativeBuildInputs; };
          };
        };

        rust-toolchain = pkgs.symlinkJoin {
          name = "rust-toolchain";
          paths = with pkgs; [
            rustc
            cargo
            cargo-audit
            clippy
            rustfmt
            rust.packages.stable.rustPlatform.rustLibSrc
          ];
        };

        nativeBuildInputs = with pkgs; [
          rust-toolchain
          gcc
          gdb
          pkgconfig
          grcov
        ];
      in rec {
        packages.${name} = project.rootCrate.build;

        # `nix build`
        packages.default = packages.${name};

        # `nix run`
        apps.${name} = utils.lib.mkApp {
          inherit name;
          drv = packages.${name};
        };
        apps.default = apps.${name};

        # `nix develop`
        devShells.default = pkgs.mkShell {
          inherit nativeBuildInputs;
          RUST_SRC_PATH =
            "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
          RUST_TOOLCHAIN_PATH = "${rust-toolchain}";
        };
      }) // {
        # Nix OS module.
        nixosModules.default = import ./module/default.nix;
        # Example container.
        # `sudo nixos-container create acmecrab --flake .#container`
        # `sudo nixos-container start acmecrab`
        # `dig @10.233.1.2 pki.example.com A`
        nixosConfigurations.container = let
          # NOTE(XXX): These values must match the output from `nixos-container create`.
          host_ip = "10.233.1.1";
          guest_ip = "10.233.1.2";
        in nixpkgs.lib.nixosSystem {
          system = "x86_64-linux";
          modules = [
            self.nixosModules.default
            ({ pkgs, ... }: {
              nixpkgs.overlays = [
                (self': super: {
                  acmecrab = self.packages.${pkgs.system}.default;
                })
              ];
              system.stateVersion = "22.11";
              boot.isContainer = true;
              networking.hostName = "acmecrab";
              services.acmecrab = {
                enable = true;
                domain = "pki.example.com";
                ns_admin = "pki@example.com";
                ns_domain = "ns1.pki.example.com";
                api_addr = guest_ip;
                acl = { "${host_ip}/32" = [ "test" ]; };
                addrs = {
                  "pki.example.com" = [ guest_ip ];
                  "ns1.pki.example.com" = [ guest_ip ];
                };
                ns_records = { "pki.example.com" = [ "ns1.pki.example.com" ]; };
              };
            })
          ];
        };
      };
}
