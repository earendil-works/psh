# The Windows side of scripts/windows-gate.sh: runs in the VM (Windows PowerShell 5.1),
# started by the host through `prlctl exec`. Do not run it by hand on the host.
#
#   windows-gate.ps1 -Repo //Mac/Home/.../.git -Ref refs/windows-gate/NAME -Name NAME
#                    -Toolchain 1.99.1 -Msrv 1.96 -Legs fmt:300,clippy:1800,...
#   windows-gate.ps1 -Name NAME -Kill
#
# The host runs one at a time (D1072). It fetches the commit at Ref from the host's
# repository (over the Parallels share) into the one shared checkout C:\psh-gate\shared,
# whose target\ every run reuses, and runs the legs there in order, each with its own
# timeout, stopping at the first failure. Output streams back as it is produced.
#
# Nothing outlives it: the runner puts itself into a Job Object that kills every process
# in it when its last handle closes, which happens when the runner exits for any reason.
# After each leg it also kills and waits for everything else in the job, and fails the
# leg if a process of the checkout (a test binary, psh.exe) was still running: a test
# leaked it. Killing the host's prlctl does not kill the runner; the host calls -Kill.
#
# Windows on arm64's XtaCache service opens x64 executables that ran under emulation to
# build its translation cache, and holds them, under load for minutes; a relink of such a
# file fails (LNK1104, "os error 5"). So before each leg the runner waits until nothing
# holds the checkout's executables, and a leg that fails that way runs once more (D1072).
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
# An error of the runner itself is a setup failure (2), not a failing leg (1).
trap {
  Say "windows-gate: runner error: $_"
  Say "$($_.InvocationInfo.PositionMessage)"
  exit 2
}

New-Item -ItemType Directory -Force $Root | Out-Null
$pidFile = Join-Path $Root 'runner.pid'
$dir = Join-Path $Root 'shared'

# The runner of an earlier run, if it is still alive: the PID file names it, and its
# command line proves it is a runner (PIDs are reused).
function Stop-StaleRunner {
  if (-not (Test-Path $pidFile)) { return }
  $old = [int](Get-Content $pidFile -Raw).Trim()
  if ($old -eq $PID) { return }
  $p = Get-CimInstance Win32_Process -Filter "ProcessId = $old" -ErrorAction SilentlyContinue
  if ($p -and $p.CommandLine -and $p.CommandLine -like '*windows-gate.ps1*' -and
      $p.CommandLine -notlike '*-Kill*') {
    Say "windows-gate: killing the runner of an earlier run (pid $old) and its job"
    # Its Job Object takes every process of that run along.
    Stop-Process -Id $old -Force -ErrorAction SilentlyContinue
    Wait-Process -Id $old -Timeout 30 -ErrorAction SilentlyContinue
  }
}

if ($Kill) {
  Stop-StaleRunner
  exit 0
}

Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;

// The runner's Job Object: everything it starts is in it, and dies with it.
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
  static extern bool QueryInformationJobObject(IntPtr job, int infoClass, IntPtr info, int size, IntPtr returned);
  [DllImport("kernel32.dll", SetLastError = true)]
  static extern bool AssignProcessToJobObject(IntPtr job, IntPtr process);
  [DllImport("kernel32.dll")]
  static extern IntPtr GetCurrentProcess();

  static IntPtr job;

  // The handle is deliberately never closed: the process exit closes it.
  public static void KillOnExit() {
    job = CreateJobObjectW(IntPtr.Zero, IntPtr.Zero);
    if (job == IntPtr.Zero) throw new System.ComponentModel.Win32Exception();
    var info = new ExtendedLimits();
    info.Basic.LimitFlags = 0x2000; // JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
    if (!SetInformationJobObject(job, 9, ref info, Marshal.SizeOf(typeof(ExtendedLimits))))
      throw new System.ComponentModel.Win32Exception();
    if (!AssignProcessToJobObject(job, GetCurrentProcess()))
      throw new System.ComponentModel.Win32Exception();
  }

  // The PIDs of the processes in the job (JobObjectBasicProcessIdList), the runner's own
  // included.
  public static int[] Pids() {
    int cap = 1024;
    int size = 8 + IntPtr.Size * cap;
    IntPtr buf = Marshal.AllocHGlobal(size);
    try {
      if (!QueryInformationJobObject(job, 3, buf, size, IntPtr.Zero))
        throw new System.ComponentModel.Win32Exception();
      int n = Marshal.ReadInt32(buf, 4);
      var pids = new int[n];
      for (int i = 0; i < n; i++) pids[i] = (int)Marshal.ReadIntPtr(buf, 8 + i * IntPtr.Size).ToInt64();
      return pids;
    } finally { Marshal.FreeHGlobal(buf); }
  }
}

