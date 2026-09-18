# PLAN-COM.md — petrovich-com: drop-in замена padeg.dll (`Padeg.Declension`)

## 0. Рамки (решение 2026-09-19)

- **Только 4 метода** сервера автоматизации `Declension`, совместимость **по вызовам**:
  `CreateObject("Padeg.Declension")` + `GetSex`, `GetFIOPadegFS`,
  `GetNominativePadeg`, `GetAppointmentPadeg`. Плоские C-экспорты padeg.dll
  (`GetFIOPadeg` с `PChar`-буферами и т.д.) — вне скоупа.
- **Без прав администратора** (Windows 10/11): никакой записи в HKLM, никакого
  `regsvr32` в HKLM. Регистрация — только **HKCU\Software\Classes** (+ 32-битный
  вид `Wow6432Node` для x86-клиентов) через собственный скрипт; альтернатива
  вообще без реестра — **registration-free COM** через манифест клиента.
- **x32 и x64**: две сборки (`i686-pc-windows-gnu`, `x86_64-pc-windows-gnu`),
  один установщик регистрирует обе; битность DLL обязана совпадать с битностью
  процесса-потребителя (1С/VBS).
- **Сборка COM — только MSVC** (решение 2026-09-19; остальной workspace остаётся
  на gnu). Проверено: VS 2022 BuildTools 17.14 + SDK 10.0.22621 на месте,
  `cargo build --target x86_64-pc-windows-msvc` работает из коробки.
- **Без внешних crate-зависимостей** в COM-слое: весь COM-plumbing — вручную на
  raw FFI (`Ole32.lib`/`OleAut32.lib` из SDK). Причина: меньше зависимостей —
  меньше мест, где x86/x64 сборки могут разойтись.
- Свой CLSID (генерируем один раз, фиксируем). ProgID — **`Padeg.Declension`**
  (drop-in: существующий код `CreateObject("Padeg.Declension")` работает без
  правок) + алиас `Petrovich.Declension`. Внимание: запись в HKCU **затеняет**
  HKLM-запись настоящего padeg для текущего пользователя — это и есть механизм
  drop-in; документировано в `com/README.md`.
- `ThreadingModel = Apartment` (как у VB6-серверов автоматизации; 1С/VBS —
  STA-клиенты).

## 1. Сигнатуры (факты из COM-примера Directum)

```vbscript
Decl = CreateObject("Padeg.Declension")
Somes   = Decl.GetFIOPadegFS(FIO, "", 3)   ' дательный
SomeStr = Decl.GetFIOPadegFS(FIO, "", 1)   ' именительный
Somest  = Decl.GetFIOPadegFS(FIO, "", 5)   ' творительный
```

Отсюда фиксируем контракт (safecall = HRESULT + скрытый retval):

| Метод | Параметры | Возврат | Семантика |
|---|---|---|---|
| `GetSex(FIO)` | `WideString` (полное ФИО, несмотря на имя `cMiddleName` в Pascal-доках) | `Integer` | `1` = мужской (для `IIf(Gender,"м","ж")`), `0` = женский/не определён |
| `GetFIOPadegFS(FIO, Sex, Padeg)` | `WideString, WideString, Integer` | `WideString` | склонение `"Фамилия Имя Отчество"`; `Sex`: `""`/`auto` = определить, `м*` = муж., `ж*`/`f*` = жен. (регистронезависимо, рус/лат); `Padeg`: `1`–`6` (1 = именительный = как есть) |
| `GetNominativePadeg(FIO)` | `WideString` | `WideString` | восстановление именительного падежа. **Ограничение**: ядро умеет только именительный→падеж, поэтому identity (документ.). Для штатного потока (`FIO` уже в именительном) — функциональный эквивалент |
| `GetAppointmentPadeg(Appointment, Padeg)` | `WideString, Integer` | `WideString` | склонение должности. **Ограничение**: в ядре нет правил должностей → identity (документ.). Не молча «почти правильно», а предсказуемо |

`nPadeg ∉ 1..6` → COM-ошибка (`DISP_E_BADPARAMCOUNT`? нет — `DISP_E_OVERFLOW`/`E_INVALIDARG` с текстом; решение: `E_INVALIDARG` + `EXCEPINFO.bstrDescription`). Пустое ФИО → пустая строка (не ошибка — как у padeg для пустого входа; tolerate).

