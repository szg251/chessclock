{ pkgs, ... }:
with pkgs;
rustPlatform.buildRustPackage rec {
  pname = "defmt-print";
  version = "0.3.13";

  src = fetchFromGitHub {
    owner = "knurling-rs";
    repo = "defmt";
    rev = "defmt-print-v${version}";
    hash = "sha256-wcT2PaqlDKm9LvmPZvygDTiFJXnYuIWzf+G0PZ8vplc=";
  };
  cargoLock = {
    lockFile = ./Cargo.lock;
  };
  postPatch = ''
    ln -s ${./Cargo.lock} Cargo.lock
  '';

  useFetchCargoVendor = true;

  nativeBuildInputs = [ pkg-config ];

  buildInputs =
    [
      openssl
      zlib
    ]
    ++ lib.optionals stdenv.hostPlatform.isDarwin [
      darwin.Security
    ];

  doCheck = false; # integration tests depend on changing cargo config
}