// Restart Manager: which processes hold files open.
public static class PshGateHolders {
  [StructLayout(LayoutKind.Sequential)]
  struct UniqueProcess { public int Pid; public uint StartLow, StartHigh; }
  [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
  struct ProcessInfo {
    public UniqueProcess Process;
    [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 256)] public string AppName;
    [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 64)] public string ServiceShortName;
    public int ApplicationType; public uint AppStatus; public uint TSSessionId;
    [MarshalAs(UnmanagedType.Bool)] public bool Restartable;
  }
  [DllImport("rstrtmgr.dll", CharSet = CharSet.Unicode)]
  static extern int RmStartSession(out uint session, int flags, string key);
  [DllImport("rstrtmgr.dll")]
  static extern int RmEndSession(uint session);
  [DllImport("rstrtmgr.dll", CharSet = CharSet.Unicode)]
  static extern int RmRegisterResources(uint session, uint nFiles, string[] files, uint nApps, IntPtr apps, uint nSvc, string[] svcs);
  [DllImport("rstrtmgr.dll")]
  static extern int RmGetList(uint session, out uint needed, ref uint n, [In, Out] ProcessInfo[] info, ref uint reasons);

  // "PID name" for every process holding one of the files open.
  public static string[] Of(string[] files) {
    var result = new List<string>();
    if (files.Length == 0) return result.ToArray();
    uint session;
    if (RmStartSession(out session, 0, Guid.NewGuid().ToString()) != 0) return result.ToArray();
    try {
      if (RmRegisterResources(session, (uint)files.Length, files, 0, IntPtr.Zero, 0, null) != 0)
        return result.ToArray();
      uint needed = 0, n = 0, reasons = 0;
      int rc = RmGetList(session, out needed, ref n, null, ref reasons);
      if (rc == 234 && needed > 0) { // ERROR_MORE_DATA
        var info = new ProcessInfo[needed];
        n = needed;
        if (RmGetList(session, out needed, ref n, info, ref reasons) == 0)
          for (int i = 0; i < n; i++) {
            string svc = info[i].ServiceShortName;
            result.Add(info[i].Process.Pid + " " + info[i].AppName + (string.IsNullOrEmpty(svc) ? "" : " (" + svc + ")"));
          }
      }
    } finally { RmEndSession(session); }
    return result.ToArray();
  }
}
'@
[PshGateJob]::KillOnExit()

Stop-StaleRunner
Set-Content -Path $pidFile -Value $PID -Encoding ascii

$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
$env:PSH_GATE = '1'
$env:CARGO_TERM_COLOR = 'never'

# Kills every process in the job but the runner and waits until they are gone. Returns
# "name (pid)" of those that ran an executable from the checkout: leaked by a test.
function Stop-JobOthers {
  $leaked = @()
  $deadline = [Diagnostics.Stopwatch]::StartNew()
  while ($true) {
    $others = @([PshGateJob]::Pids() | Where-Object { $_ -ne $PID })
    if ($others.Count -eq 0) { break }
    foreach ($p in $others) {
      $proc = Get-CimInstance Win32_Process -Filter "ProcessId = $p" -ErrorAction SilentlyContinue
      if ($proc -and $proc.ExecutablePath -and $proc.ExecutablePath -like "$dir\*") {
        $leaked += "$($proc.Name) ($p)"
      }
      Stop-Process -Id $p -Force -ErrorAction SilentlyContinue
    }
    if ($deadline.Elapsed.TotalSeconds -ge 60) {
      Say "windows-gate: processes of the job do not exit: $($others -join ', ')"
      exit 2
    }
    Start-Sleep -Milliseconds 200
  }
  return @($leaked | Select-Object -Unique)
}

