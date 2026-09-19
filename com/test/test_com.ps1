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

foreach ($ProgId in @("PadegUCA.Declension", "Padeg.Declension", "Petrovich.Declension")) {
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
    Check "$ProgId GetSex unknown" ([int]$Decl.GetSex("Саша")) -1
    Check "$ProgId GetNominativePadeg" ($Decl.GetNominativePadeg($Fio)) $Fio
    Check "$ProgId GetAppointmentPadeg 3" ($Decl.GetAppointmentPadeg("заведующий сектором", 2)) "заведующего сектором"
    Check "$ProgId GetAppointmentPadeg 5" ($Decl.GetAppointmentPadeg("заведующий сектором", 5)) "заведующим сектором"
    Check "$ProgId GetOfficePadeg 2" ($Decl.GetOfficePadeg("Сектор разработки", 2)) "Сектора разработки"
    Check "$ProgId GetOfficePadeg 4" ($Decl.GetOfficePadeg("Сектор разработки", 4)) "Сектор разработки"
    Check "$ProgId GetFullAppointmentPadeg" ($Decl.GetFullAppointmentPadeg("Начальник цеха", "Цех нестандартного оборудования", 1)) "Начальник цеха нестандартного оборудования"
    Check "$ProgId GetFIOPadeg" ($Decl.GetFIOPadeg("Иванов", "Иван", "Иванович", "м", 3)) "Иванову Ивану Ивановичу"
    Check "$ProgId GetIFPadeg" ($Decl.GetIFPadeg("Иван", "Иванов", "м", 2)) "Ивана Иванова"
    Check "$ProgId GetIFPadegFS" ($Decl.GetIFPadegFS("Марк Твен", "м", 3)) "Марку Твену"
    $a = ""; $b = ""; $c = ""
    $Decl.SeparateFIO("Иванов Иван Иванович", [ref]$a, [ref]$b, [ref]$c)
    Check "$ProgId SeparateFIO" ("$a|$b|$c") "Иванов|Иван|Иванович"
    Check "$ProgId SetDictionary" ($Decl.SetDictionary("C:\none.dic")) $false
    Check "$ProgId Update_Exceptions" ($Decl.Update_Exceptions()) $true
    Check "$ProgId GetExceptionsFileName" ($Decl.GetExceptionsFileName()) ""

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
