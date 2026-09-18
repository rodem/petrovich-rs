# PLAN.md — Актуализация petrovich-rs + Axum web + COM drop-in для padeg.dll

## Контекст

- Базовая логика склонения: не пишем с нуля, актуализируем заброшенный форк в текущей папке
  (build.rs кодогенерирует правила из YAML/JSON в статические Rust-структуры — сохраняем
  этот подход).
- Источник правды по данным склонения — [petrovich-ruby](https://github.com/petrovich/petrovich-ruby)
  / submodule `petrovich-rules`.
- COM-часть — **не** полный порт padeg.dll v4.1, а drop-in только для 4 методов
  сервера автоматизации `Declension`, задокументированных в readme v4.1
  (studfile.net/preview/13930707): `GetSex`, `GetFIOPadegFS`,
  `GetNominativePadeg`, `GetAppointmentPadeg`.

## Оценка по этапам (человеко-дни, разработчик с Rust + AI-ассистент)

| # | Этап | Оценка |
|---|---|---|
| 0 | Discovery / подготовка | 0.5–0.75 |
| 1 | petrovich-core | 2.5 |
| 2 | petrovich-cli | 0.25 |
| 3 | petrovich-web (axum) | 0.5–0.75 |
| 4 | petrovich-com | 2.5–3 |
| 5 | Интеграционное тестирование, релиз | 0.5 |
| | **Итого** | **≈6.5–7.5 дня** |

---

## Этап 0. Discovery / подготовка (0.5–0.75 дня) — ✅ ЗАВЕРШЁН 2026-09-17 (COM-пункты отложены)

Делается **до** написания кода — влияет на все остальные этапы.

- [x] Сравнить дату/версию правил в форке (`rules.yml`/`gender.yml`) с HEAD
      актуального `petrovich-rules`; зафиксировать diff (см. результаты ниже).
- [x] Проверить, не менялся ли формат правил апстрима (JSON vs YAML, схема
      полей `test`/`mods`/`tags`).
- [ ] Получить реальную копию `padeg.dll`/`PadegUCA.dll`, установленную у
      заказчика (или доступ к машине с ней).
- [ ] Через реестр (`HKCR`) выяснить точный **ProgID и CLSID**, под которым
      существующий код в 1С/VBScript делает `CreateObject(...)`.
- [ ] Уточнить целевую разрядность процесса-потребителя (1С тонкий клиент:
      x86/x64) — собирать нужно то, что реально грузится.
- [ ] Уточнить, обрабатывает ли существующий VB/1С-код `EOleException`
      (`ErrorCode`/`Message`) — если нет, не тратим время на точное
      воспроизведение исключений в этапе 4.
- [ ] Решить: нужна ли 100% побитовая совместимость вывода с padeg.dll, или
      достаточно функционального эквивалента под новым ProgID.

> COM-пункты (padeg.dll, ProgID/CLSID, разрядность, EOleException, побитовая
> совместимость) отложены по решению от 2026-09-17: этап 0 ограничен сверкой
> правил Ruby/Rust. Вернуться к ним перед стартом этапа 4.

### Результаты этапа 0 (2026-09-17, только правила + Rust; COM-пункты отложены)

- Правила в форке (`src/rules.yml`, `src/gender.yml`, коммит 2020-04-02) отстают
  от HEAD `petrovich-rules` (df207fbe, 2024-11-15): ~33 строки правил фамилий
  (`-жая`, `рн`, `мец`, `сец`, `ав`, `бек`/`ёк`, `ок`/`ек`, firstname-исключения
  `-ия`) и ~35 строк гендерных эвристик (`рауль`/`шамиль`/женские `-ь`,
  суффиксы `кизи`/`кзы`/`углы`/`угли`, `ен` вместо `бен`/`вен`/`ген`/`ден`,
  `артём`, `карен`). Схема не менялась: `test`/`mods`/`tags` те же; YAML —
  источник, JSON в апстриме генерируется из YAML (`rules/Rakefile`), так что
  в Rust продолжаем YAML (замена `serde_yaml` — семантически безопасна).
- Ruby-эталон: petrovich gem 1.1.5, формат правил — из YAML (`RuleSet`),
  модификатор `.` = без изменений, `-` = счётчик вырезаемых символов —
  семантика совпадает с Rust-кодогенерацией в `build.rs`.
  Отличие Ruby-движка: gендер определяется по каждой части слова отдельно,
  fallback-логика сложнее (см. `lib/petrovich/gender.rb`) — учесть в этапе 1.
- Golden-данные: `petrovich-eval` submodule (`eval/*.tsv`, ~76k firstname /
  ~12.7k гендерных записей, формат `lemma<TAB>word<TAB>grammemes`,
  граммемы `мр/жр/0/им/рд/дт/вн/тв/пр`); задача `rake evaluate` в
  `lib/tasks/evaluate.rake`. Для Rust — распарсить эти TSV в тесты этапа 1.
- Окружение Rust (Windows): изначально MSVC-линкер `link.exe` отсутствовал →
  использовали `stable-x86_64-pc-windows-gnu`. **Обновление 2026-09-19:**
  VS 2022 BuildTools 17.14 + SDK 10.0.22621 на месте, весь workspace переведён
  на MSVC (`rustup override set stable-x86_64-pc-windows-msvc` для каталога;
  глобальный default не тронут). Причины: COM собирается только MSVC; релизные
  бинари в 3–5 раз меньше (petrovich.exe 703 КБ против 2057 КБ на gnu);
  единый тулчейн вместо двух; разблокирован путь к `windows-sys`-зависимостям
  (clap-color, axum — при необходимости). gnu-совместимость кода сохраняется
  (проверено сборкой), таргет `i686-pc-windows-msvc` доставлен для x86.
- Базлайн-проверки после смены тулчейна: `cargo test` — 7+1 ok;
  `cargo clippy --all-targets -- -D warnings` — ok (поправлены `since = "0.2.0"`
  в deprecated.rs, `contains()` в lib.rs, `truncate(true)` + `&` в build.rs);
  `cargo fmt --check` — ok. Найденные и исправленные предупреждения —
  только lint, поведение не менялось.

## Статус (2026-09-18, этапы 0–3 ✅ без COM)

- **Workspace**: `petrovich-core` (lib `petrovich`), `cli` (бинарь `petrovich`),
  `web` (lib `petrovich_web` + бинарь `petrovich-web`).
- **Верификация**: `cargo test --locked --workspace` — 7 lib + 2 declension +
  1 gender + 7 web + 1 doc, всё ok; `clippy --workspace --all-targets
  -D warnings` ok; `cargo fmt --check` ok.
- Дальше по плану: этап 5 (частично — workspace/CI/README уже по факту есть
  для 3 крейтов), затем COM по отдельному плану.

## Статус (2026-09-17, этап 0 ✅ + начало этапа 1) — архив

- **Environment**: Windows, `stable-x86_64-pc-windows-gnu` (MSVC `link.exe`
  отсутствует, BuildTools не ставятся); cargo test 7+1 ok,
  clippy `--all-targets -D warnings` ok, fmt ok.
- **Deps**: `serde_yaml` → `serde_yaml_ng` 0.10; serde обновлён до 1.0.229.
- **Workspace**: `petrovich-core` (edition 2024, lib name `petrovich`).
- **Правила**: побитово = `petrovich-rules` df207fbe.
- **Поведенческое изменение**: апстрим удалил суффикс `ыч` из гендерных
  эвристик отчеств → `detect_gender("Степаныч")` теперь `Androgynous`
  (тест обновлён; единственное изменённое ожидание).
- **TODO-очередь этапа 1**:
  1. Golden-тесты: скопировать `eval/*.tsv` в `tests/data`, парсер TSV
     (lemma/word/grammemes; граммемы `мр/жр/0/им/рд/дт/вн/тв/пр`),
     интеграционный тест против движка; зафиксировать accuracy.
  2. Исправить подтверждённые расхождения движка.
  3. API ядра: `&str`/`Cow<str>` где возможно.
  4. Затем этап 2 (petrovich-cli) и этап 3 (petrovich-web) параллельно.

---

## Этап 1. petrovich-core (2.5 дня)

Форк `petrovich-rs` как основа, актуализация вместо переписывания.

- [x] Обновить зависимости: `serde_yaml` (deprecated) → `serde_yaml_ng` 0.10
      (build-dependencies; YAML остаётся источником — см. этап 0).
- [x] Поднять `edition` до 2024; workspace resolver 3; крейт переименован в
      `petrovich-core`, lib name = `petrovich` (обратная совместимость
      `use petrovich::*`).
- [x] Синхронизировать вшитые правила (`build.rs` codegen) с актуальным
      `petrovich-rules` df207fbe (2024-11-15) — оба файла побитово,
      `git diff --no-index` чист.
- [x] Прогнать движок против golden-датасета Ruby-версии (`eval/*.tsv`
      → `tests/data/`), зафиксировать расхождения через Ruby-эталон
      (petrovich gem из discovery-копии, скрипт `baseline.rb`).
      Критерий — **Ruby-parity**: mismatch-множество Rust == mismatch-множеству
      эталона (эталон сам даёт 63088/63680 firstnames, 79927/80025 surnames,
      10964/12720 firstnames.gender — это расхождения «датасет vs правила»,
      а не баги движка).
- [x] Исправить подтверждённые расхождения движка портом 1:1 (2026-09-18):
      выбор правил — file-order-first + two-pass gender + `known=is_last`,
      теги правил игнорируются (как `@tags = []` в эталоне); гендер —
      per-part голоса, суффиксы longest-first, voting из `gender.rb`.
      Сюрприз: longest-match местами «лучше» эталона (АЖА→АЖИ), но оставлен
      1:1 — такие строки в allowlist с пометкой etalon-bug.
- [x] Golden-тесты как regression-gate: `tests/data/*.allow.tsv`
      (mismatch-множество эталона); тест падает при любом новом mismatch
      И при молча исправленной строке (негативный контроль проверен).
      `midnames`/people — строго 0 расхождений.
- [ ] API ядра: оставлен `String` (решение 2026-09-18: `&str`/`Cow<str>`
      убран из блокеров — CLI/web/COM всё равно аллоцируют на границе;
      вернуться по профайлингу на этапе 3).
- [x] Опубликовать/зафиксировать `petrovich-core` как отдельный крейт в
      workspace (корневой крейт workspace; публикация в crates.io — при
      релизе, этап 5).
- [x] Harness: отдельные `[[test]]` таргеты `golden_declension` /
      `golden_gender` (убран `autotests = false` + `pub mod` хак).

---

## Этап 2. petrovich-cli (0.25 дня)

Тонкая обвязка поверх `petrovich-core`, идёт параллельно с этапом 3.

- [x] `clap`-интерфейс: `decline` (имя/фамилия/отчество, падеж, пол
      auto/male/female) + `gender` (решение 2026-09-18, крейт `cli/`,
      бинарь `petrovich`).
- [x] Batch-режим (`--batch file|-` → TSV построчно, stdin поддерживается).
- [x] Коды выхода (0 ok / 1 runtime / 2 usage от clap), `--help`/`--version`.
- [x] Походя найден и исправлен parity-баг: `detect_gender(Some(""))`
      голосовал Androgynous, в Ruby `""` не голосует (`split == []`);
      добавлен unit-тест. Примечание: clap без `color` — `windows-sys`
      требует `dlltool.exe`, которого нет в gnu-тулчейне.

---

## Этап 3. petrovich-web — axum (0.5–0.75 дня)

Framework: ~~axum~~ → **std-only HTTP** (отклонение 2026-09-18: axum/tokio
тянут `windows-sys`, которому нужен `dlltool.exe` — его нет в gnu-тулчейне,
WinLibs/MSYS2 ради этого не ставим; для 3 GET-роутов под 1С достаточно
`TcpListener` + thread-per-connection, ноль новых зависимостей, офлайн-сборка
цела. Вернуться к axum, когда появится MSVC/dlltool — API роутов не менять).

- [x] Простые маршруты `text/plain` (крейт `web/`, бинарь `petrovich-web`,
      решение 2026-09-18):
      - `GET /decline?firstname=...&lastname=...&middlename=...&case=...&sex=...`
      - `GET /gender?...`
- [x] Batch-маршрут `GET /cases?...` — 6 падежей табом, nominative…prepositional.
- [x] JSON-роуты `/api/v1/decline`, `/api/v1/gender` для не-1С-потребителей.
- [x] Health-check `/health` → `ok`.
- [x] `case` = имена/`1`–`6`/`им|рд|…`; `sex` = `male|female|auto` (+`m|f|м|ж`).
      Ошибки — `400 text/plain`, неизвестное — `404`.
- [ ] Минимизация аллокаций (`&str`/`Cow<str>`) — отложено (см. этап 1).
- [x] Docker-образ (`web/Dockerfile`, multi-stage) + `web/README.md` с примером 1С.
- [x] Проверено: 7 unit-тестов роутера + живой сокет-тест побайтово
      (`Иванову Ивану`, UTF-8, `Content-Type: text/plain; charset=utf-8`).

---

## Этап 4. petrovich-com — ✅ ВЫПОЛНЕН 2026-09-19 (план: `PLAN-COM.md`)

COM drop-in реализован: крейт `com/` (`petrovich-com`), ручной IDispatch
без внешних crate, сборка только MSVC (`x86_64` + `i686`), регистрация
только HKCU (без админа), все E2E зелёные (PowerShell/VBScript × x64/x86).
Детали, контракт и ловушки — в `PLAN-COM.md` и `com/README.md`.

<details><summary>Старый текст этапа 4 (архив, раскрыть при создании PLAN-COM.md)</summary>

Только 4 метода интерфейса `Declension`, сигнатуры — из документации padeg.dll
v4.1 (не реверс-инжиниринг):

```pascal
function GetSex(const cMiddleName: WideString): Integer; stdcall;
function GetFIOPadegFS(const cFIO, cSex: WideString; nPadeg: Integer): WideString; safecall;
function GetNominativePadeg(const cFIO: WideString): WideString; safecall;
function GetAppointmentPadeg(const cAppointment: WideString; nPadeg: Integer): WideString; safecall;
```

Шаги:

- [ ] Подтвердить точный ProgID/CLSID (см. этап 0) — под этим именем клиентский
      код должен находить объект.
- [ ] Выбрать инструмент реализации IDispatch-сервера: `windows-rs`
      (`#[implement]`) или крейт `intercom` (заточен под VB/VBScript-COM).
- [ ] Реализовать `IDispatch::GetIDsOfNames`/`Invoke` для 4 методов;
      маршалинг `BSTR` ↔ `String`, `Integer`, строковый параметр «пол»
      (`''`/`'м'`/`'ж'`, регистронезависимо).
- [ ] Реализация `GetAppointmentPadeg` (используется дважды в примере — падежи
      3 и 4) — тонкая обвязка над той же логикой склонения, что и ФИО, но без
      персональных грамматических правил (должность как отдельная категория
      правил).
- [ ] Обработка ошибок: неверный падеж/род → HRESULT-ошибка с текстом (или
      просто исходная строка без изменений — см. решение из этапа 0 про
      `EOleException`).
- [ ] `DllRegisterServer`/`DllUnregisterServer` (self-registration), запись
      `CLSID`, `ProgID`, `InprocServer32`, `ThreadingModel`.
- [ ] Сборка **и под x86, и под x64** (если по этапу 0 подтверждена
      необходимость обеих).
- [ ] Тестирование ровно по сценариям из псевдокода:
      ```
      FullNameNominative = Padeg.GetNominativePadeg(FIO)
      Gender = Padeg.GetSex(FullNameNominative)
      Padeg.GetFIOPadegFS(FullNameNominative, IIf(Gender, "м", "ж"), 3)
      Padeg.GetFIOPadegFS(FullNameNominative, IIf(Gender, "м", "ж"), 4)
      Padeg.GetAppointmentPadeg(Post, 3)
      Padeg.GetAppointmentPadeg(Post, 4)
      ```
      — прогнать из VBScript, из 1С (толстый/тонкий клиент), при наличии — из
      VBA.
- [ ] Установочный скрипт (`regsvr32` / `.reg`) и короткая инструкция для
      1С-программистов.

</details>

---

## Этап 5. Интеграция и релиз (0.5 дня)

- [ ] Cargo workspace: `petrovich-core`, `petrovich-cli`, `petrovich-web`
      (COM — отдельно, см. этап 4) — единая версия правил, общий CI.
- [ ] CI: сборка всех крейтов, прогон golden-датасета как regression-теста.
- [ ] Финальный README на весь проект: что это, как собрать, как
      зарегистрировать COM-компонент, как задеплоить web-сервис.
- [ ] Тег релиза.

---

## Ключевые риски (не влияют на порядок, но влияют на срок)

1. **Дрейф правил апстрима** — частично закрыт: golden-gate на allowlist
   поймает любое изменение поведения при синке `rules.yml`/`gender.yml`
   (тест упадёт с diff). Осталось: механизм синка (submodule + CI-check)
   вынести правила из `src/` в общее место workspace при этапе 5.
2. **ProgID/CLSID не подтверждён до старта этапа 4** — без реальной DLL или
   доступа к реестру заказчика этап 4 нельзя тестировать по-настоящему; в
   этом случае делаем реализацию, но верификацию откладываем до получения
   доступа.
3. **Разрядность 1С-клиента** — если нужны и x86, и x64, а не что-то одно,
   к этапу 4 добавляется тестовый прогон на обеих сборках.
4. **Точность воспроизведения `EOleException`** — если выясняется (этап 0),
   что обработка ошибок в клиентском коде всё же есть, нужно закладывать
   дополнительное время на этап 4 (см. таблицу выше, верхняя граница 3 дня
   уже это учитывает).
