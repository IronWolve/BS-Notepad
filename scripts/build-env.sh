# Sourced by packagers after ROOT and any project-local toolchain paths are set.
# Encoded flags preserve paths containing spaces and avoid overwriting caller flags.
_notepad_remap_paths() {
  local flags=${CARGO_ENCODED_RUSTFLAGS:-} flag
  local inherited=()
  if [ -z "$flags" ] && [ -n "${RUSTFLAGS:-}" ]; then
    read -r -a inherited <<< "$RUSTFLAGS"
    for flag in "${inherited[@]}"; do flags="${flags:+$flags$'\037'}$flag"; done
  fi
  for flag in \
    "--remap-path-prefix=$HOME=/user" \
    "--remap-path-prefix=$ROOT=/app-source" \
    "--remap-path-prefix=${CARGO_HOME:-$HOME/.cargo}=/cargo" \
    "--remap-path-prefix=${RUSTUP_HOME:-$HOME/.rustup}=/toolchain"; do
    case $'\037'"$flags"$'\037' in
      *$'\037'"$flag"$'\037'*) ;;
      *) flags="${flags:+$flags$'\037'}$flag" ;;
    esac
  done
  export CARGO_ENCODED_RUSTFLAGS="$flags"
}
_notepad_remap_paths
unset -f _notepad_remap_paths

_notepad_verify_release_paths() {
  local binary=$1 prefix
  for prefix in "$ROOT/" "$HOME/"; do
    if LC_ALL=C grep -a -F -q -- "$prefix" "$binary"; then
      echo "Release path audit failed: a development path remains in $binary" >&2
      return 1
    fi
  done
}
