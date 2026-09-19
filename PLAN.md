# PLAN.md — petrovich-rs: ядро + CLI + Web + COM drop-in для padeg

> Единый план. Раньше это были три файла (`PLAN.md`, `PLAN-COM.md`,
> `PLAN-COM-API.md`); 2026-09-19 объединены сюда как подразделы
> (§6 — COM drop-in, §7 — весь API padeg, §8 — должности, §9 — дифф-харнес).
> Статус: этапы 0–4 выполнены, этап 5 — частично (нет CI и тега релиза).

## 0. Статус на 2026-09-19 — всё запланированное выполнено

- **Workspace** (единая версия правил, resolver 3, edition 2024):
  `petrovich-core` (lib `petrovich`, корень), `cli` (бинарь `petrovich`),
  `web` (lib `petrovich_web` + бинарь `petrovich-web`), `com`
  (`petrovich-com`: адаптер + COM + плоские C-экспорты).
- **Зависимости — строгая звезда** (коммит `7911bf1`): `cli`, `com`, `web`
  зависят **только** от `petrovich-core`. Логика должностей живёт в ядре
  (`src/appointment.rs`); COM-адаптер и CLI — тонкие обёртки.
- **Верификация**: `cargo test --locked --offline --workspace` — ядро 11 +
  com 24 + web 9 + golden 2+1 + doctests 2, всё ok;
  `cargo clippy --locked --offline --workspace --all-targets -- -D warnings`
  ok; `cargo fmt --check` ok.
- **COM E2E**: PowerShell 66/66 + VBScript PASS на **x64 и x86** (поведение
  заморожено: DISPID 1–13 append-only, `scode` -1/-2 как у padeg).
- **Тулчейн**: весь workspace на MSVC
  (`rustup override set stable-x86_64-pc-windows-msvc` для каталога;
  глобальный default не тронут). Причины: COM собирается только MSVC;
  релизные бинари в 3–5 раз меньше (petrovich.exe 703 КБ против 2057 КБ
  на gnu); единый тулчейн; разблокирован путь к `windows-sys`-зависимостям.
  gnu-совместимость кода сохраняется, таргет `i686-pc-windows-msvc`
  доставлен для x86.
- **Источник правды по padeg**: дока v4.1 `github.com/rodem/padeg-docs`
  (первична); статья Directum — вторична (примеры + absolute ground truth
  для должностей).

## 1. Оценка по этапам (человеко-дни, разработчик с Rust + AI-ассистент)

| # | Этап | Оценка | Факт |
|---|---|---|---|
| 0 | Discovery / подготовка | 0.5–0.75 | ✅ 2026-09-17 |
| 1 | petrovich-core | 2.5 | ✅ 2026-09-18 |
| 2 | petrovich-cli | 0.25 | ✅ 2026-09-18 (+`appoint` 2026-09-19) |
| 3 | petrovich-web (std-only вместо axum) | 0.5–0.75 | ✅ 2026-09-18 (+`/appoint` 2026-09-19) |
| 4 | petrovich-com | 2.5–3 | ✅ 2026-09-19 (расширен до всего API §7) |
| 5 | Интеграционное тестирование, релиз | 0.5 | ⏳ частично (нет CI, нет тега) |
| | **Итого** | **≈6.5–7.5 дня** | уложились |

## 2. Этап 0. Discovery — ✅ 2026-09-17

- [x] Сверка правил форка (`src/rules.yml`, `src/gender.yml`, коммит
      2020-04-02) с HEAD `petrovich-rules` (df207fbe, 2024-11-15): ~33 строки
      правил фамилий (`-жая`, `рн`, `мец`, `сец`, `ав`, `бек`/`ёк`, `ок`/`ек`,
      firstname-исключения `-ия`) и ~35 строк гендерных эвристик
      (`рауль`/`шамиль`/женские `-ь`, суффиксы `кизи`/`кзы`/`углы`/`угли`,
      `ен` вместо `бен`/`вен`/`ген`/`ден`, `артём`, `карен`). Схема не
      менялась: `test`/`mods`/`tags` те же; YAML — источник, JSON в апстриме
      генерируется из YAML (`rules/Rakefile`), в Rust продолжаем YAML.
- [x] Ruby-эталон: petrovich gem 1.1.5, модификатор `.` = без изменений,
      `-` = счётчик вырезаемых символов — семантика совпадает с кодогенерацией
      `build.rs`. Отличие движка: гендер определяется по каждой части слова
      отдельно, fallback сложнее (`lib/petrovich/gender.rb`) — учтено в этапе 1.
