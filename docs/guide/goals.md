# What psh optimizes for

Every unusual choice in psh serves one of a few goals. This page states each of them: what
it means for someone who uses psh, why psh chose it, how it gets there, and what it costs
or rules out. The other chapters refer to the goals by the names used here.

> **In short.** psh is **agent-first**: it is built for the commands coding agents write,
> not for people at a prompt. It is **portable** to Windows without any Unix layer, and
> **consistent**: one shell and one set of utilities everywhere, with **bash's behavior**.
> A host can **understand commands** before running them, keeps **control of execution**,
> can **embed** psh with no async runtime, and can run it **sandboxed** over a virtual
> file system.

The goals are not ranked. Where two of them pull in different directions, the chapter
that makes the trade-off says which one won and why. Licensing is a requirement under all
of them.

## The goals

### Agent-first

**For you:** the commands agents write work, and what comes back (output, errors, exit
status) is what an agent expects from bash.

**Why:** psh has one kind of user. In a corpus of about 295,000 commands run by coding
agents, the syntax is narrow: `&&` appears in 55% of commands, pipes in 47%, `cd X && …`
in 46%, `2>&1` in 15%, quoted here-docs in 13%, `$(…)` in 5%; `if`, `case`, `[[ ]]`,
functions and arrays are each below 1%. Agents also mix GNU and BSD habits: GNU
`timeout` and `grep -P` next to BSD `sed -i ''`.

**How:** compatibility work is ordered by how often agents use a feature; utilities accept
both GNU and BSD spellings; everything interactive is left out.

**The cost:** psh is not a shell for people. It has no line editing, completion, history,
prompts or job control beyond `&`, `wait`, `kill` and `$!`.

### Portable

**For you:** the same command runs on macOS, Linux and Windows. On Windows, psh needs no
Git Bash, MSYS, Cygwin or WSL.

**Why:** on Windows, agents otherwise depend on Git Bash, where killed pipelines leave
processes running, output in the console's code page arrives garbled, and `> /dev/null`
creates stray files. Or they fall back to another shell language entirely.

**How:** psh starts processes itself, with the platform's own mechanisms (job objects,
`PATHEXT`, the Windows command-line quoting rules), ships the common utilities in-process,
and accepts both `C:\path` and `/c/path`.

**The cost:** psh owns a substantial amount of Windows process code, and every change is
tested on Windows. Some Unix features, such as stopping and continuing a process, have no
Windows counterpart.

### Consistent

**For you:** one shell and one utility dialect wherever an agent runs: a laptop, a remote
machine, a sandbox.

**Why:** without it, an agent meets bash 3.2 on one machine, zsh on another (where an
unmatched glob is an error instead of a literal word), and Git Bash on a third. The same
command fails in one place and works in another.

**How:** psh ships with its host instead of using whatever shell is installed. Its
utilities are the same code on every platform, with one flag dialect: the union of GNU and
BSD, with fixed rules where the two conflict.

**The cost:** a psh utility can differ in a detail from the platform's own tool (column
padding, for example). Commands that report on the system, such as `uname`, still report
the real host.

### Bash's behavior

**For you:** a script does what GNU bash 5.3 does with it, prints what bash prints,
including error messages, and exits with the same status.

**Why:** models are trained on bash and read its messages. A shell that is "like bash"
but differs in corners is harder to use than one that matches.

**How:** every behavior test compares psh with output recorded from bash 5.3. Each
deliberate difference has a reason and is listed in one place.

**The cost:** bash's quirks become targets too, and the last part of compatibility is a
long tail.

### Understandable commands

**For you:** a host can see what a command will do before it runs it: which programs it
calls, which files it writes, whether it only reads, whether it reaches the network, with
positions into the original text.

**Why:** hosts summarize commands for people, auto-approve the ones that only read (about
half of what agents run), and check permissions. Patterns matched against the command
string are easy to get around (`$(…)`, `;` after a harmless prefix, a command name in a
variable).

**How:** psh's parser records the byte range of every part of a command, and the
classifier works on the same syntax tree the engine runs, so a command cannot mean one
thing to the check and another to the shell. The parser builds on its own, also for
WebAssembly.

**The cost:** the syntax tree becomes an interface others depend on. And the classifier
must be conservative: anything it cannot see through (a command name from a variable,
`eval`, a script it does not read) is "unknown", never "read-only".

### Control of execution

**For you:** a command that is cancelled or times out leaves nothing behind; background
processes are tracked; `timeout` exists on every platform.

**Why:** agents start servers, poll with `sleep` loops and forget processes. Without
control, orphans keep running for days, and on Windows a kill often misses grandchildren.

**How:** every program psh starts runs in its own process group or job object;
cancellation is checked between commands and sends a terminate signal, then a kill after
a grace period, to the whole tree.

**The cost:** psh must start every process itself. A program that deliberately detaches
needs `nohup` or an explicit release.

### Embeddable

**For you:** psh links into a host program as a library. It brings no async runtime and
few dependencies, the host keeps control of its threads, and the binary stays small.

**Why:** psh's first host is a small synchronous daemon copied over SSH to eight
platforms, where every dependency costs build time and size.

**How:** the engine is a state machine the host steps, woken with the standard library's
wakers; nearly all code is psh's own.

**The cost:** psh writes more itself (the parser, the process layer, many utilities) and
cannot lean on the async ecosystem.

### Sandboxable

**For you:** the same shell runs in a browser or a sandbox, over a virtual file system,
with commands the host provides instead of real programs.

**Why:** some hosts have no operating system processes at all.

**How:** every side effect goes through a host interface whose operations may complete
later; the engine itself does no I/O and compiles to WebAssembly without special
transforms.

**The cost:** every engine path has to cope with operations that are not finished yet.
In a browser there is no git, Python or Node unless the host supplies them.

## Under every goal: a permissive license

**For you:** psh can be shipped inside other products under a permissive license.

**How:** psh uses only MIT, Apache and BSD-style code. It is written without reading GNU
bash's source or any other GPL implementation; GPL programs serve only as references to
compare output with.

**The cost:** some parts are slower to write than a port would be.

## Non-goals

psh is not an interactive shell, a login shell or a POSIX `sh`, and has no zsh mode. Its
utilities never include interpreters, compilers, package managers or database clients
(`python`, `cargo`, `npm`, `sqlite3`): those stay real programs, which also keeps them
visible to a classifier. Conformance to a test
suite for its own sake is not a target; matching bash on what agents run is.
