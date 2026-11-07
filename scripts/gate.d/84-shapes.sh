# timeout: 120
# inputs: tests/shapes/ scripts/shape-corpus-check
# The committed shape corpus holds only vocabulary words and placeholders (0003, D1030).
python3 scripts/shape-corpus-check --self-test
python3 scripts/shape-corpus-check