# Waits (at most $secs) until no process holds an executable of the checkout's target\
# open; the holders are named when there were any. Returns $false if they remain.
function Wait-Unheld([int]$secs) {
  $target = Join-Path $dir 'target'
  if (-not (Test-Path $target)) { return $true }
  $exes = @(Get-ChildItem $target -Recurse -Include *.exe -File -ErrorAction SilentlyContinue |
    ForEach-Object FullName)
  $watch = [Diagnostics.Stopwatch]::StartNew()
  $said = $false
  while ($true) {
    $holders = @([PshGateHolders]::Of($exes))
    if ($holders.Count -eq 0) { return $true }
    if (-not $said) {
      Say "windows-gate: waiting for other processes to release the checkout's executables: $(($holders | Select-Object -Unique) -join ', ')"
      $said = $true
    }
    if ($watch.Elapsed.TotalSeconds -ge $secs) {
      Say "windows-gate: still held after ${secs}s: $(($holders | Select-Object -Unique) -join ', ')"
      return $false
    }
    Start-Sleep -Seconds 2
  }
}

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

# --- the checkout: one, shared by every run; only changed files are rewritten, so
# cargo's incremental state in target\ stays valid across runs of different worktrees.
if (-not (Test-Path (Join-Path $dir '.git'))) {
  New-Item -ItemType Directory -Force $dir | Out-Null
  Invoke-Setup 'git init' { cmd /d /c "git init -q `"$dir`" 2>&1" }
  Invoke-Setup 'git config' { cmd /d /c "git -C `"$dir`" config core.autocrlf false 2>&1" }
  Invoke-Setup 'git config' { cmd /d /c "git -C `"$dir`" config core.longpaths true 2>&1" }
}
Set-Location $dir

# Leftovers of an earlier run that escaped every job (they should not exist): processes
# running an executable of the checkout.
Get-CimInstance Win32_Process | Where-Object { $_.ExecutablePath -and $_.ExecutablePath -like "$dir\*" } |
  ForEach-Object {
    Say "windows-gate: killing a leftover process: $($_.Name) ($($_.ProcessId))"
    Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue
  }

# The repository on the share belongs to another user as far as Windows can tell.
Invoke-Setup "git fetch $Ref" {
  cmd /d /c "git -c safe.directory=* fetch -q --no-tags `"$Repo`" `"+${Ref}:refs/gate/head`" 2>&1"
}
Invoke-Setup 'git checkout' { cmd /d /c 'git checkout -q --force --detach refs/gate/head 2>&1' }
Invoke-Setup 'git clean' { cmd /d /c 'git clean -q -ffdx -e /target 2>&1' }
Say "windows-gate: $Name at $((& git log -1 --format='%h %s') -join '') ($Toolchain, MSRV $Msrv)"

# --- the legs. Keep them in step with scripts/gate.d (D1070).
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

# One attempt at a leg: 'ok', 'failed', 'held' (failed because an executable was held
# open), 'leaked' or 'timeout'.
function Invoke-Leg([string]$leg, [string]$cmd, [int]$secs, [string]$log) {
  # Only the test legs link executables.
  if ($leg -like 'test*') { [void](Wait-Unheld 300) }
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
    if ($watch.Elapsed.TotalSeconds -ge $secs) { $timedOut = $true; break }
  }
  $fs.Close()
  $leaked = @(Stop-JobOthers)
  if ($timedOut) { return 'timeout' }
  if ($leaked.Count -gt 0) {
    Say "windows-gate: leg '$leg' left processes running (a test leaked them): $($leaked -join ', ')"
    return 'leaked'
  }
  if ($p.ExitCode -eq 0) { return 'ok' }
  if (Select-String -Path $log -Quiet -Pattern 'LNK1104', 'failed to remove file', 'Access is denied. \(os error 5\)') {
    return 'held'
  }
  return 'failed'
}

$logDir = Join-Path $Root 'logs'
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
  $watch = [Diagnostics.Stopwatch]::StartNew()
  $result = Invoke-Leg $leg $cmd ([int]$secs) $log
  if ($result -eq 'held') {
    Say "== windows-gate: $leg could not replace an executable another process held open; once more"
    $result = Invoke-Leg $leg $cmd ([int]$secs - [int]$watch.Elapsed.TotalSeconds) $log
  }
  $took = [int]$watch.Elapsed.TotalSeconds
  switch ($result) {
    'ok' { Say "== windows-gate: $leg ok (${took}s)" }
    'timeout' {
      Say "== windows-gate: TIMEOUT in leg '$leg' after ${took}s"
      exit 124
    }
    default {
      Say "== windows-gate: FAILED in leg '$leg' after ${took}s ($result)"
      exit 1
    }
  }
}
Say "== windows-gate: passed ($([int]$total.Elapsed.TotalSeconds)s)"
Remove-Item $pidFile -ErrorAction SilentlyContinue
exit 0