- [x] Golden-данные: `petrovich-eval` submodule (`eval/*.tsv`, ~76k firstname /
      ~12.7k гендерных записей, `lemma<TAB>word<TAB>grammemes`,
      `мр/жр/0/им/рд/дт/вн/тв/пр`); `rake evaluate` в `lib/tasks/evaluate.rake`.
- [x] COM-вопросы закрыты 2026-09-19 через padeg-docs (вместо доступа к машине
      заказчика): первичный ProgID **`PadegUCA.Declension`** (дока v4.1 §5,
      пример 1С) + алиасы `Padeg.Declension`, `Petrovich.Declension`;
      разрядность — **обе** (x64 + x86, битность DLL = битности потребителя);
      ошибки — `EXCEPINFO.scode` = код padeg (`-1` падеж, `-2` род), т.е.
      `EOleException.ErrorCode` совпадает с padeg; побитовая совместимость —
      функциональный эквивалент под своим CLSID, расхождения ловит дифф-харнес
      (§9). Ручного прогона из 1С по-прежнему нет.

## 3. Этап 1. petrovich-core — ✅ 2026-09-18 (+должности 2026-09-19)

- [x] Зависимости: `serde_yaml` (deprecated) → `serde_yaml_ng` 0.10
      (build-dependencies); edition 2024, resolver 3; крейт `petrovich-core`,
      lib name = `petrovich` (`use petrovich::*` совместим).
- [x] Вшитые правила (`build.rs` codegen) побитово = `petrovich-rules`
      df207fbe (2024-11-15).
- [x] **Ruby-parity**: движок против golden-датасета (`eval/*.tsv` →
      `tests/data/`), mismatch-множество Rust == mismatch-множеству эталона
      (petrovich gem через `baseline.rb`; эталон сам даёт 63088/63680
      firstnames, 79927/80025 surnames, 10964/12720 firstnames.gender —
      расхождения «датасет vs правила», не баги движка).
- [x] Порт 1:1: выбор правил — file-order-first + two-pass gender +
      `known=is_last`, теги правил игнорируются (как `@tags = []` в эталоне);
      гендер — per-part голоса, суффиксы longest-first, voting из `gender.rb`.
      Longest-match местами «лучше» эталона (АЖА→АЖИ), но оставлен 1:1 —
      такие строки в allowlist с пометкой etalon-bug.
- [x] Golden-тесты как regression-gate: `tests/data/*.allow.tsv`; тест падает
      при любом новом mismatch И при молча исправленной строке (негативный
      контроль проверен). `midnames`/people — строго 0 расхождений. Harness:
      отдельные `[[test]]` `golden_declension` / `golden_gender`.
- [x] **Должности в ядре** (2026-09-19, `src/appointment.rs`):
      `decline_appointment` / `decline_office` / `merge_appointment` /
      `decline_full_appointment`, семантика — §8. COM-адаптер и CLI поверх них
      — тонкие обёртки.
- [ ] API ядра: оставлен `String` (решение 2026-09-18: `&str`/`Cow<str>` убран
      из блокеров — CLI/web/COM всё равно аллоцируют на границе; вернуться
      по профайлингу).
- [x] Примеры: `examples/decline_fio.rs`, `examples/detect_gender.rs`.

## 4. Этап 2. petrovich-cli — ✅ 2026-09-18 (+`appoint` 2026-09-19)

- [x] `clap`: `decline` (ФИО, падеж, пол auto/male/female) + `gender`.
- [x] `appoint --appointment/--office/--case/--batch` — должности через ядро
      напрямую (`7911bf1`, зависимости `cli → {clap, petrovich-core}`).
- [x] Batch-режим (`--batch file|-` → TSV построчно, stdin).
- [x] Коды выхода 0 ok / 1 runtime / 2 usage (clap), `--help`/`--version`.
- [x] Parity-баг походя: `detect_gender(Some(""))` голосовал Androgynous,
      в Ruby `""` не голосует (`split == []`); добавлен unit-тест.

## 5. Этап 3. petrovich-web — ✅ 2026-09-18 (+`/appoint` 2026-09-19)

Framework: ~~axum~~ → **std-only HTTP** (отклонение 2026-09-18: axum/tokio
тянут `windows-sys`, которому нужен `dlltool.exe` — его не было в gnu-тулчейне;
для GET-роутов под 1С достаточно `TcpListener` + thread-per-connection, ноль
новых зависимостей, офлайн-сборка цела. Вернуться к axum при необходимости —
API роутов не менять).

