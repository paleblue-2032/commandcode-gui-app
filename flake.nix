{
  description = "Command Code GUI";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
  };

  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };

      runtimeLibs = with pkgs; [
        libxkbcommon
        wayland
        libGL
        vulkan-loader
        libX11
        libXcursor
        libXrandr
        libXi
        libxcb
        xkeyboard-config
      ];

      cjkFont =
        "${pkgs.noto-fonts-cjk-sans}/share/fonts/opentype/noto-cjk/NotoSansCJK-VF.otf.ttc";
    in
    {
      packages.${system}.default = pkgs.rustPlatform.buildRustPackage {
        pname = "commandcode-gui-app";
        version = "0.1.0";

        src = ./.;
        cargoLock.lockFile = ./Cargo.lock;

        nativeBuildInputs = with pkgs; [
          pkg-config
          makeWrapper
          cmake
        ];
        buildInputs = runtimeLibs ++ [ pkgs.noto-fonts-cjk-sans ];

        postInstall = ''
          wrapProgram "$out/bin/commandcode-gui-app" \
            --prefix LD_LIBRARY_PATH : "${pkgs.lib.makeLibraryPath runtimeLibs}" \
            --set-default COMMANDCODE_GUI_CJK_FONT "${cjkFont}"
        '';

        meta.mainProgram = "commandcode-gui-app";
      };

      devShells.${system}.default = pkgs.mkShell {
        packages = with pkgs; [
          rustc
          cargo
          rustfmt
          clippy
          pkg-config
          cmake
          ninja
          fontconfig
          noto-fonts-cjk-sans
        ];

        LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath runtimeLibs;
        COMMANDCODE_GUI_CJK_FONT = cjkFont;
      };
    };
}
