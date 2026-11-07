# timeout: 300
# inputs: crates/ Cargo.toml Cargo.lock .cargo/
# Dependency sets (D006, D1013): a new dependency changes this list along with its
# decision in docs/decisions.md.
#
# check_deps CRATE "ALLOWED...": the crate's normal, build and dev dependencies, direct
# and transitive, are exactly the allowed ones.
check_deps() {
  local got
  got=$(cargo tree -p "$1" -e normal,build,dev --prefix none --format '{p}' --locked |
    awk 'NR > 1 { print $1 }' | sort -u | tr '\n' ' ' | sed 's/ $//')
  if [ "$got" != "$2" ]; then
    echo "gate: $1 depends on '$got'; allowed: '$2' (a new dependency needs a decision)" >&2
    return 1
  fi
}
check_deps psh-syntax ""
check_deps psh-engine "psh-syntax"
check_deps psh-classify "psh-syntax"
check_deps psh-spec "psh-syntax"
