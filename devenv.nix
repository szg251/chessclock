{ pkgs, ... }:

let
  pyocd = import ./nix/pyocd pkgs;
  defmt-print = import ./nix/defmt-print pkgs;
in
{
  packages = [
    pyocd
    defmt-print
  ];

  env.CARGO_BUILD_TARGET = "thumbv7m-none-eabi";

  languages.rust = {
    enable = true;
    channel = "stable";
    targets = [ "thumbv7m-none-eabi" ];
  };

  # https://devenv.sh/pre-commit-hooks/
  pre-commit.hooks = {
    # clippy.enable = true;
    rustfmt.enable = true;
  };

  # See full reference at https://devenv.sh/reference/options/
}
