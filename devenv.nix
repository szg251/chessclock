{ pkgs, ... }:

{
  packages = [
    # pkgs.pkgsCross.raspberryPi.stdenv.cc
    # pkgs.probe-rs-tools
    pkgs.elf2uf2-rs
    pkgs.flip-link
  ];

  env = {
    CARGO_BUILD_TARGET = "thumbv6m-none-eabi";
    # CARGO_TARGET_THUMBV6_NONE_EABI_LINKER =
    #   let
    #     inherit (pkgs.pkgsCross.raspberryPi.stdenv) cc;
    #   in
    #   "${cc}/bin/${cc.targetPrefix}cc";
  };

  languages.rust = {
    enable = true;
    channel = "stable";
    targets = [ "thumbv6m-none-eabi" ];
  };

  # https://devenv.sh/pre-commit-hooks/
  pre-commit.hooks = {
    # clippy.enable = true;
    rustfmt.enable = true;
  };

  # See full reference at https://devenv.sh/reference/options/
}
