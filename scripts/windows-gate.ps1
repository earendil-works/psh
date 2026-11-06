# The Windows side of scripts/windows-gate.sh: runs in the VM (Windows PowerShell 5.1),
# started by the host through `prlctl exec`. Do not run it by hand on the host.
#
#   windows-gate.ps1 -Repo //Mac/Home/.../.git -Ref refs/windows-gate/NAME -Name NAME
#                    -Toolchain 1.99.1 -Msrv 1.96 -Legs fmt:300,clippy:1800,...
#   windows-gate.ps1 -Name NAME -Kill
#
# It fetches the commit at Ref from the host's repository (over the Parallels share) into
# a checkout of its own, C:\psh-gate\NAME, and runs the legs there in order, each with its
# own timeout, stopping at the first failure. Output streams back as it is produced.
#
# Nothing outlives it: the runner puts itself into a Job Object that kills every process
# in it when its last handle closes, which happens when the runner exits for any reason.
# Killing the host's prlctl does not kill the runner, so the host calls -Kill as cleanup.
#
# Exit status: 0 all legs passed, 1 a leg failed, 124 a leg timed out, 2 setup failed.
param(
  [string]$Repo,
  [string]$Ref,
  [Parameter(Mandatory = $true)][string]$Name,
  [string]$Toolchain,
  [string]$Msrv,
  [string]$Legs,
  [string]$Root = 'C:\psh-gate',
  [switch]$Kill
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 3

$out = [Console]::OpenStandardOutput()
function Say([string]$s) {
  $b = [Text.Encoding]::UTF8.GetBytes($s + "`n")
  $out.Write($b, 0, $b.Length)
  $out.Flush()
}

$pidFile = Join-Path $Root "$Name.pid"

# The runner of an earlier run for this NAME, if it is still alive: its PID file names
# it, and its command line proves it is ours (PIDs are reused).
function Get-StaleRunner {
  if (-not (Test-Path $pidFile)) { return $null }
  $old = [int](Get-Content $pidFile -Raw).Trim()
  if ($old -eq $PID) { return $null }
  $p = Get-CimInstance Win32_Process -Filter "ProcessId = $old" -ErrorAction SilentlyContinue
  if ($p -and $p.CommandLine -and $p.CommandLine -like '*windows-gate.ps1*' -and
      $p.CommandLine -like "*-Name $Name *" -and $p.CommandLine -notlike '*-Kill*') {
    return $p
  }
  return $null
}

function Stop-StaleRunner {
  $p = Get-StaleRunner
  if ($p) {
    Say "windows-gate: killing the runner of an earlier run (pid $($p.ProcessId)) and its job"
    # Its Job Object takes every process of that run along.
    Stop-Process -Id $p.ProcessId -Force -ErrorAction SilentlyContinue
    Wait-Process -Id $p.ProcessId -Timeout 30 -ErrorAction SilentlyContinue
  }
}

if ($Kill) {
  Stop-StaleRunner
  exit 0
}

# --- the Job Object: everything this runner starts dies with it.
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class PshGateJob {
  [StructLayout(LayoutKind.Sequential)]
  struct BasicLimits {
    public long PerProcessUserTimeLimit; public long PerJobUserTimeLimit;
    public uint LimitFlags; public UIntPtr MinimumWorkingSetSize; public UIntPtr MaximumWorkingSetSize;
    public uint ActiveProcessLimit; public UIntPtr Affinity; public uint PriorityClass; public uint SchedulingClass;
  }
  [StructLayout(LayoutKind.Sequential)]
  struct IoCounters {
    public ulong ReadOperationCount, WriteOperationCount, OtherOperationCount;
    public ulong ReadTransferCount, WriteTransferCount, OtherTransferCount;
  }
  [StructLayout(LayoutKind.Sequential)]
  struct ExtendedLimits {
    public BasicLimits Basic; public IoCounters Io;
    public UIntPtr ProcessMemoryLimit, JobMemoryLimit, PeakProcessMemoryUsed, PeakJobMemoryUsed;
  }
  [DllImport("kernel32.dll", SetLastError = true)]
  static extern IntPtr CreateJobObjectW(IntPtr attributes, IntPtr name);
  [DllImport("kernel32.dll", SetLastError = true)]
  static extern bool SetInformationJobObject(IntPtr job, int infoClass, ref ExtendedLimits info, int size);
  [DllImport("kernel32.dll", SetLastError = true)]
  static extern bool AssignProcessToJobObject(IntPtr job, IntPtr process);
  [DllImport("kernel32.dll")]
  static extern IntPtr GetCurrentProcess();
  // The handle is deliberately never closed: the process exit closes it.
  public static void KillOnExit() {
    IntPtr job = CreateJobObjectW(IntPtr.Zero, IntPtr.Zero);
    if (job == IntPtr.Zero) throw new System.ComponentModel.Win32Exception();
    var info = new ExtendedLimits();
    info.Basic.LimitFlags = 0x2000; // JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
    if (!SetInformationJobObject(job, 9, ref info, Marshal.SizeOf(typeof(ExtendedLimits))))
      throw new System.ComponentModel.Win32Exception();
    if (!AssignProcessToJobObject(job, GetCurrentProcess()))
      throw new System.ComponentModel.Win32Exception();
  }
}
'@
[PshGateJob]::KillOnExit()

New-Item -ItemType Directory -Force $Root | Out-Null
Stop-StaleRunner
Set-Content -Path $pidFile -Value $PID -Encoding ascii

$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
$env:PSH_GATE = '1'
$env:CARGO_TERM_COLOR = 'never'

# Runs a native command for setup; output streams through, failure stops the run.
function Invoke-Setup([string]$what, [scriptblock]$cmd) {
  $global:LASTEXITCODE = 0
  # Native stderr must not become PowerShell errors (5.1 turns redirected stderr lines
  # into ErrorRecords); cmd merges the streams before PowerShell sees them.
  & $cmd | ForEach-Object { Say "$_" }
  if ($LASTEXITCODE -ne 0) {
    Say "windows-gate: setup failed: $what (exit $LASTEXITCODE)"
    exit 2
  }
}

# --- toolchains: the host's rustc version (so clippy and rustfmt agree with the host
# gate) and the MSRV, installed on first use.
$installed = (& cmd /d /c 'rustup toolchain list 2>&1') -join "`n"
foreach ($tc in @($Toolchain, $Msrv)) {
  if ($installed -notmatch "(?m)^$([regex]::Escape($tc))-") {
    Invoke-Setup "rustup toolchain install $tc" {
      cmd /d /c "rustup toolchain install $tc --profile minimal -c clippy,rustfmt 2>&1"
    }
  }
}
$targets = (& cmd /d /c "rustup target list --installed --toolchain $Toolchain 2>&1") -join "`n"
if ($targets -notmatch 'x86_64-pc-windows-msvc') {
  Invoke-Setup "rustup target add x86_64-pc-windows-msvc" {
    cmd /d /c "rustup target add --toolchain $Toolchain x86_64-pc-windows-msvc 2>&1"
  }
}

# --- the checkout: a clone of its own per NAME; only changed files are rewritten, so
# cargo's incremental state in target\ stays valid.
$dir = Join-Path $Root $Name
if (-not (Test-Path (Join-Path $dir '.git'))) {
  New-Item -ItemType Directory -Force $dir | Out-Null
  Invoke-Setup 'git init' { cmd /d /c "git init -q `"$dir`" 2>&1" }
  Invoke-Setup 'git config' { cmd /d /c "git -C `"$dir`" config core.autocrlf false 2>&1" }
  Invoke-Setup 'git config' { cmd /d /c "git -C `"$dir`" config core.longpaths true 2>&1" }
}
Set-Location $dir
# The repository on the share belongs to another user as far as Windows can tell.
Invoke-Setup "git fetch $Ref" {
  cmd /d /c "git -c safe.directory=* fetch -q --no-tags `"$Repo`" `"+${Ref}:refs/gate/head`" 2>&1"
}
Invoke-Setup 'git checkout' { cmd /d /c 'git checkout -q --force --detach refs/gate/head 2>&1' }
Invoke-Setup 'git clean' { cmd /d /c 'git clean -q -ffdx -e /target 2>&1' }
Say "windows-gate: $Name at $((& git log -1 --format='%h %s') -join '') ($Toolchain, MSRV $Msrv)"

# --- the legs. Keep them in step with scripts/gate.sh (D1070).
function Get-LegCommand([string]$leg) {
  switch ($leg) {
    'fmt' { return "cargo +$Toolchain fmt --all --check" }
    'clippy' { return "cargo +$Toolchain clippy --workspace --all-targets --locked -- -D warnings" }
    'test' { return "cargo +$Toolchain test --workspace --locked" }
    'test-x64' { return "cargo +$Toolchain test --workspace --locked --target x86_64-pc-windows-msvc" }
    'msrv' { return "cargo +$Msrv check --workspace --all-targets --locked" }
  }
  return $null
}

$logDir = Join-Path $Root "$Name.logs"
New-Item -ItemType Directory -Force $logDir | Out-Null
$total = [Diagnostics.Stopwatch]::StartNew()
foreach ($entry in $Legs.Split(',')) {
  $leg, $secs = $entry.Split(':')
  $cmd = Get-LegCommand $leg
  if (-not $cmd) {
    Say "windows-gate: unknown leg '$leg'"
    exit 2
  }
  Say "== windows-gate: $leg"
  $log = Join-Path $logDir "$leg.log"
  Set-Content -Path $log -Value $null -NoNewline
  # cmd merges stdout and stderr into the log in order; the loop below streams the log.
  $p = Start-Process -FilePath cmd.exe -NoNewWindow -PassThru -WorkingDirectory $dir `
    -ArgumentList "/d /s /c `"$cmd > `"$log`" 2>&1`""
  $null = $p.Handle # without it, ExitCode stays empty after the exit (a .NET quirk)
  $fs = [IO.File]::Open($log, 'Open', 'Read', 'ReadWrite, Delete')
  $buf = New-Object byte[] 65536
  $watch = [Diagnostics.Stopwatch]::StartNew()
  $timedOut = $false
  while ($true) {
    $exited = $p.WaitForExit(500)
    while (($n = $fs.Read($buf, 0, $buf.Length)) -gt 0) { $out.Write($buf, 0, $n) }
    $out.Flush()
    if ($exited) { break }
    if ($watch.Elapsed.TotalSeconds -ge [int]$secs) { $timedOut = $true; break }
  }
  $fs.Close()
  $took = [int]$watch.Elapsed.TotalSeconds
  if ($timedOut) {
    # Exiting closes the job, which kills the leg's whole tree.
    Say "== windows-gate: TIMEOUT in leg '$leg' after ${took}s"
    exit 124
  }
  if ($p.ExitCode -ne 0) {
    Say "== windows-gate: FAILED in leg '$leg' after ${took}s (exit $($p.ExitCode))"
    exit 1
  }
  Say "== windows-gate: $leg ok (${took}s)"
}
Say "== windows-gate: passed ($([int]$total.Elapsed.TotalSeconds)s)"
Remove-Item $pidFile -ErrorAction SilentlyContinue
exit 0
