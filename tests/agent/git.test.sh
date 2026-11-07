# Agent shape: git as agents use it (the most frequent single utility in the corpus):
# `git status --short` (`--branch`), `git diff` (`-- path…`, `--stat`, `--check`,
# `--cached`, piped into `sed -n`), `git log -n N --pretty=…`/`--oneline`,
# `git show --name-only --format= REV`, `git show REV:path`, `git add … && git commit -m`,
# `git rev-parse`, `git branch --show-current`, `git -C dir`, `git ls-files`,
# `git grep -n`, stash, checkout/switch/restore, push/rebase against a local remote, and
# the status/diff chains (`git status --short && git diff --stat && git diff --check`),
# also after `cd dir &&`.

#### status short on a clean repo
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
git status --short
echo "status=$?"

#### status short with every kind of change
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf 'Usage: demo run\n' >> README.md
rm tests/basic.rs
printf 'pub mod util;\n' > src/util.rs
git add src/util.rs
printf 'scratch\n' > notes.txt
git status --short

#### status short branch before the first commit
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
printf 'x\n' > a.txt
git status --short --branch

#### status short branch on main
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf 'Usage: demo run\n' >> README.md
git status --short --branch

#### status short branch ahead of upstream
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q --bare -b main remote.git
git init -q -b main repo && cd repo
printf 'one\n' > a.txt && git add a.txt && git commit -qm "first"
git remote add origin ../remote.git && git push -q -u origin main 2>/dev/null
printf 'two\n' >> a.txt && git commit -qam "second"
git status --short --branch

#### status short from a subdirectory shows relative paths
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf 'Usage: demo run\n' >> README.md
printf '// note\n' >> src/lib.rs
cd src && git status --short

#### status short shows a staged rename
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
git mv tests/basic.rs tests/smoke.rs
git status --short

#### status porcelain with a gitignore
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf 'target/\n*.log\n' > .gitignore
mkdir -p target && touch target/out.bin build.log
git status --porcelain

#### diff of one path
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n\npub fn sub(a: i32, b: i32) -> i32 {\n    a - b\n}\n' > src/lib.rs
git diff -- src/lib.rs

#### diff of one path piped into sed -n
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a.wrapping_add(b)\n}\n' > src/lib.rs
git diff -- src/lib.rs | sed -n '1,8p'

#### diff of two paths
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf 'Usage: demo run\n' >> README.md
printf 'fn main() {\n    println!("hello");\n}\n' > src/main.rs
git diff -- README.md src/main.rs

#### diff of three paths where one is unchanged
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf 'Usage: demo run\n' >> README.md
printf 'use demo::add;\nuse demo::sub;\n' > tests/basic.rs
git diff -- README.md src/lib.rs tests/basic.rs

#### diff of four paths piped into sed -n
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf 'Usage: demo run\n' >> README.md
printf '// end\n' >> src/main.rs
printf '// end\n' >> src/lib.rs
printf '// end\n' >> tests/basic.rs
git diff -- README.md src/main.rs src/lib.rs tests/basic.rs | sed -n '/^diff --git/p'

#### diff of two paths piped into sed -n range
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf 'Usage: demo run\n' >> README.md
printf '// end\n' >> src/lib.rs
git diff -- README.md src/lib.rs | sed -n '5,12p'

#### diff of many paths with a glob pathspec
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf '// a\n' >> src/main.rs
printf '// b\n' >> src/lib.rs
printf '// c\n' >> tests/basic.rs
git diff -- 'src/*.rs' README.md tests/basic.rs | grep '^+[^+]'

#### diff of a path that does not exist
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
git diff -- no/such/file.rs
echo "status=$?"

#### diff stat
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n\npub fn sub(a: i32, b: i32) -> i32 {\n    a - b\n}\n' > src/lib.rs
printf 'Usage: demo run\n' >> README.md
rm tests/basic.rs
git diff --stat

#### diff check on a clean change
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf '// fine\n' >> src/main.rs
git diff --check
echo "status=$?"

#### diff check finds trailing whitespace
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf 'let x = 1;   \n' >> src/main.rs
printf 'Usage:\t\n' >> README.md
git diff --check
echo "status=$?"

#### diff check failure stops the chain
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf 'trailing \n' >> README.md
git diff --check && git status --short
echo "status=$?"

#### diff check then status short
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf 'Usage: demo run\n' >> README.md
git diff --check && git status --short

#### status short then diff stat
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf 'Usage: demo run\n' >> README.md
printf 'notes\n' > TODO.md
git status --short && git diff --stat

#### status short then diff check then diff stat
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf '// extra\n' >> src/lib.rs
git status --short && git diff --check && git diff --stat

#### status short then diff stat then cached stat
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf '// extra\n' >> src/lib.rs
printf 'Usage: demo run\n' >> README.md
git add README.md
git status --short && git diff --stat && git diff --cached --stat

#### status short then diff stat then a failing diff check
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf 'oops  \n' >> src/lib.rs
git status --short && git diff --stat && git diff --check
echo "status=$?"

#### diff check then status short then diff stat
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf 'Usage: demo run\n' >> README.md
git diff --check && git status --short && git diff --stat