- [x] `GET /decline`, `/gender`, `/cases` (6 падежей табом), `/health` → `ok`.
- [x] JSON: `/api/v1/decline`, `/api/v1/gender`.
- [x] **Должности** (`1c3c37a`): `/appoint?appointment=&office=&case=` →
      `text/plain`, `/api/v1/appoint` → `{"appointment":"…"}` — через ядро,
      зависимости `web → petrovich-core`.
- [x] `case` = имена/`1`–`6`/`им|рд|…`; `sex` = `male|female|auto`
      (+`m|f|м|ж`). Ошибки — `400 text/plain`, неизвестное — `404`.
- [x] Docker-образ (`web/Dockerfile`, multi-stage) + `web/README.md` с
      примером 1С (+таблица `/appoint`).
- [x] Проверено: 9 unit-тестов роутера + живой сокет-тест побайтово
      (`Иванову Ивану`, UTF-8, `Content-Type: text/plain; charset=utf-8`).
- [ ] Минимизация аллокаций — отложено (см. этап 1).

## 6. Этап 4. petrovich-com — ✅ 2026-09-19 (бывший PLAN-COM.md)

Рамки: начинали как drop-in 4 методов `Declension`, по доке v4.1 расширен
до всего вызываемого API (§7). Плоские C-экспорты padeg.dll с `PChar`-буферами
тоже покрыты (§7.2). Вне скоупа — §7.3.

- **Регистрация без админа**: только **HKCU\Software\Classes** (+ 32-битный вид
  `Wow6432Node` для x86-клиентов) через `com/install-user.ps1` (+ `.cmd`-двойники
  на случай запрета `ps1`); альтернатива вообще без реестра —
  registration-free COM (`com/client.manifest`). Запись в HKCU **затеняет**
  HKLM-запись настоящего padeg для текущего пользователя — это и есть
  механизм drop-in.
- **Сборки**: только MSVC, `x86_64-pc-windows-msvc` + `i686-pc-windows-msvc`
  (на x86 имена экспортов без декорирования — проверено `dumpbin /exports`).
- **Без внешних crate** в COM-слое: весь plumbing вручную на raw FFI
  (`Ole32.lib`/`OleAut32.lib` из SDK).
- **Контракт**: ProgID первичный `PadegUCA.Declension` + `Padeg.Declension` +
  `Petrovich.Declension`; `ThreadingModel = Apartment`; DISPID 1–13
  заморожены, новые методы — только append; `GetSex` → `1/0/-1`;
  `nPadeg ∉ 1..6` → COM-ошибка (`E_INVALIDARG` + `EXCEPINFO`, `scode` -1/-2).
  Свой CLSID (зафиксирован); если код заказчика привязан к CLSID строкой —
  понадобится алиас-ключ (уточнить при внедрении).
- **Ограничения** (документированы в `com/README.md`): `GetNominativePadeg` —
  identity (ядро умеет только именительный→падеж; для штатного потока, где на
  входе уже именительный, — функциональный эквивалент); внешнего словаря
  `Except.dic` нет (правила из `petrovich-rules` — осознанное отличие);
  `GetPadegID` — заглушка с ошибкой в адаптере.
- **Адаптер** (`com/src/adapter.rs`): 4 базовые функции + расширения поверх
  `petrovich-core`; парсинг `"Фамилия Имя Отчество"` (первые 3 слова, остаток
  игнорируется); `SeparateFIO` — запись частей через BYREF (`VT_BYREF`);
  `Sex`: `""`/`auto` = определить, `м*` = муж., `ж*`/`f*` = жен.
  (регистронезависимо, рус/лат); пустое ФИО → пустая строка (не ошибка).
- **Верификация**: PowerShell E2E (`com/test/test_com.ps1`) 66/66 × x64/x86,
  VBScript (`com/test/test_padeg.vbs`) PASS, `cargo test -p petrovich-com`
  (24: адаптер + COM + C-протокол), clippy/fmt чисто, `com/README.md`.

## 7. Весь API padeg: сложность, порядок, дифф-тесты (бывший PLAN-COM-API.md)

Легенда: **S** — часы, чистая обвязка; **M** — дни, эвристика + валидация;
**L** — недели, исследование; **XL** — вне досягаемости без reverse-морфологии.

### 7.1. COM `Padeg.Declension` (late binding — то, что зовёт 1С/VBS)

