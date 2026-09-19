#Requires -Version 5.1
<#
.SYNOPSIS
  Дифф-тест ours-vs-padeg на TSV-датасете + замеры single/batch.

.DESCRIPTION
  Формат датасета (TSV с заголовком):
    method<TAB>p1<TAB>p2<TAB>p3<TAB>expected
  method: FIO | OFFICE | APPOINT | FULLAPPOINT | SEX | NOMINATIVE.
  Пустой expected = только сравнение A vs B (дифф), без абсолюта.

  Режимы: если -Theirs не создаётся (настоящего padeg нет) — ours-only
  (self-check + baseline + замеры). Без настоящего padeg диффов быть
  не должно в self-check режиме (-Ours -eq -Theirs).

.PARAMETER Dataset
  Путь к TSV. По умолчанию com/test/dataset.sample.tsv.

.PARAMETER Ours
  ProgID нашей реализации. По умолчанию Petrovich.Declension.

.PARAMETER Theirs
  ProgID эталона. По умолчанию Padeg.Declension (у нас сейчас тоже наш —
  для сверки с настоящим padeg отдайте ему этот ProgID через реестр/HKCU).

.PARAMETER Reps
  Повторы одного вызова для медианы single-замера.

.PARAMETER BatchRounds
  Кругов прогона всего датасета для batch-замера.
#>
[CmdletBinding()]
param(
    [string]$Dataset = (Join-Path $PSScriptRoot 'dataset.sample.tsv'),
    [string]$Ours = "Petrovich.Declension",
    [string]$Theirs = "Padeg.Declension",
    [string]$Out = "",
    [int]$Reps = 5,
    [int]$BatchRounds = 3
)

$ErrorActionPreference = "Stop"

function Esc([string]$s) {
    # ASCII-безопасный вывод в консоль (кириллица -> \uXXXX).
    ($s.ToCharArray() | ForEach-Object {
        $c = [int]$_
        if ($c -lt 128) { [string]$_ } else { '\u{0:X4}' -f $c }
    }) -join ''
}

function New-Decl([string]$ProgId) {
    try {
        return New-Object -ComObject $ProgId
    } catch {
        Write-Host ("SKIP: cannot create " + $ProgId + " : " + $_.Exception.Message)
        return $null
    }
}

# Возвращает @{ Status = 'ok'|'na'|'err'; Value = ... }
function Invoke-Decl($Decl, [string]$Method, [string]$P1, [string]$P2, [string]$P3) {
    try {
        $v = switch ($Method) {
            "FIO" { $Decl.GetFIOPadegFS($P1, $P2, [int]$P3) }
            "FIO5" {
                $w = $P1 -split " ", 3
                while ($w.Count -lt 3) { $w += "" }
                $Decl.GetFIOPadeg($w[0], $w[1], $w[2], $P2, [int]$P3)
            }
            "IF" { $Decl.GetIFPadegFS($P1, $P2, [int]$P3) }
            "OFFICE" { $Decl.GetOfficePadeg($P1, [int]$P3) }
            "APPOINT" { $Decl.GetAppointmentPadeg($P1, [int]$P3) }
            "FULLAPPOINT" { $Decl.GetFullAppointmentPadeg($P1, $P2, [int]$P3) }
            "SEX" { [int]$Decl.GetSex($P1) }
            "NOMINATIVE" { $Decl.GetNominativePadeg($P1) }
            default { return @{ Status = 'na'; Value = $null } }
        }
        return @{ Status = 'ok'; Value = "$v" }
    } catch {
        return @{ Status = 'err'; Value = $_.Exception.Message }
    }
}

function Median([double[]]$xs) {
    $s = $xs | Sort-Object
    return $s[[math]::Floor($s.Count / 2)]
}

$rows = @()
foreach ($line in (Get-Content -Encoding UTF8 $Dataset)) {
    if ([string]::IsNullOrWhiteSpace($line)) { continue }
    if ($line.StartsWith("method`t")) { continue }
    $f = $line -split "`t", 5
    while ($f.Count -lt 5) { $f += "" }
    $rows += [pscustomobject]@{
        Method = $f[0]; P1 = $f[1]; P2 = $f[2]; P3 = $f[3]; Expected = $f[4]
    }
}
Write-Host ("rows: " + $rows.Count)

$oursObj = New-Decl $Ours
if ($null -eq $oursObj) { throw "Cannot create ours ($Ours), abort." }
$theirsObj = $null
$selfCheck = ($Ours -ceq $Theirs)
if (-not $selfCheck) { $theirsObj = New-Decl $Theirs }
$compareBoth = ($null -ne $theirsObj) -or $selfCheck

$mm = @()
$singleOurs = @{}
$singleTheirs = @{}
$absOursFail = 0
$absTheirsFail = 0
$diffCount = 0

