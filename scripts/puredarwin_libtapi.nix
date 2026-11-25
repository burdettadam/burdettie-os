# Opt-in source staging only. Apply to the exact pinned flake's package set.
{ packages, stager }:
let
  p = packages;
  matches = builtins.filter (input: (input.pname or "") == "apple-libtapi") p.native-ld.buildInputs;
  upstream = assert builtins.length matches == 1; builtins.head matches;
  sdk = p.apple-sdk;
  unpack = "runHook preUnpack\n"
    + "python3 - --source \"$src\" --destination source --manifest libtapi-staging.json <<'BURDETTIE_LIBTAPI_STAGE_PY'\n"
    + stager + "\nBURDETTIE_LIBTAPI_STAGE_PY\nsourceRoot=source\nrunHook postUnpack\n";
  candidate = upstream.overrideAttrs (_: { unpackPhase = unpack; });
  identity = package: { drv = package.drvPath; out = package.outPath; };
  paths = values: map toString values;
  unchanged = {
    source = toString upstream.src == toString candidate.src;
    patches = paths upstream.patches == paths candidate.patches;
    post_patch = upstream.postPatch == candidate.postPatch;
    build_phase = upstream.buildPhase == candidate.buildPhase;
    native_build_inputs = paths upstream.nativeBuildInputs == paths candidate.nativeBuildInputs;
    build_inputs = paths (upstream.buildInputs or []) == paths (candidate.buildInputs or []);
    configure_phase = (upstream.configurePhase or null) == (candidate.configurePhase or null);
    install_phase = (upstream.installPhase or null) == (candidate.installPhase or null);
    dont_install = upstream.dontInstall == candidate.dontInstall;
    cmake_driver = upstream.dontUseCmakeConfigure == candidate.dontUseCmakeConfigure;
    pre_unpack = (upstream.preUnpack or null) == (candidate.preUnpack or null);
    post_unpack = (upstream.postUnpack or null) == (candidate.postUnpack or null);
  };
in
assert upstream.version == "1300.6.5";
assert upstream.src.rev == "640b4623929c923c0468143ff2a363a48665fa54";
assert upstream.src.outputHash == "sha256-eXzQRB07AcH9nBgPoGpIQZbC4O/bIzkdbjJ442qM9VA=";
assert !(upstream ? unpackPhase);
assert builtins.any (input: (input.pname or "") == "python3") upstream.nativeBuildInputs;
assert builtins.all (value: value) (builtins.attrValues unchanged);
assert upstream.drvPath != candidate.drvPath && upstream.outPath != candidate.outPath;
{
  upstream = identity upstream;
  candidate = identity candidate;
  sdk = identity sdk;
  full_libtapi_source = (identity upstream.src) // {
    rev = upstream.src.rev;
    output_hash = upstream.src.outputHash;
    retained_unchanged = true;
  };
  unchanged_recipe_fields = unchanged;
  upstream_patch_files = map (path: {
    source_path = toString path;
    sha256 = builtins.hashFile "sha256" path;
  }) upstream.patches;
  upstream_build_phase_sha256 = builtins.hashString "sha256" upstream.buildPhase;
  upstream_post_patch_sha256 = builtins.hashString "sha256" upstream.postPatch;
  stager_sha256 = builtins.hashString "sha256" stager;
  unpack_phase_sha256 = builtins.hashString "sha256" unpack;
  unpack_phase = unpack;
}