#### diff stat then status short
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf '// extra\n' >> src/main.rs
git diff --stat && git status --short

#### status short then diff check
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf 'tab\t\n' >> README.md
git status --short && git diff --check
echo "status=$?"

#### status short then diff of two paths
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf 'Usage: demo run\n' >> README.md
printf '// end\n' >> src/lib.rs
git status --short && git diff -- README.md src/lib.rs

#### diff of a path then status short
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf '// end\n' >> src/lib.rs
git diff -- src/lib.rs && git status --short

#### diff stat then diff of a path into sed
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    b + a\n}\n' > src/lib.rs
git diff --stat && git diff -- src/lib.rs | sed -n '6,20p'

#### status and stats with printf separators
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf '// end\n' >> src/lib.rs
printf 'Usage: demo run\n' >> README.md && git add README.md
git status --short && printf '\n--- unstaged ---\n' && git diff --stat && printf '\n--- staged ---\n' && git diff --cached --stat

#### status stats and untracked files
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf '// end\n' >> src/lib.rs
printf 'new\n' > CHANGELOG.md && git add CHANGELOG.md
printf 'draft\n' > draft.txt
git status --short && git diff --stat && git diff --cached --stat && git ls-files --others --exclude-standard

#### name-only lists for unstaged staged and untracked
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf '// end\n' >> src/lib.rs
printf 'Usage\n' >> README.md && git add README.md
mkdir -p docs && printf 'x\n' > docs/new.md
git diff --name-only && git diff --cached --name-only && git ls-files --others --exclude-standard

#### diff cached
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf 'Usage: demo run\n' >> README.md && git add README.md
printf '// unstaged\n' >> src/main.rs
git diff --cached

#### diff quiet reports changes in the status
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
git diff --quiet; echo "clean=$?"
printf '// x\n' >> src/main.rs
git diff --quiet; echo "dirty=$?"

#### diff against a revision for a path
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
printf 'v1\n' > config.toml && printf 'a\n' > other.txt && git add -A && git commit -qm "v1"
printf 'v2\n' > config.toml && git commit -qam "v2"
printf 'v3\n' > config.toml
git diff HEAD~1 -- config.toml

#### diff against a branch for several paths
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
git checkout -q -b feature
printf '// feature\n' >> src/lib.rs && printf 'use demo::sub;\n' >> tests/basic.rs
git commit -qam "feature work"
git diff main -- src/lib.rs src/main.rs tests/basic.rs README.md

#### log n with a pretty format
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf 'a\n' >> README.md && git commit -qam "docs: usage"
printf '// b\n' >> src/lib.rs && git commit -qam "lib: comment"
git log -n 3 --pretty=format:'%h %s'
echo
echo "status=$?"

#### log n with pretty oneline
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf 'a\n' >> README.md && git commit -qam "second"
git log -n 2 --pretty=oneline

#### log with a format of author and date
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
git log -n 1 --pretty='%an <%ae> %ad %s' --date=short

#### log oneline
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf 'a\n' >> README.md && git commit -qam "docs"
printf '// t\n' >> tests/basic.rs && git commit -qam "tests"
git log --oneline

#### log oneline limited to a path
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf 'a\n' >> README.md && git commit -qam "docs"
printf '// t\n' >> tests/basic.rs && git commit -qam "tests"
git log --oneline -- tests/

#### log of a branch range
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
git checkout -q -b feature
printf 'f1\n' >> README.md && git commit -qam "feature one"
printf 'f2\n' >> README.md && git commit -qam "feature two"
git log --oneline main..feature

#### status short then log one oneline
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf 'x\n' > new.txt
git status --short && git log -1 --oneline

#### show name-only with an empty format
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf '// x\n' >> src/lib.rs && printf 'use demo::sub;\n' >> tests/basic.rs
git commit -qam "lib and tests"
git show --name-only --format= HEAD

#### show a commit piped into sed -n
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf 'Usage: demo run\n' >> README.md && git commit -qam "docs: usage"
git show HEAD | sed -n '1,14p'

#### show a commit numbered with nl
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf 'Usage: demo run\n' >> README.md && git commit -qam "docs: usage"
git show HEAD | nl -ba | sed -n '5,12p'

#### show a commit by hash
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
printf 'hello\n' > greet.txt && git add greet.txt && git commit -qm "greet"
git show "$(git rev-parse --short HEAD)"

#### show a file at a revision
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf 'changed\n' > src/lib.rs
git show HEAD:src/lib.rs

#### show a file at an older revision into head
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf 'Usage: demo run\n' >> README.md && git commit -qam "docs"
git show HEAD~1:README.md | head -n 2

#### show a missing path at a revision fails
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
git show HEAD:src/missing.rs 2>/dev/null
echo "status=$?"

#### add then commit with a message
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
printf 'hello\n' > a.txt
git add a.txt && git commit -m "add a"

#### add two files then commit
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf '// x\n' >> src/lib.rs && printf 'Usage\n' >> README.md
git add src/lib.rs README.md && git commit -m "lib: comment; docs: usage"
git status --short