| # | Метод | Сигнатура COM | Зависимости | Сложн. | Статус |
|---|---|---|---|---|---|
| 1 | `GetSex` | `(FIO) → Integer: 1/0/-1` | `detect_gender` + fallback на отчество | S | ✅ done (-1 = неизвестно) |
| 2 | `GetFIOPadegFS` | `(FIO, Sex, Padeg) → String` | ядро ФИО | S | ✅ done |
| 3 | `GetNominativePadeg` | `(FIO) → String` | reverse-морфология (нет) | XL | ⚠️ identity + документ. |
| 4 | `GetAppointmentPadeg` | `(Appointment, Padeg) → String` | ядро, первое слово | M | ✅ done |
| 5 | `GetOfficePadeg` | `(Office, Padeg) → String` | то же, что 4 | M | ✅ done |
| 6 | `GetFullAppointmentPadeg` | `(Appointment, Office, Padeg) → String` | 4 + 5 + дедуп по стемам | M | ✅ done |
| 7 | `GetFIOPadeg` | `(LN, FN, MN, Sex, Padeg) → String` | адаптер | S | ✅ done, DISPID 7 |
| 8 | `GetIFPadeg`, `GetIFPadegFS` | `(FN, LN, Sex, Padeg)` / `(IF, Sex, Padeg)` | адаптер, правило «последнее слово — фамилия» | S | ✅ done, DISPID 8–9 |
| 9 | `SeparateFIO` | `(FIO, out LN, FN, MN)` — процедура | `split_fio` + запись через BYREF | S | ✅ done, DISPID 10 |
| 10 | Словарь (`SetDictionary`, `Update_Exceptions`, `GetExceptionsFileName`) | `WordBool`/`String` | правила вшиты, словаря нет | S | ✅ done, DISPID 11–13 (честные no-op: `false`/`true`/`""`) |
| 11 | `GetPadegID` | `(FIO) → Integer` | reverse-морфология (нет) | XL | ⛔ заглушка с ошибкой в адаптере; в COM нет (в доке его нет тоже) |
| 12 | Числительные (`NumberToString`, `SumInWords`, `DoubleToVerbal`, `DeclNumeral`, `DeclCurrency`) | числа/словари валют | новый движок морфологии чисел | L | ⛔ следующий этап (не этот план) |

### 7.2. Плоские C-экспорты (`stdcall`, UTF-16 `PChar` — дока v4.1 §4)

✅ Реализованы (`com/src/capi.rs`), поведение — строго по доке: буфер выделяет
вызывающий (`Length(src)+20`, в примерах 255), `nLen` на входе — размер буфера
в символах, на выходе — записано символов (без терминатора), возврат — код
(`0` успех, `-1` падеж, `-2` род, `-3` мал буфер с усечением, `-4`/`-5` —
буферы имени/отчества в `GetFIOParts`). Delphi-пример §10.3 подтверждает
протокол (`Left(tmpS, nLen)` при `ret = 0`). `bSex: Boolean` читаем младшим
байтом (ненулевое = мужской).

Покрыто: `GetFIOPadeg`, `GetFIOPadegAS`, `GetFIOPadegFS`, `GetFIOPadegFSAS`,
`GetIFPadeg`, `GetIFPadegFS`, `GetNominativePadeg`, `GetAppointmentPadeg`,
`GetFullAppointmentPadeg`, `GetOfficePadeg`, `GetSex` (1/0/-1),
`GetFIOParts` (структура `TPartsFIO`, коды -3/-4/-5), `UpdateExceptions`
(`true`, no-op), `GetExceptionsFileName` (`""`), `SetDictionary` (`false`),
`GetPadegID` (`0` = не определён). Unit-тесты буферного протокола в `capi.rs`.

### 7.3. Вне скоупа — числительные, Firebird, словарь (зафиксировано)

- **Числительные/суммы** (`NumberToString`, `SumInWords`, `DoubleToVerbal`,
  `DeclNumeral`, `DeclCurrency` + `Currency.txt`): отдельный движок, L-tier.
- **Firebird UDF** (`PadegFB.dll`, `cdecl`, UTF8/WIN1251): важно для будущего
  DB-endpoint; дифф-харнес DB-агностичен (TSV), вернёмся при endpoint-задаче.
- **Словарь `Except.dic`** (15 секций): у нас правила из `petrovich-rules`,
  внешнего словаря нет — осознанное отличие, задокументировано.
- **Конфиг реестра** `HKEY_CURRENT_CONFIG\Software\Padeg`: нам не нужен
  (нечего настраивать), игнорируем осознанно.

## 8. Как padeg склоняет должности (бывший §4 PLAN-COM-API.md)

Ключевая находка из примеров статьи Directum (absolute ground truth):

- Склоняется **только первое слово**, остальное без изменений:
  `заведующий сектором …` → `заведующего сектором …`;
  `Сектор разработки …` → `Сектора разработки …` (все 6 падежей в статье).
