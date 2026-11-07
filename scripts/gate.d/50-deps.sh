# timeout: 300
# inputs: crates/ Cargo.toml Cargo.lock .cargo/ scripts/deps-allowlist.txt
# Dependency sets (D006, D1223): every dependency of every workspace crate, on every
# target, is listed in scripts/deps-allowlist.txt with the decision that allows it, and
# the list holds nothing else.
list=scripts/deps-allowlist.txt
tmp=$(mktemp -d "${TMPDIR:-/tmp}/psh-deps.XXXXXX")
trap 'rm -rf "$tmp"' EXIT
bad=0

# Each line: crate, dependency, and a citation with a decision id.
grep -v '^#' "$list" | grep -v '^[[:space:]]*$' >"$tmp/lines" || true
while read -r crate dep why; do
  case "$why" in
    *D[0-9]*) ;;
    *)
      echo "gate: $list: '$crate $dep' cites no decision" >&2
      bad=1
      ;;
  esac
done <"$tmp/lines"
awk '{ print $1, $2 }' "$tmp/lines" | sort >"$tmp/allowed"

# What the workspace actually depends on.
: >"$tmp/actual"
for manifest in crates/*/Cargo.toml; do
  crate=$(sed -n 's/^name = "\(.*\)"/\1/p' "$manifest" | head -1)
  cargo tree -p "$crate" -e normal,build,dev --target all --prefix none --format '{p}' --locked |
    awk -v c="$crate" 'NR > 1 { print c, $1 }' >>"$tmp/actual"
done
sort -u "$tmp/actual" -o "$tmp/actual"

missing=$(comm -23 "$tmp/actual" "$tmp/allowed")
stale=$(comm -13 "$tmp/actual" "$tmp/allowed")
if [ -n "$missing" ]; then
  echo "gate: dependencies not in $list (a new dependency needs a decision first):" >&2
  echo "$missing" | sed 's/^/  /' >&2
  bad=1
fi
if [ -n "$stale" ]; then
  echo "gate: $list lists dependencies the workspace does not have (remove them):" >&2
  echo "$stale" | sed 's/^/  /' >&2
  bad=1
fi
[ $bad = 0 ] && echo "deps: $(wc -l <"$tmp/actual" | tr -d ' ') dependency pairs, all allowed"
exit $bad
