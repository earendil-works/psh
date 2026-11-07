# timeout: 300
# inputs: scripts/
bash scripts/test-timeout.sh
bash scripts/test-merge-to-main.sh
bash scripts/test-gate-cache.sh