- Составные должности (≥3.3.0.21): разделитель `' - '`, каждая часть со своим
  дефисом (`инженер-конструктор`) склоняется независимо.
- `GetFullAppointmentPadeg`: склейка с удалением дублей
  (`Начальник цеха` + `Цех …` → `Начальник цеха …`).
- Реализация (`src/appointment.rs::decline_head`, COM/CLI/Web — тонкие
  обёртки): head-word + `lastname(head_gender, head, case)` ядра; пол — детект
  ядра, при Androgynous окончание -а/-я (кроме `MASCULINE_A_WORDS`: судья,
  коллега…) → женский, иначе мужской; винительный мужских: -ий/-ый/-ой и
  `ANIMATE_TITLES` (~40 лиц: директор, инженер…) → родительный, иначе
  номинатив (Сектор). Женские -ая дают -ой (норма ядра, как у фамилий).
  Расхождения фиксирует дифф-харнес.

## 9. Дифф-тесты ours-vs-padeg (бывший §5 PLAN-COM-API.md)

Формат датасета (TSV, будущий endpoint БД выгружает именно так):

```text
method\tp1\tp2\tp3\texpected
FIO\tИванов Иван Иванович\t\t3\tИванову Ивану Ивановичу
FIO5\tИванов Иван Иванович\tм\t3\tИванову Ивану Ивановичу
IF\tМарк Твен\tм\t3\tМарку Твену
OFFICE\tСектор разработки\t\t2\tСектора разработки
APPOINT\tзаведующий сектором\t\t3\tзаведующему сектором
FULLAPPOINT\tНачальник цеха\tЦех нестандартного оборудования\t1\tНачальник цеха нестандартного оборудования
SEX\tИванович\t\t\t1
NOMINATIVE\tИванов Иван Иванович\t\t\tИванов Иван Иванович
```

- Методы `PARTS`/`PADEGID`/`DICT` в харнесе пропущены осознанно: у них нет
  COM-поверхности (покрыты cargo-тестами адаптера). Эталонный набор —
  `com/test/dataset.sample.tsv` (ground truth из публикаций padeg).
- `expected` пустой = только сравнение A vs B (дифф), не абсолют.
- Харнес `com/test/difftest.ps1 -Dataset f.tsv -Ours ProgID -Theirs ProgID`:
  late binding к обоим, per-row вызовы, запись mismatches-TSV.
- Метрики: **correctness** — совпадения A vs B и A/B vs expected;
  **single** — среднее мс/вызов (N повторов, медиана); **batch** — сквозная
  пропускная способность (весь датасет × K кругов, записей/сек).
- Без настоящего padeg: `-Theirs` = наш же алиас (self-check: диффов 0,
  валидирует сам харнес) или пропуск B-стороны (baseline ours + замеры).
- `com/test/dataset.sample.tsv` — стартовый набор из примеров статьи
  (абсолютные expected для должностей/офисов — ground truth из публикации).

## 10. Этап 5. Интеграция и релиз — ⏳ частично

- [x] Cargo workspace: `petrovich-core`, `petrovich-cli`, `petrovich-web`,
      `petrovich-com` — единая версия правил, звезда зависимостей (§0).
- [x] README на весь проект (корневой) + `com/README.md` + `web/README.md`:
      сборка, регистрация COM, деплой web.
- [x] Golden-датасет как regression-gate в тестах (этап 1).
- [ ] CI: сборка всех крейтов + тесты + clippy + fmt (нет даже `.github/`).
- [ ] Синк правил: механизм (submodule + CI-check) — golden-gate поймает дрейф
      (тест упадёт с diff), но автоматической проверки свежести нет; вынести
      правила из `src/` в общее место workspace.
- [ ] Тег релиза (публикация в crates.io — при релизе).

## 11. Ключевые риски — итоги

1. **Дрейф правил апстрима** — частично закрыт golden-gate (упадёт с diff);
   осталось: синк-механизм (§10).
2. **ProgID/CLSID** — закрыт: `PadegUCA.Declension` первичный (дока v4.1 §5),
   CLSID свой зафиксирован; риск остался только для кода, привязанного к
   CLSID строкой (алиас-ключ при внедрении).
3. **Разрядность** — закрыт: собираются и тестируются обе (x64 + x86, E2E
   66/66 на каждой).
4. **`EOleException`** — закрыт: `scode` -1/-2 совпадает с padeg.
5. **Ручной прогон из 1С** — открыт: E2E только PowerShell/VBScript.
6. **Числительные** — следующий этап (L-tier, §7.3).
