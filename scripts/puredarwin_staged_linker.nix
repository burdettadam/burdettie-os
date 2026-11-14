# Explicit opt-in rebind. Existing native linker and library recipes stay intact.
{ packages, libtapiRecipe, nativeLinkerRecipe, stager }:
let
  p = packages;
  identity = package: { drv = package.drvPath; out = package.outPath; };
  paths = inputs: map toString inputs;
  occurrences = text: value:
    (builtins.stringLength text - builtins.stringLength
      (builtins.replaceStrings [ value ] [ "" ] text)) / builtins.stringLength value;
  tapiPlan = libtapiRecipe { packages = p; inherit stager; };
  matches = builtins.filter (input: (input.pname or "") == "apple-libtapi") p.native-ld.buildInputs;
  originalTapi = assert builtins.length matches == 1; builtins.head matches;
  stagedTapi = originalTapi.overrideAttrs (_: { unpackPhase = tapiPlan.unpack_phase; });
  original = p.native-ld;
  # Preserve the original bootstrap toolchain argument; change only libtapi.
  rebound = original.override { libtapi = stagedTapi; };
  boundPlan = nativeLinkerRecipe (p // { native-ld = rebound; });
  baseline = nativeLinkerRecipe p;
  before = "ninja -C build-nix-native host_ld host_strip";
  after = "ninja -j1 -C build-nix-native host_ld host_strip";
  bounded = rebound.overrideAttrs (old: {
    buildPhase = assert occurrences old.buildPhase before == 1;
      builtins.replaceStrings [ before ] [ after ] old.buildPhase;
  });
  replaceTapi = builtins.replaceStrings [ originalTapi.outPath ] [ stagedTapi.outPath ];
  expectedInputs = map
    (path: if path == originalTapi.outPath then stagedTapi.outPath else path)
    (paths original.buildInputs);
  preserved = {
    source = toString original.src == toString bounded.src;
    native_build_inputs = paths original.nativeBuildInputs == paths bounded.nativeBuildInputs;
    bootstrap_toolchain = original.NIX_DARWIN_TOOLCHAIN_DIR == bounded.NIX_DARWIN_TOOLCHAIN_DIR;
    install_phase = original.installPhase == bounded.installPhase;
    other_configure_flags = bounded.configurePhase == replaceTapi original.configurePhase;
    original_build_before_job_bound = rebound.buildPhase == original.buildPhase;
    other_build_inputs = paths bounded.buildInputs == expectedInputs;
    sdk = boundPlan.sdk == tapiPlan.sdk && boundPlan.sdk == baseline.sdk;
  };
in
assert identity originalTapi == tapiPlan.upstream;
assert identity stagedTapi == tapiPlan.candidate;
assert identity bounded == boundPlan.candidate;
assert occurrences original.configurePhase originalTapi.outPath == 2;
assert builtins.length (builtins.filter (path: path == originalTapi.outPath)
  (paths original.buildInputs)) == 1;
assert builtins.all (value: value) (builtins.attrValues preserved);
assert bounded.drvPath != baseline.candidate.drv && bounded.outPath != baseline.candidate.out;
{
  upstream = identity original;
  previous_bounded = baseline.candidate;
  rebound_unbounded = identity rebound;
  candidate = identity bounded;
  sdk = boundPlan.sdk;
  staged_libtapi = tapiPlan;
  unchanged_recipe_fields = preserved;
  build_inputs = { upstream = paths original.buildInputs; candidate = paths bounded.buildInputs; };
  configure_phase_sha256 = {
    upstream = builtins.hashString "sha256" original.configurePhase;
    candidate = builtins.hashString "sha256" bounded.configurePhase;
  };
  build_phase_sha256 = {
    upstream = builtins.hashString "sha256" original.buildPhase;
    candidate = builtins.hashString "sha256" bounded.buildPhase;
  };
  ninja_patch_sha256 = boundPlan.patch_sha256;
  changed = { libtapi_build_inputs = 1; libtapi_configure_paths = 2; ninja_commands = 1; };
}
