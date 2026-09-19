# petrovich-rs

Склонение русских ФИО по падежам. Rust-порт Ruby-гема
[petrovich](https://github.com/rocsci/petrovich), поведение 1:1 с эталоном
(проверено golden-тестами против эталона на всех датасетах `eval/*.tsv`).

Состав workspace:

| Крейт | Артефакт | Назначение |
|---|---|---|
| `petrovich-core` (корень) | библиотека `petrovich` | склонение + определение пола |
| `cli/` | бинарь `petrovich` | командная строка, batch-режим |
| `web/` | бинарь `petrovich-web` | HTTP-сервис для 1С/VBScript/скриптов |
| `com/` | `petrovich_com.dll` x86+x64 | drop-in замена padeg.dll (`Padeg.Declension`, без админа) |

> Старая версия `petrovich = "0.2"` на crates.io — заброшенный форк.
> Публикация обновлённой версии запланирована на релиз (см. `PLAN.md`, этап 5);
> пока подключайтесь через path внутри workspace.

## Сквозной пример

Одна задача — дательный падеж от «Иванов Иван Иванович» — тремя способами:

```rust
// Библиотека:
use petrovich::{Case, Gender};
let fio = ["Иванов", "Иван", "Иванович"];
assert_eq!(petrovich::lastname(Gender::Male, fio[0], Case::Dative), "Иванову");
```

```sh
# CLI:
petrovich decline --lastname Иванов --firstname Иван --middlename Иванович \
  --case dative --gender male
# -> Иванову Ивану Ивановичу
```

```sh
# Web:
curl "http://127.0.0.1:8080/decline?lastname=Иванов&firstname=Иван&middlename=Иванович&case=dative&sex=male"
# -> Иванову Ивану Ивановичу
```

## Библиотека (`petrovich-core`)

Подключение внутри workspace:

```toml
[dependencies]
petrovich-core = { path = "../petrovich-rs" }
```

```rust
use petrovich::{Case, Gender, detect_gender, firstname, lastname, middlename};

fn main() {
    // Склонение по частям имени: пол влияет на результат.
    assert_eq!(firstname(Gender::Male, "Саша", Case::Dative), "Саше");
    assert_eq!(firstname(Gender::Female, "Изабель", Case::Genitive), "Изабель");

    assert_eq!(
        lastname(Gender::Male, "Станкевич", Case::Prepositional),
        "Станкевиче"
    );
    assert_eq!(
        lastname(Gender::Female, "Станкевич", Case::Prepositional),
        "Станкевич"
    );

    assert_eq!(
        middlename(Gender::Male, "Сергеич", Case::Instrumental),
        "Сергеичем"
    );
    assert_eq!(
        middlename(Gender::Female, "Прокопьевна", Case::Accusative),
        "Прокопьевну"
    );

    // Составные фамилии/имена через дефис склоняются по частям.
    assert_eq!(
        lastname(Gender::Male, "Иванов-Сидоров", Case::Dative),
        "Иванову-Сидорову"
    );
    assert_eq!(
        firstname(Gender::Male, "Илья-Александр", Case::Dative),
        "Илье-Александру"
    );

    // Определение пола по любой комбинации частей (пустые строки игнорируются).
    assert_eq!(detect_gender(None, Some("Александр"), None), Gender::Male);
    assert_eq!(detect_gender(None, Some("Александра"), None), Gender::Female);
    assert_eq!(detect_gender(None, Some("Саша"), None), Gender::Androgynous);
    assert_eq!(
        detect_gender(Some("Иванов"), Some("Саша"), None),
        Gender::Male
    );
    assert_eq!(
        detect_gender(Some("Склифасовская"), Some("Александра"), None),
        Gender::Female
    );
}
```

Падежи (`Case`): `Genitive` (кого? чего?), `Dative` (кому? чему?),
`Accusative` (кого? что?), `Instrumental` (кем? чем?), `Prepositional`
(о ком? о чём?). Именительного нет — это исходная форма.

Пол (`Gender`): `Male`, `Female`, `Androgynous` (не определяется —
имена типа «Саша», несклоняемые фамилии типа «Осипчук»).

## CLI (`petrovich`)

```sh
cargo run -p petrovich-cli -- decline --lastname Иванов --firstname Иван --case dative --gender male
# -> Иванову Ивану

cargo install --path cli   # бинарь `petrovich` в ~/.cargo/bin
```

Команды:

```sh
# Склонение. Пол по умолчанию auto (определяется из переданных частей).
petrovich decline --lastname Иванов --firstname Иван --middlename Иванович \
  --case dative --gender male
# -> Иванову Ивану Ивановичу

petrovich decline --firstname Александра --case genitive
# -> Александры

petrovich decline --lastname Станкевич --case prepositional --gender female
# -> Станкевич

# Только пол:
petrovich gender --firstname Саша
# -> androgynous
petrovich gender --lastname Иванов --firstname Саша
# -> male

# Batch: TSV `фамилия<TAB>имя<TAB>отчество` (пустые поля допустимы).
printf 'Иванов\tИван\tИванович\nСклифасовская\tАлександра\t\n' > names.tsv
petrovich decline --case dative --gender auto --batch names.tsv
# -> Иванову Ивану Ивановичу
# -> Склифасовской Александре

cat names.tsv | petrovich decline --case dative --batch -  # stdin через `-`

# Должности: склоняется первое слово, остальное без изменений.
petrovich appoint --appointment "генеральный директор" --case dative
# -> генеральному директор

petrovich appoint --appointment "заведующий сектором" --case instrumental
# -> заведующим сектором

# Должность + подразделение: склейка без дублей («Цех» не повторяется).
petrovich appoint --appointment "Начальник цеха" \
  --office "Цех нестандартного оборудования" --case genitive
# -> Начальника цеха нестандартного оборудования

# Неодушевлённое в винительном не меняется:
petrovich appoint --appointment "Сектор разработки" --case accusative
# -> Сектор разработки

# Batch для должностей: TSV `должность<TAB>подразделение`.
printf 'генеральный директор\t\nНачальник цеха\tЦех нестандартного оборудования\n' > app.tsv
petrovich appoint --case dative --batch app.tsv
# -> генеральному директор
# -> Начальнику цеха нестандартного оборудования
```

Значения `--case`: `nominative|genitive|dative|accusative|instrumental|prepositional`
(`nominative` возвращает исходную форму).
Значения `--gender`: `auto|male|female|androgynous`.

Коды выхода: `0` — успех; `1` — ошибка выполнения (нет имени, нет файла);
`2` — ошибка аргументов (печатает usage, код самого clap).

## Web (`petrovich-web`)

```sh
cargo run -p petrovich-web -- --bind 127.0.0.1:8080
```

Хост и порт задаются флагом `--bind` (по умолчанию `127.0.0.1:8080`;
`0.0.0.0:8080` — слушать все интерфейсы). Примеры для 1С и VBScript
(`MSXML2.XMLHTTP`) — в [`web/README.md`](web/README.md).

Все маршруты — `GET`, UTF-8. Подробнее — в [`web/README.md`](web/README.md).

```sh
curl "http://127.0.0.1:8080/health"
# -> ok

curl "http://127.0.0.1:8080/decline?lastname=Иванов&firstname=Иван&case=dative&sex=male"
# -> Иванову Ивану

curl "http://127.0.0.1:8080/gender?firstname=Саша"
# -> androgynous

# Все 6 падежей сразу табом (именительный … предложный):
curl "http://127.0.0.1:8080/cases?firstname=Иван&sex=male"
# -> Иван<TAB>Ивана<TAB>Ивану<TAB>Ивана<TAB>Иваном<TAB>Иване

# JSON для не-1С потребителей:
curl "http://127.0.0.1:8080/api/v1/decline?lastname=Иванов&case=3&sex=male"
# -> {"lastname":"Иванову","firstname":"","middlename":""}
curl "http://127.0.0.1:8080/api/v1/gender?firstname=Саша"
# -> {"gender":"androgynous"}
```

Параметры: `case` — имя падежа, `1`–`6` в стиле padeg (`3` = дательный)
или `им|рд|дт|вн|тв|пр`; `sex` — `male|female|auto` (также `m|f|м|ж`,
пусто = `auto`). Хотя бы одна часть имени обязательна, иначе `400`.
Кириллицу в query URL-кодируйте.

Примеры для 1С (`HTTPСоединение`/`HTTPЗапрос`) — только в
[`web/README.md`](web/README.md).

Docker:

```sh
docker build -f web/Dockerfile -t petrovich-web .
docker run --rm -p 8080:8080 petrovich-web
```

## COM (`petrovich-com`)

Drop-in замена `padeg.dll`: `CreateObject("PadegUCA.Declension")`
(+ алиасы `Padeg.Declension`, `Petrovich.Declension`), весь вызываемый API
по доке v4.1 — ФИО, должности/офисы, `SeparateFIO`, словарь-заглушки
(DISPID 1–13) + плоские C-экспорты.
Без прав администратора (регистрация только в HKCU), сборки x86 и x64,
сборка только MSVC-таргетом. Детали, ограничения и установка —
в [`com/README.md`](com/README.md), план — в [`PLAN.md`](PLAN.md) (§6–§7).

## Проверки

```sh
cargo test --locked --workspace        # юниты + golden-тесты паритета с Ruby-эталоном
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo fmt --check
# COM отдельно (только MSVC, оба битности):
cargo test -p petrovich-com --target x86_64-pc-windows-msvc
cargo test -p petrovich-com --target i686-pc-windows-msvc
```

План работ и детали паритета с эталоном — в [`PLAN.md`](PLAN.md).
