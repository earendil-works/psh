# timeout: 60
# inputs: scripts/
# Every script parses (they run under macOS's bash 3.2 too): the .sh files and the
# extensionless bash scripts in scripts/.
for f in scripts/*.sh scripts/gate.d/*.sh; do bash -n "$f"; done
for f in scripts/*; do
  [ -f "$f" ] || continue
  case "$f" in *.*) continue ;; esac
  if head -1 "$f" | grep -q 'bash'; then bash -n "$f"; fi
done
