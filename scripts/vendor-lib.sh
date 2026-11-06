# Sourced by the scripts/fetch-* scripts: export a pinned upstream subtree into tests/.
#
#   vendor_sync NAME URL COMMIT DEST "PATHS..."   [--check]
#
# The upstream clone lives in vendor/NAME of the primary checkout (shared by all
# worktrees, gitignored); PSH_VENDOR_SRC points at another clone instead. Files come from
# `git archive COMMIT`, so local edits in the clone never leak into tests/. Without
# --check, DEST's copies of PATHS are replaced; with --check, they are compared and any
# difference fails.
vendor_sync() {
  local name=$1 url=$2 commit=$3 dest=$4 paths=$5 mode=${6:-} root primary src tmp p
  root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
  primary=$(git -C "$root" worktree list --porcelain | awk '/^worktree /{ sub(/^worktree /, ""); print; exit }')
  src=${PSH_VENDOR_SRC:-$primary/vendor/$name}
  if [ ! -d "$src/.git" ] && [ ! -f "$src/.git" ]; then
    echo "== cloning $url into $src"
    git clone -q "$url" "$src"
  fi
  if ! git -C "$src" cat-file -e "$commit^{commit}" 2>/dev/null; then
    git -C "$src" fetch -q origin
  fi
  tmp=$(mktemp -d "${TMPDIR:-/tmp}/psh-vendor.XXXXXX")
  # shellcheck disable=SC2086
  git -C "$src" archive "$commit" $paths | tar -x -C "$tmp"
  if [ "$mode" = --check ]; then
    local bad=0
    for p in $paths; do
      if ! diff -r "$tmp/$p" "$root/$dest/$p" >/dev/null 2>&1; then
        echo "vendor: $dest/$p differs from $name@$commit" >&2
        diff -r "$tmp/$p" "$root/$dest/$p" 2>&1 | head -20 >&2
        bad=1
      fi
    done
    rm -rf "$tmp"
    [ $bad = 0 ] && echo "vendor: $dest matches $name@${commit:0:12}"
    return $bad
  fi
  for p in $paths; do
    rm -rf "${root:?}/$dest/$p"
    mkdir -p "$(dirname "$root/$dest/$p")"
    cp -R "$tmp/$p" "$root/$dest/$p"
  done
  rm -rf "$tmp"
  echo "vendor: $dest updated from $name@${commit:0:12}"
}
