#Requires -Version 5.1
<#
.SYNOPSIS
  E2E-проверка Padeg.Declension через late binding (без админа).
  Вызывается вручную или из install-user.ps1 -Test.
#>
$ErrorActionPreference = "Stop"

$Failed = 0
function Check([string]$Name, $Actual, $Expected) {
    if ("$Actual" -ceq "$Expected") {
        Write-Host "PASS: $Name"
    } else {
        Write-Host "FAIL: $Name -- expected [$Expected], got [$Actual]"
        $script:Failed++
    }
}

foreach ($ProgId in @("Padeg.Declension", "Petrovich.Declension")) {
    try {
        $Decl = New-Object -ComObject $ProgId
    } catch {
        Write-Host "FAIL: New-Object $ProgId : $($_.Exception.Message)"
        $script:Failed++
        continue
    }
    Write-Host "PASS: New-Object $ProgId"

    $Fio = "Иванов Иван Иванович"
    Check "$ProgId GetFIOPadegFS 1" ($Decl.GetFIOPadegFS($Fio, "", 1)) $Fio
    Check "$ProgId GetFIOPadegFS 3" ($Decl.GetFIOPadegFS($Fio, "", 3)) "Иванову Ивану Ивановичу"
    Check "$ProgId GetFIOPadegFS 4 m" ($Decl.GetFIOPadegFS($Fio, "м", 4)) "Иванова Ивана Ивановича"
    Check "$ProgId GetFIOPadegFS 5" ($Decl.GetFIOPadegFS($Fio, "", 5)) "Ивановым Иваном Ивановичем"
    Check "$ProgId GetSex male" ([int]$Decl.GetSex($Fio)) 1
    Check "$ProgId GetSex female" ([int]$Decl.GetSex("Петрова Анна Сергеевна")) 0
    Check "$ProgId GetNominativePadeg" ($Decl.GetNominativePadeg($Fio)) $Fio
    Check "$ProgId GetAppointmentPadeg" ($Decl.GetAppointmentPadeg("генеральный директор", 3)) "генеральный директор"

    # Ошибка на неверном падеже должна прийти как COM-исключение.
    try {
        $null = $Decl.GetFIOPadegFS($Fio, "", 9)
        Write-Host "FAIL: $ProgId bad padeg did not throw"
        $script:Failed++
    } catch {
        Write-Host "PASS: $ProgId bad padeg throws"
    }

    [void][Runtime.InteropServices.Marshal]::ReleaseComObject($Decl)
}

if ($Failed -gt 0) { throw "E2E: $Failed проверок не прошли." }
Write-Host "E2E: все проверки прошли."
