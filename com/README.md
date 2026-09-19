# petrovich-com — drop-in замена padeg.dll (`Padeg.Declension`)

COM-сервер автоматизации поверх `petrovich-core`. Существующий код вида
`CreateObject("Padeg.Declension")` работает без правок и **без прав
администратора** (Windows 10/11), в процессах **x86 и x64**.

Подробный план и контракт — в [`../PLAN-COM.md`](../PLAN-COM.md).

## Методы COM (`PadegUCA.Declension` — первичный по доке v4.1; также
`Padeg.Declension`, `Petrovich.Declension`)

| Метод | Параметры | Возврат |
|---|---|---|
| `GetSex(FIO)` | полное ФИО | `Integer`: `1`/`0`/`-1` (неизвестно), как в доке |
| `GetFIOPadegFS(FIO, Sex, Padeg)` | `"Фамилия Имя Отчество"`, пол (`""`/`auto`/`м*`/`ж*`), падеж `1`–`6` | склонённое ФИО |
| `GetFIOPadeg(LN, FN, MN, Sex, Padeg)` | части раздельно | склонённое ФИО |
| `GetIFPadeg(FN, LN, Sex, Padeg)` / `GetIFPadegFS(IF, Sex, Padeg)` | порядок «имя фамилия»; последнее слово — фамилия | склонение |
| `SeparateFIO(FIO, out LN, FN, MN)` | процедура | разбивка |
| `GetNominativePadeg(FIO)` | ФИО | вход без изменений (ограничение ядра) |
| `GetAppointmentPadeg(App, Padeg)` | должность | первое слово склоняется (см. находки) |
| `GetOfficePadeg(Office, Padeg)` | подразделение | первое слово склоняется |
| `GetFullAppointmentPadeg(App, Office, Padeg)` | оба | склейка без дублей + склонение |
| `SetDictionary` / `Update_Exceptions` / `GetExceptionsFileName` | — | `false` / `true` / `""` (словаря нет, честно) |

Ошибки: `EXCEPINFO.scode` = код padeg (`-1` падеж, `-2` род).
Плоские C-экспорты (`stdcall`, UTF-16): все вышеплюс `GetFIOPadegAS/FSAS`,
`GetFIOParts`, `GetPadegID` (`0`), словарь — см. `src/capi.rs` и доку v4.1 §4.

`Padeg`: `1` именительный, `2` родительный, `3` дательный, `4` винительный,
`5` творительный, `6` предложный. Неверный падеж/пол → COM-ошибка с текстом
(`EXCEPINFO`), пустой вход → пустая строка.

## Сборка (только MSVC)

```bat
cargo build -p petrovich-com --target x86_64-pc-windows-msvc --release
cargo build -p petrovich-com --target i686-pc-windows-msvc --release
```

COM-слой без внешних crate: ручной `IDispatch` на raw FFI (`Ole32.lib` /
`OleAut32.lib` из SDK). Нужны VS 2022 BuildTools + Windows SDK (проверено:
17.14 + 10.0.22621) и таргет `i686-pc-windows-msvc` (`rustup target add`).

## Установка без админа

```powershell
# из каталога репозитория, обычные права пользователя:
powershell -ExecutionPolicy Bypass -File com\install-user.ps1
powershell -ExecutionPolicy Bypass -File com\install-user.ps1 -Test        # + E2E-проверка
powershell -ExecutionPolicy Bypass -File com\uninstall-user.ps1            # снять регистрацию
```

## Если запуск ps1 запрещён политикой

`ExecutionPolicy` касается только PowerShell-скриптов. Рядом лежат
`.cmd`-двойники с той же логикой (чистый batch, политики не касается):

```bat
com\install-user.cmd            :: регистрация обеих DLL, Release
com\install-user.cmd Debug      :: то же для Debug-сборок
com\uninstall-user.cmd          :: снять регистрацию
```

E2E-проверка без PowerShell — расширенный VBScript (оба ProgID, все методы,
ошибка на неверном падеже; требует UTF-16 с BOM — так и лежит):

```bat
cscript //nologo com\test\test_padeg.vbs                        :: x64
C:\Windows\SysWOW64\cscript.exe //nologo com\test\test_padeg.vbs :: x86
```

Вердикт — строка `E2E VBS RESULT=PASS` + код выхода `0`. (Если запрещён и
`cscript`, остаётся ручная проверка из 1С/VBA по примеру Directum.)

Скрипт вызывает `DllRegisterServer` обеих DLL через `rundll32`; DLL пишет
**только** `HKCU\Software\Classes` (для x86-клиентов — вид `Wow6432Node`,
через 32-битный `rundll32` из `SysWOW64`). HKLM не трогается, UAC нет.
`ProgID`: `Padeg.Declension` (drop-in) + алиас `Petrovich.Declension`,
`ThreadingModel=Apartment`, CLSID `{E4B2C1A0-5D6F-4A7B-8C9D-0E1F2A3B4C5D}`.

ВНИМАНИЕ: запись `HKCU\...\Padeg.Declension` затеняет одноимённую запись
настоящего padeg в HKLM **для текущего пользователя** — в этом и состоит
механизм drop-in. На машинах с настоящим padeg это осознанная замена.

Без реестра вообще: `com/client.manifest` (registration-free COM) — положить
рядом с .exe клиента как `<имя>.manifest`, DLL той же битности — рядом.

## Проверки

```bat
cargo test -p petrovich-com --target x86_64-pc-windows-msvc   :: 13 тестов: адаптер + COM in-process
powershell -ExecutionPolicy Bypass -File com\test\test_com.ps1  :: E2E обоих ProgID (x64; 32-битным powershell — x86)
cscript //nologo com\test\test_padeg.vbs                        :: полный E2E без PowerShell (20 проверок)
```

## Ловушки реализации (зафиксировано отладкой 2026-09-19)

1. **Фабрика обязана быть объектом.** Указатель интерфейса, возвращаемый
   `DllGetClassObject`, ole32 разыменовывает как объект (`vtbl` = первое поле).
   Возврат адреса самой vtable даёт wild jump (`0xC0000409`, fault в «unknown»).
   У нас: `static FACTORY_OBJ: ClassFactory { vtbl }`. In-process тесты этого
   не ловят (там тот же неверный адрес читали «правильно» вручную).
2. **VBScript/1C шлют `VT_BYREF`.** Переменные приходят как
   `VT_VARIANT|VT_BYREF` (скаляры — `VT_<T>|VT_BYREF`); без разыменования —
   ошибка 438. `resolve_alias()` снимает ссылки (только `[in]`, записи нет).
3. **Флаги `Invoke`:** VBScript зовёт с `DISPATCH_METHOD|DISPATCH_PROPERTYGET`
   (`0x3`) — проверять надо именно бит `DISPATCH_METHOD`.
4. **Кодировки скриптов:** `.ps1` с кириллицей — только с UTF-8 BOM (иначе PS 5.1
   парсит как ANSI и ломается); `.vbs` с кириллицей — UTF-16 с BOM.
5. **`rgszNames` в `GetIDsOfNames` — это LPWSTR, не BSTR!** Читать до `\0`,
   `SysStringLen` здесь роняет процесс.
