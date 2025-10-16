{ pkgs, ... }:
with pkgs.python312Packages;
buildPythonPackage rec {
  pname = "pyocd";
  version = "0.36.0";
  pyproject = true;

  src = fetchPypi {
    inherit pname version;
    hash = "sha256-k3eCrMna/wVNUPt8b3iM2UqE+A8LhfJarKuZ3Jgihkg=";
  };

  patches = [
    # https://github.com/pyocd/pyOCD/pull/1332
    (pkgs.fetchpatch {
      name = "libusb-package-optional.patch";
      url = "https://github.com/pyocd/pyOCD/commit/0b980cf253e3714dd2eaf0bddeb7172d14089649.patch";
      hash = "sha256-B2+50VntcQELeakJbCeJdgI1iBU+h2NkXqba+LRYa/0=";
    })
    # https://github.com/pyocd/pyOCD/pull/1680
    (pkgs.fetchpatch {
      name = "rtt-allow-no-down-channels.patch";
      url = "https://github.com/pyocd/pyOCD/commit/209f3dd691ea993a84fd22c543fe3b916b4aab06.patch";
      hash = "sha256-Ol//vqMadtNvtvD+xaT5OvPu307qBi/l83mgTSexPh4=";
    })
  ];

  pythonRemoveDeps = [ "libusb-package" ];

  build-system = [ setuptools-scm ];

  dependencies = [
    capstone_4
    cmsis-pack-manager
    colorama
    importlib-metadata
    importlib-resources
    intelhex
    intervaltree
    lark
    natsort
    prettytable
    pyelftools
    pylink-square
    pyusb
    pyyaml
    typing-extensions
  ] ++ lib.optionals (!stdenv.hostPlatform.isLinux) [ hidapi ];

  doCheck = false;
}