foreach ($row in $rows) {
    # --- single-замеры (медиана из Reps) ---
    $to = @(); $tt = @()
    $ro = $null; $rt = $null
    for ($i = 0; $i -lt $Reps; $i++) {
        $sw = [Diagnostics.Stopwatch]::StartNew()
        $ro = Invoke-Decl $oursObj $row.Method $row.P1 $row.P2 $row.P3
        $sw.Stop(); $to += $sw.Elapsed.TotalMilliseconds
        if ($compareBoth) {
            $obj = if ($selfCheck) { $oursObj } else { $theirsObj }
            $sw = [Diagnostics.Stopwatch]::StartNew()
            $rt = Invoke-Decl $obj $row.Method $row.P1 $row.P2 $row.P3
            $sw.Stop(); $tt += $sw.Elapsed.TotalMilliseconds
        }
    }
    if (-not $singleOurs.ContainsKey($row.Method)) {
        $singleOurs[$row.Method] = @()
        $singleTheirs[$row.Method] = @()
    }
    $singleOurs[$row.Method] += Median $to
    if ($compareBoth) { $singleTheirs[$row.Method] += Median $tt }

    # --- сверки ---
    $exp = $row.Expected
    $oOk = $ro.Status -eq 'ok'
    $tOk = (-not $compareBoth) -or ($rt.Status -eq 'ok')
    if ($oOk -and $exp -ne "" -and ("$($ro.Value)" -cne $exp)) { $absOursFail++ }
    if ($compareBoth -and $tOk -and $exp -ne "" -and ("$($rt.Value)" -cne $exp)) { $absTheirsFail++ }
    $isDiff = $false
    if ($compareBoth -and $oOk -and $tOk -and ("$($ro.Value)" -cne "$($rt.Value)")) {
        $diffCount++; $isDiff = $true
    }
    if ($isDiff -or ($oOk -and $exp -ne "" -and ("$($ro.Value)" -cne $exp))) {
        $mm += [pscustomobject]@{
            Method = $row.Method; P1 = $row.P1; P2 = $row.P2; P3 = $row.P3
            Ours = if ($oOk) { $ro.Value } else { "ERR:" + $ro.Value }
            Theirs = if (-not $compareBoth) { "-" } elseif ($tOk) { $rt.Value } else { "ERR:" + $rt.Value }
            Expected = $exp; Diff = $isDiff
        }
    }
}

# --- batch-замер: весь датасет кругами ---
function Batch-Rate($Obj) {
    $n = 0
    $sw = [Diagnostics.Stopwatch]::StartNew()
    for ($r = 0; $r -lt $BatchRounds; $r++) {
        foreach ($row in $rows) {
            $null = Invoke-Decl $Obj $row.Method $row.P1 $row.P2 $row.P3
            $n++
        }
    }
    $sw.Stop()
    return [math]::Round($n / $sw.Elapsed.TotalSeconds, 1)
}
$rateOurs = Batch-Rate $oursObj
$rateTheirs = if ($compareBoth -and -not $selfCheck) { Batch-Rate $theirsObj } else { $null }

# --- отчёт ---
Write-Host "== correctness =="
Write-Host ("rows=" + $rows.Count + " diffs(AvsB)=" + $diffCount +
    " ours-vs-expected-fail=" + $absOursFail +
    " theirs-vs-expected-fail=" + $absTheirsFail)
Write-Host "== single ms/call (median of medians) =="
foreach ($m in ($singleOurs.Keys | Sort-Object)) {
    $a = [math]::Round((Median $singleOurs[$m]), 3)
    $line = "  " + $m + ": ours=" + $a + "ms"
    if ($compareBoth -and $singleTheirs[$m].Count -gt 0) {
        $b = [math]::Round((Median $singleTheirs[$m]), 3)
        $line += " theirs=" + $b + "ms"
    }
    Write-Host $line
}
Write-Host "== batch throughput =="
Write-Host ("  ours: " + $rateOurs + " rows/sec (" + $BatchRounds + " rounds)")
if ($null -ne $rateTheirs) { Write-Host ("  theirs: " + $rateTheirs + " rows/sec") }

if ([string]::IsNullOrWhiteSpace($Out)) {
    $Out = Join-Path ([IO.Path]::GetTempPath()) "petrovich-difftest"
}
New-Item -ItemType Directory -Path $Out -Force | Out-Null
$mmPath = Join-Path $Out "mismatches.tsv"
"method`tP1`tP2`tP3`tours`ttheirs`texpected`tdiff" | Set-Content -Encoding UTF8 $mmPath
foreach ($m in $mm) {
    ($m.Method + "`t" + $m.P1 + "`t" + $m.P2 + "`t" + $m.P3 + "`t" +
     $m.Ours + "`t" + $m.Theirs + "`t" + $m.Expected + "`t" + $m.Diff) |
        Add-Content -Encoding UTF8 $mmPath
}
Write-Host ("mismatches: " + $mm.Count + " -> " + $mmPath)
if ($mm.Count -gt 0) {
    foreach ($m in ($mm | Select-Object -First 10)) {
        Write-Host ("  MM " + $m.Method + " p1=" + (Esc $m.P1) +
            " ours=" + (Esc ([string]$m.Ours)) +
            " theirs=" + (Esc ([string]$m.Theirs)) +
            " exp=" + (Esc $m.Expected))
    }
}