## 2. Этапы (от простого к сложному)

### Этап C1. Чистый адаптер — без Windows API ✅ 2026-09-19
- `com/` крейт `petrovich-com` (lib): `adapter.rs` — 4 функции поверх
  `petrovich-core`, парсинг `"Фамилия Имя Отчество"` (split whitespace,
  лишние части: 4+ слова → фамилия = первое слово? решение: фамилия = слова
  до 2 последних? **Нет**: padeg берёт ровно 3 — лишнее отбрасываем справа?
  Решение: берём первые 3 слова, остаток игнорируем — документ.).
- `nPadeg 1` = identity; `2..6` → `Case::{Genitive,Dative,Accusative,Instrumental,Prepositional}`.
- Unit-тесты адаптера (все 4 метода × полы × падежи 1–6 + ошибки).
- Собирается и тестируется на любом таргете (даже Linux) — чистая логика.

### Этап C2. COM-plumbing вручную ✅ 2026-09-19
- `com/src/com.rs` (только Windows): GUID-ы (CLSID фиксирован), `DllGetClassObject`,
  `DllCanUnloadNow`, `IClassFactory`, `IDispatch` (`GetIDsOfNames`/`Invoke`,
  DISPID 1–4), маршалинг `BSTR↔String` (`SysAllocStringLen`/`SysFreeString`),
  `VARIANT` (VT_BSTR/VT_I4), safecall-стиль (ошибки → HRESULT + EXCEPINFO).
- `DllRegisterServer`/`DllUnregisterServer` пишут **только HKCU**
  (через advapi32 `RegCreateKeyExW`/`RegSetValueExW` — `libadvapi32.a` есть
  в тулчейне; проверить, иначе тот же raw-dylib).
- cdylib `petrovich_com.dll`; дефолт-билд x64; x86 — `i686-pc-windows-gnu`
  (доставить `rustup target add`, проверить stdcall-декорации на линковке).

### Этап C3. Установка без админа ✅ 2026-09-19
- `com/install-user.ps1`: определяет разрядность ОС, регистрирует x64 DLL
  всегда + x86 DLL (виды `HKCU\Software\Classes\CLSID` и `...\Wow6432Node\CLSID`,
  `InprocServer32` с полным путём, `ProgID` + `Petrovich.Declension`,
  `ThreadingModel=Apartment`). Проверка: скрипт падает с понятной ошибкой,
  если запущен с путями из прошлой установки.
- `com/uninstall-user.ps1`: удаляет те же ключи.
- `com/client.manifest`: registration-free вариант (клиент кладёт рядом со
  своим .exe; реестра вообще нет). Проверка на тестовом .exe-хосте.
- `.reg`-файлы НЕ используем как основной путь (в них абсолютные пути —
  генерирует скрипт), но `install-user.ps1` умеет `-WhatIf`-дамп.

### Этап C4. Верификация ✅ 2026-09-19 (кроме ручного прогона из 1С — нет доступа)
- PowerShell E2E: регистрация → `New-Object -ComObject Padeg.Declension` →
  4 метода по сценарию из PLAN.md (п. Тестирование) → сверка строк → разрегистрация.
  x64 — обязательно; x86 — через 32-битный PowerShell, если есть.
- VBScript `com/test/test_padeg.vbs` (cscript) — ручной прогон 1в1 по примеру Directum.
- `cargo test -p petrovich-com` (адаптер), `clippy -D warnings`, `fmt --check`.
- `com/README.md` (установка без админа, ProgID/CLSID, ограничения Nominative/Appointment,
  x32/x64) + отметка в корневом `PLAN.md`/README.

## 3. Открытые вопросы (не блокеры C1–C2)

1. Точный CLSID настоящего padeg — неизвестен; используем свой. Если у заказчика
   код привязан к CLSID строкой (редко) — понадобится алиас-ключ, уточнить.
2. `EOleException`-обработка в клиентском коде — как в PLAN.md риск №4.
3. Нужна ли склоняемость должностей по-настоящему (правила должностей) —
   отдельная задача с отдельным датасетом; сейчас identity.
