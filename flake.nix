{
  description = "Realtime Vulkan ray tracing";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable"; 
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay,... }: let
    lib = {
      inherit (flake-utils.lib) defaultSystems eachSystem;
    };
    supportedSystems = [ "x86_64-linux" ];
  in lib.eachSystem supportedSystems (system: let
    nightlyVersion = "2023-01-15";

    #pkgs = mars-std.legacyPackages.${system};
    pkgs = import nixpkgs {
        inherit system;
        overlays = [
          (import rust-overlay)
          #(import ./pkgs)
        ];
      };
    pinnedRust = pkgs.rust-bin.nightly.${nightlyVersion}.default.override {
      extensions = ["rustc-dev" "rust-src" "rust-analyzer-preview" ];
      targets = [ "x86_64-unknown-linux-gnu" ];
    };
    rustPlatform = pkgs.makeRustPlatform {
      rustc = pinnedRust;
      cargo = pinnedRust;
    };
    cargoExpand = pkgs.cargo-expand.override { inherit rustPlatform; };
    cargoFlame = pkgs.cargo-flamegraph.override { inherit rustPlatform; };
    #cargoMSRV = pkgs.cargo-msrv.override { inherit rustPlatform; };
    cargoBloat = pkgs.cargo-bloat.override { inherit rustPlatform; };
    cargoLicense = pkgs.cargo-license.override { inherit rustPlatform; };
    cargo-supply-chain = pkgs.cargo-supply-chain.override { inherit rustPlatform; };
    #cargoREADME = pkgs.cargo-readme.override { inherit rustPlatform; };
    #cargoFeature = pkgs.cargo-feature.override { inherit rustPlatform; };
    #cargoGeiger = pkgs.cargo-geiger.override { inherit rustPlatform; };
  in {
    
devShell = pkgs.mkShell rec {
  hardeningDisable = [
    "fortify"
  ];
  nativeBuildInputs = [
    pinnedRust 
    cargoExpand
    cargoFlame
    cargoBloat
    cargoLicense
    cargo-supply-chain
    #cargoREADME
    #cargoFeature
    #cargoGeiger
    #cargoMSRV
  ];
  buildInputs = with pkgs; [
#    alsaLib
#    binaryen
    fontconfig
#    freetype
#    libxkbcommon
     pkg-config
#    spirv-tools
    #udev

    vulkan-loader
    vulkan-tools
    
    xorg.libX11 
    xorg.libXcursor
    xorg.libXi
    xorg.libXrandr
    shaderc
    #renderdoc
#    gcc-unwrapped.lib
  ];

  VULKAN_SDK = "${pkgs.vulkan-validation-layers}/share/vulkan/explicit_layer.d";
  VK_LAYER_PATH="${pkgs.vulkan-validation-layers}/share/vulkan/explicit_layer.d";

  shellHook = ''
    export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${ pkgs.lib.makeLibraryPath buildInputs}"
  '';
};

  });
}
