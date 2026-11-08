#!/usr/bin/env bash
# Vendors Elanor, Earendil's design system, into the guide's site (0036, D1360).
#
#   scripts/elanor-update.sh [SHA|main] [BRAND]
#
# Installs Elanor's published packages from pkg.earendil.com, pinned to one full elanor
# commit (crates/psh-docs/elanor/package.json and its lockfile), and copies the files the
# site uses, unchanged, into crates/psh-docs/assets/elanor/. psh-docs copies that into
# the site, so `check` and `build` need no login and nothing expires (builds on
# pkg.earendil.com expire 30 days after upload, even pinned ones).
#
# Without arguments it reinstalls the pinned commit; `main` moves the pin to the newest
# build of elanor's main. BRAND is the token stylesheet (earendil, elanor, pi, radius,
# lefos; default: the one in assets/elanor/VERSION, else earendil).
#
# Needs an Earendil Works login once: npm login --registry=https://pkg.earendil.com
# Afterwards run `cargo test -p psh-docs`: it checks that every token style.css uses
# still exists.
set -euo pipefail
cd "$(dirname "$0")/.."
root=$PWD
pkgdir=crates/psh-docs/elanor
out=crates/psh-docs/assets/elanor
reg=https://pkg.earendil.com/@earendil-works

pinned=$(sed -n 's/^elanor \([0-9a-f]\{40\}\)$/\1/p' "$out/VERSION" 2>/dev/null || true)
brand=$(sed -n 's/^brand \(.*\)$/\1/p' "$out/VERSION" 2>/dev/null || true)
ref=${1:-$pinned}
brand=${2:-${brand:-earendil}}
if [ -z "$ref" ]; then
  echo "elanor-update.sh: no pinned commit yet; pass a SHA or main" >&2
  exit 2
fi
if [ "$ref" = main ] || ! [[ "$ref" =~ ^[0-9a-f]{40}$ ]]; then
  # Resolve a ref (main) to the full commit its build came from: the lockfile must pin a
  # SHA, because a ref's tarball changes with every publish.
  ref=$(curl -sSf "https://api.github.com/repos/earendil-works/elanor/commits/$ref" \
    | sed -n 's/^  "sha": "\([0-9a-f]\{40\}\)",$/\1/p' | head -1)
  [ -n "$ref" ] || { echo "elanor-update.sh: cannot resolve the commit" >&2; exit 1; }
fi
echo "elanor-update.sh: elanor $ref, brand $brand"

packages=(design-tokens elanor-fonts elanor-icons elanor-logos)
(
  cd "$pkgdir"
  rm -rf node_modules package-lock.json
  node -e '
    const fs = require("fs");
    const [sha, base, ...names] = process.argv.slice(1);
    const p = JSON.parse(fs.readFileSync("package.json", "utf8"));
    p.dependencies = Object.fromEntries(names.map((n) => [`@earendil-works/${n}`, `${base}/${n}@${sha}`]));
    fs.writeFileSync("package.json", JSON.stringify(p, null, 2) + "\n");
  ' "$ref" "$reg" "${packages[@]}"
  npm install --no-audit --no-fund --ignore-scripts
)

nm=$pkgdir/node_modules/@earendil-works
tokens=$nm/design-tokens/dist/$brand
[ -f "$tokens/tokens.css" ] || { echo "elanor-update.sh: no brand $brand" >&2; exit 1; }
rm -rf "$out"
mkdir -p "$out/fonts/files" "$out/fonts/licenses" "$out/icons" "$out/logos"

# Design tokens: one brand's stylesheet.
cp "$tokens/tokens.css" "$out/tokens.css"
cp "$nm/design-tokens/LICENSE" "$out/tokens.LICENSE"
# Fonts: Mallos Serif (prose and UI) and Commit Mono (code), with their OFL licenses.
f=$nm/elanor-fonts
cp "$f/mallos-serif.css" "$f/commit-mono.css" "$out/fonts/"
cp "$f"/files/MallosSerif-*.woff2 "$f"/files/CommitMono-*.woff2 "$out/fonts/files/"
cp "$f/licenses/mallos-serif.txt" "$f/licenses/commit-mono.txt" "$out/fonts/licenses/"

# Icons (Material Symbols, Apache-2.0) used by the page template.
i=$nm/elanor-icons
for icon in search dark-mode light-mode menu open-in-new; do cp "$i/dist/svg/$icon.svg" "$out/icons/"; done
cp "$i/LICENSE" "$i/NOTICE" "$out/icons/"

# The Earendil symbol for the header and the favicon (BRAND-GUIDELINES.md: unmodified,
# on-light/on-dark to match the background).
l=$nm/elanor-logos/dist
cp "$l/svg/earendil/earendil-logo-on-light.svg" "$l/svg/earendil/earendil-logo-on-dark.svg" "$out/logos/"
cp "$l/favicons/earendil/favicon.svg" "$out/logos/"
cp "$l/guidelines/earendil/BRAND-GUIDELINES.md" "$out/logos/" 2>/dev/null || true

printf 'elanor %s\nbrand %s\n' "$ref" "$brand" > "$out/VERSION"
echo "elanor-update.sh: wrote $out ($(du -sk "$out" | cut -f1) KB); now run: cargo test -p psh-docs"