#### add all then commit
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf 'new\n' > CHANGELOG.md && rm tests/basic.rs && printf '// x\n' >> src/lib.rs
git add -A && git commit -m "housekeeping"

#### commit with a subject and a body
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf '// x\n' >> src/lib.rs && git add src/lib.rs
git commit -q -m "lib: add comment" -m "Explains the add function."
git log -1 --format=%B

#### commit with nothing to commit fails
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
git commit -m "empty" >/dev/null
echo "status=$?"

#### add of a missing path fails and stops the chain
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
git add nope.txt 2>/dev/null && git commit -m "never"
echo "status=$?"

#### rev-parse show-toplevel from a subdirectory
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main app && mkdir -p app/src/deep && cd app/src/deep
basename "$(git rev-parse --show-toplevel)"
git rev-parse --show-prefix

#### rev-parse of HEAD names
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
git rev-parse --abbrev-ref HEAD
git rev-parse --short HEAD

#### rev-parse outside a repository
export GIT_CEILING_DIRECTORIES="$(dirname "$PWD")"
git rev-parse --is-inside-work-tree 2>/dev/null
echo "status=$?"

#### branch show-current
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
git branch --show-current

#### status short then branch show-current
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf 'x\n' >> README.md
git status --short && git branch --show-current

#### branch show-current then status short on a feature branch
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
git checkout -q -b fix/parser
printf '// fix\n' >> src/lib.rs
git branch --show-current && git status --short

#### branch show-current on a detached HEAD
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf 'a\n' >> README.md && git commit -qam "second"
git checkout -q HEAD~1
git branch --show-current
echo "status=$?"

#### git -C a directory
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main repo
printf 'a\n' > repo/a.txt && git -C repo add a.txt && git -C repo commit -qm "first"
printf 'b\n' > repo/b.txt
git -C repo status --short && git -C repo log --oneline

#### git -C two repositories in a loop
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
for r in api web; do git init -q -b main "$r" && printf '%s\n' "$r" > "$r/name.txt" && git -C "$r" add -A && git -C "$r" commit -qm "init $r"; done
printf 'dirty\n' >> web/name.txt
for r in api web; do echo "== $r"; git -C "$r" status --short; done

#### cd into a repo then status short
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
mkdir -p work && git init -q -b main work/app
printf 'x\n' > work/app/main.go
cd work/app && git status --short

#### cd into a repo then diff
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf '// x\n' >> src/main.rs
cd src && git diff

#### cd into a repo then diff a revision
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main proj && cd proj
printf 'one\n' > f.txt && git add f.txt && git commit -qm "one"
printf 'two\n' >> f.txt && git commit -qam "two"
cd .. && cd proj && git diff HEAD~1

#### cd into a repo then diff stat
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main proj && cd proj
printf 'one\n' > f.txt && git add f.txt && git commit -qm "one"
cd .. && printf 'two\nthree\n' >> proj/f.txt
cd proj && git diff --stat

#### cd into a repo then status and stats
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main proj
printf 'one\n' > proj/f.txt && printf 'x\n' > proj/g.txt && git -C proj add -A && git -C proj commit -qm "one"
printf 'two\n' >> proj/f.txt && printf 'y\n' >> proj/g.txt && git -C proj add g.txt
cd proj && git status --short && git diff --stat && git diff --cached --stat

#### ls-files
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
git ls-files

#### ls-files of untracked files excluding ignored
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf '*.log\n' > .gitignore && touch debug.log new.rs src/extra.rs
git ls-files --others --exclude-standard

#### grep n
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
git grep -n 'add'

#### grep n with a pathspec
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
git grep -n 'fn' -- 'src/*.rs'

#### grep n with no match
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
git grep -n 'no_such_symbol'
echo "status=$?"

#### stash then stash list
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf '// wip\n' >> src/lib.rs
git stash -q && git stash list && git status --short
echo "status=$?"

#### checkout a new branch
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
git checkout -b feature/login 2>&1
git branch --show-current

#### switch create and switch back
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
git switch -q -c topic
printf 'topic\n' >> README.md && git commit -qam "topic work"
git switch -q main && git log --oneline && git branch

#### restore a modified file
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf 'broken\n' > src/lib.rs
git restore src/lib.rs && git status --short && cat src/lib.rs

#### restore staged unstages a file
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
printf 'Usage\n' >> README.md && git add README.md
git restore --staged README.md && git status --short

#### push to a local remote and list remotes
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q --bare -b main remote.git
git init -q -b main repo && cd repo
printf 'a\n' > a.txt && git add a.txt && git commit -qm "first"
git remote add origin ../remote.git
git push -q origin main && git -C ../remote.git log --oneline
git remote -v

#### rebase a feature branch onto main
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src tests && printf '# demo\n\nA small demo.\n' > README.md
printf 'fn main() {\n    println!("hi");\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'use demo::add;\n' > tests/basic.rs
git add -A && git commit -qm "initial commit"
git checkout -q -b feature
printf '// feature\n' >> src/lib.rs && git commit -qam "feature"
git checkout -q main
printf 'Usage\n' >> README.md && git commit -qam "docs"
git checkout -q feature && git rebase -q main && git log --oneline
