# PLAN-COM-API.md — весь API padeg: сложность, порядок, дифф-тесты

Источник сигнатур: **дока v4.1 `github.com/rodem/padeg-docs`
(`PADEG_DOCUMENTATION_LLM.md`)** — первична; статья Directum — вторична
(примеры использования + absolute ground truth для должностей).
Легенда сложности:
**S** — часы, чистая обвязка; **M** — дни, нужна эвристика + валидация;
**L** — недели, исследование; **XL** — вне досягаемости без reverse-морфологии.

## 1. Таблица: COM `Padeg.Declension` (late binding, то что зовёт 1С/VBS)

| # | Метод | Сигнатура COM | Зависимости | Сложн. | Статус |
|---|---|---|---|---|---|
| 1 | `GetSex` | `(FIO) → Integer: 1/0/-1` | `detect_gender` + fallback на отчество | S | ✅ done (дока: -1 = неизвестно) |
| 2 | `GetFIOPadegFS` | `(FIO, Sex, Padeg) → String` | ядро ФИО | S | ✅ done |
| 3 | `GetNominativePadeg` | `(FIO) → String` | reverse-морфология (нет) | XL | ⚠️ identity + документ. |
| 4 | `GetAppointmentPadeg` | `(Appointment, Padeg) → String` | ядро, первое слово | M | ✅ done (было identity) |
| 5 | `GetOfficePadeg` | `(Office, Padeg) → String` | то же, что 4 | M | ✅ done |
| 6 | `GetFullAppointmentPadeg` | `(Appointment, Office, Padeg) → String` | 4 + 5 + дедуп по стемам | M | ✅ done |
| 7 | `GetFIOPadeg` | `(LN, FN, MN, Sex, Padeg) → String` | адаптер (есть) | S | ✅ done, DISPID 7 |
| 8 | `GetIFPadeg`, `GetIFPadegFS` | `(FN, LN, Sex, Padeg)` / `(IF, Sex, Padeg)` | адаптер, правило «последнее слово — фамилия» | S | ✅ done, DISPID 8–9 |
| 9 | `SeparateFIO` | `(FIO, out LN, FN, MN)` — процедура | `split_fio` + запись через BYREF | S | ✅ done, DISPID 10 |
| 10 | Словарь (`SetDictionary`, `Update_Exceptions`, `GetExceptionsFileName`) | `WordBool`/`String` | правила вшиты, словаря нет | S | ✅ done, DISPID 11–13 (честные no-op: `false`/`true`/`""`) |
| 11 | `GetPadegID` | `(FIO) → Integer` | reverse-морфология (нет) | XL | ⛔ заглушка с ошибкой в адаптере; в COM нет (в доке его нет тоже) |
| 12 | Числительные (`NumberToString`, `SumInWords`, `DoubleToVerbal`, `DeclNumeral`, `DeclCurrency`) | числа/словари валют | новый движок морфологии чисел | L | ⛔ следующий этап (не этот план) |

DISPID 1–6 заморожены. Новые COM-методы — только append (7–13).
ProgID: первичный **`PadegUCA.Declension`** (дока v4.1 §5, пример 1С),
плюс `Padeg.Declension` (v3/Directum) и `Petrovich.Declension`.
Ошибки COM: `EXCEPINFO.scode` = код padeg (`-1` падеж, `-2` род) —
`EOleException.ErrorCode` совпадает с padeg.

## 2. Плоские C-экспорты (`stdcall`, UTF-16 `PChar` — дока v4.1 §4)

Статус: ✅ реализованы (`com/src/capi.rs`), поведение — строго по доке:
буфер выделяет вызывающий (`Length(src)+20`, в примерах 255), `nLen` на входе —
размер буфера в символах, на выходе — записано символов (без терминатора),
возврат — код (`0` успех, `-1` падеж, `-2` род, `-3` мал буфер с усечением,
`-4`/`-5` — буферы имени/отчества в `GetFIOParts`). Delphi-пример §10.3
подтверждает протокол (`Left(tmpS, nLen)` при `ret = 0`). `bSex: Boolean`
читаем младшим байтом (ненулевое = мужской). На x86 имена экспортов без
декорирования (проверено `dumpbin /exports`) — Delphi-импорт по имени работает.

Покрыто: `GetFIOPadeg`, `GetFIOPadegAS`, `GetFIOPadegFS`, `GetFIOPadegFSAS`,
`GetIFPadeg`, `GetIFPadegFS`, `GetNominativePadeg`, `GetAppointmentPadeg`,
`GetFullAppointmentPadeg`, `GetOfficePadeg`, `GetSex` (1/0/-1),
`GetFIOParts` (структура `TPartsFIO`, коды -3/-4/-5), `UpdateExceptions`
(`true`, no-op), `GetExceptionsFileName` (`""`), `SetDictionary` (`false`),
`GetPadegID` (`0` = не определён). Unit-тесты буферного протокола в `capi.rs`.

## 3. Вне скоупа этого плана — числительные, Firebird, словарь (зафиксировано)

- **Числительные/суммы** (`NumberToString`, `SumInWords`, `DoubleToVerbal`,
  `DeclNumeral`, `DeclCurrency` + `Currency.txt`): отдельный движок, L-tier.
- **Firebird UDF** (`PadegFB.dll`, `cdecl`, UTF8/WIN1251): важно для будущего
  DB-endpoint; дифф-харнес DB-агностичен (TSV), вернёмся при endpoint-задаче.
- **Словарь `Except.dic`** (15 секций): у нас правила из `petrovich-rules`,
  внешнего словаря нет — осознанное отличие, задокументировано.
- **Конфиг реестра** `HKEY_CURRENT_CONFIG\Software\Padeg`: нам не нужен
  (нечего настраивать), игнорируем осознанно.

## 4. Ключевая находка: как padeg склоняет должности (из примеров статьи)

- Склоняется **только первое слово**, остальное без изменений:
  `заведующий сектором …` → `заведующего сектором …`;
  `Сектор разработки …` → `Сектора разработки …` (все 6 падежей в статье).
- Составные должности (≥3.3.0.21): разделитель `' - '`, каждая часть со своим
  дефисом (`инженер-конструктор`) склоняется независимо.
- `GetFullAppointmentPadeg`: склейка с удалением дублей
  (`Начальник цеха` + `Цех …` → `Начальник цеха …`).
- Реализация (`adapter.rs::decline_head`): head-word + `lastname(head_gender,
  head, case)` ядра; пол — детект ядра, при Androgynous окончание -а/-я
  (кроме `MASCULINE_A_WORDS`: судья, коллега…) → женский, иначе мужской;
  винительный мужских: -ий/-ый/-ой и `ANIMATE_TITLES` (~40 лиц: директор,
  инженер…) → родительный, иначе номинатив (Сектор). Женские -ая дают -ой
  (норма ядра, как у фамилий). Расхождения фиксирует дифф-харнес.

## 5. Сценарий дифф-тестов ours-vs-padeg (под DB-датасет)

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

Методы `PARTS`/`PADEGID`/`DICT` в харнесе пропущены осознанно: у них нет
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
