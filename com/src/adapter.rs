//! Чистая (без Windows API) реализация 4 методов `Padeg.Declension` поверх
//! `petrovich-core`. Собирается и тестируется на любом таргете.
//!
//! Контракт зафиксирован в `PLAN-COM.md` по COM-примеру Directum:
//! `Decl.GetFIOPadegFS(FIO, "", 3)`.

use petrovich::{Case, Gender, detect_gender, firstname, lastname, middlename};

/// Коды возврата padeg (дока v4.1, §2): 0 успех, -1 плохой падеж,
/// -2 плохой род, -3 мал буфер (-4/-5 — буферы GetFIOParts).
pub const CODE_OK: i32 = 0;
pub const CODE_BAD_PADEG: i32 = -1;
pub const CODE_BAD_SEX: i32 = -2;
pub const CODE_SMALL_BUFFER: i32 = -3;

/// Ошибка адаптера; COM-слой отображает `code` в `EXCEPINFO.scode`
/// (EOleException.ErrorCode у padeg равен этим же кодам), C-экспорты
/// возвращают `code` напрямую.
#[derive(Debug, PartialEq, Eq)]
pub struct PadegError {
    pub code: i32,
    pub message: String,
}

impl PadegError {
    fn bad_padeg(padeg: i32) -> Self {
        Self {
            code: CODE_BAD_PADEG,
            message: format!("nPadeg должен быть 1..6, получен {padeg}"),
        }
    }

    fn bad_sex(sex: &str) -> Self {
        Self {
            code: CODE_BAD_SEX,
            message: format!(
                "cSex должен быть пустым/auto, начинаться с м/m или ж/f/w, получено {sex:?}"
            ),
        }
    }
}

impl std::fmt::Display for PadegError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for PadegError {}

/// `nPadeg` padeg (1–6) → падеж ядра. `1` (именительный) = без изменений.
pub fn case_of(padeg: i32) -> Result<Option<Case>, PadegError> {
    match padeg {
        1 => Ok(None),
        2 => Ok(Some(Case::Genitive)),
        3 => Ok(Some(Case::Dative)),
        4 => Ok(Some(Case::Accusative)),
        5 => Ok(Some(Case::Instrumental)),
        6 => Ok(Some(Case::Prepositional)),
        other => Err(PadegError::bad_padeg(other)),
    }
}

/// Параметр пола padeg: `""`/`auto` = определить, иначе первая буква
/// (`м*`/`m*` — мужской, `ж*`/`f*`/`w*` — женский), регистронезависимо.
pub fn sex_of(sex: &str) -> Result<Option<Gender>, PadegError> {
    let normalized: String = sex.trim().to_lowercase();
    if normalized.is_empty() || normalized == "auto" || normalized == "авто" {
        return Ok(None);
    }
    if let Some(first) = normalized.chars().next() {
        if first == 'м' || first == 'm' {
            return Ok(Some(Gender::Male));
        }
        if first == 'ж' || first == 'f' || first == 'w' {
            return Ok(Some(Gender::Female));
        }
    }
    Err(PadegError::bad_sex(sex))
}

/// Разбор `"Фамилия Имя Отчество"` позиционно слева направо.
/// Пустой вход → все части отсутствуют (не ошибка).
/// Слов больше трёх → лишние справа игнорируются (документ. в PLAN-COM.md).
pub fn split_fio(fio: &str) -> (Option<&str>, Option<&str>, Option<&str>) {
    let mut words = fio.split_whitespace();
    (
        words.next().filter(|w| !w.is_empty()),
        words.next().filter(|w| !w.is_empty()),
        words.next().filter(|w| !w.is_empty()),
    )
}

/// `GetSex(FIO) -> Integer` (дока v4.1 §4.4): `1` = мужской, `0` = женский,
/// `-1` = невозможно определить. Для `IIf(Gender,"м","ж")` ненулевой = мужской.
/// Дока: пол определяется по отчеству — если позиционный детект не решил,
/// последнее слово пробуем как отчество (`Иванович` лежит в слоте фамилии).
pub fn get_sex(fio: &str) -> i32 {
    let (ln, fn_, mn) = split_fio(fio);
    match detect_gender(ln, fn_, mn) {
        Gender::Male => 1,
        Gender::Female => 0,
        Gender::Androgynous => match mn.or(fn_).or(ln) {
            Some(w) => match detect_gender(None, None, Some(w)) {
                Gender::Male => 1,
                Gender::Female => 0,
                Gender::Androgynous => -1,
            },
            None => -1,
        },
    }
}

/// `GetFIOPadegFS(FIO, Sex, Padeg) -> WideString`.
pub fn get_fio_padeg_fs(fio: &str, sex: &str, padeg: i32) -> Result<String, PadegError> {
    let case = case_of(padeg)?;
    let explicit = sex_of(sex)?;
    let (ln, fn_, mn) = split_fio(fio);
    if ln.is_none() && fn_.is_none() && mn.is_none() {
        return Ok(String::new());
    }
    let gender = explicit.unwrap_or_else(|| detect_gender(ln, fn_, mn));
    let mut out: Vec<String> = Vec::with_capacity(3);
    if let Some(ln) = ln {
        out.push(match case {
            Some(case) => lastname(gender, ln, case),
            None => ln.to_owned(),
        });
    }
    if let Some(fn_) = fn_ {
        out.push(match case {
            Some(case) => firstname(gender, fn_, case),
            None => fn_.to_owned(),
        });
    }
    if let Some(mn) = mn {
        out.push(match case {
            Some(case) => middlename(gender, mn, case),
            None => mn.to_owned(),
        });
    }
    Ok(out.join(" "))
}

/// Пустая (после trim) часть имени отсутствует, а не склоняется.
fn norm_part(s: &str) -> Option<&str> {
    let t = s.trim();
    if t.is_empty() { None } else { Some(t) }
}

/// `GetFIOPadeg(LN, FN, MN, Sex, Padeg)`: части по трём строкам.
/// `GetFIOPadegAS(...)`: то же с автоопределением рода (`AS` = auto-sex).
pub fn get_fio_padeg(
    lastname: &str,
    firstname: &str,
    middlename: &str,
    sex: &str,
    padeg: i32,
) -> Result<String, PadegError> {
    let case = case_of(padeg)?;
    let explicit = sex_of(sex)?;
    let (ln, fn_, mn) = (
        norm_part(lastname),
        norm_part(firstname),
        norm_part(middlename),
    );
    if ln.is_none() && fn_.is_none() && mn.is_none() {
        return Ok(String::new());
    }
    let gender = explicit.unwrap_or_else(|| detect_gender(ln, fn_, mn));
    let mut out: Vec<String> = Vec::with_capacity(3);
    for (part, inflect) in [
        (ln, petrovich::lastname as fn(Gender, &str, Case) -> String),
        (
            fn_,
            petrovich::firstname as fn(Gender, &str, Case) -> String,
        ),
        (
            mn,
            petrovich::middlename as fn(Gender, &str, Case) -> String,
        ),
    ] {
        if let Some(part) = part {
            out.push(match case {
                Some(case) => inflect(gender, part, case),
                None => part.to_owned(),
            });
        }
    }
    Ok(out.join(" "))
}

/// `GetIFPadeg(FN, LN, Sex, Padeg)` / `GetIFPadegFS("Имя Фамилия", ...)`:
/// порядок «имя фамилия» (наоборот от FIO).
pub fn get_if_padeg(
    firstname: &str,
    lastname: &str,
    sex: &str,
    padeg: i32,
) -> Result<String, PadegError> {
    let case = case_of(padeg)?;
    let explicit = sex_of(sex)?;
    let (fn_, ln) = (norm_part(firstname), norm_part(lastname));
    if fn_.is_none() && ln.is_none() {
        return Ok(String::new());
    }
    // Детект в каноническом порядке (фамилия, имя).
    let gender = explicit.unwrap_or_else(|| detect_gender(ln, fn_, None));
    let mut out: Vec<String> = Vec::with_capacity(2);
    if let Some(fn_) = fn_ {
        out.push(match case {
            Some(case) => petrovich::firstname(gender, fn_, case),
            None => fn_.to_owned(),
        });
    }
    if let Some(ln) = ln {
        out.push(match case {
            Some(case) => petrovich::lastname(gender, ln, case),
            None => ln.to_owned(),
        });
    }
    Ok(out.join(" "))
}

/// `GetIFPadegFS("Имя Фамилия", Sex, Padeg)`.
/// Правило доки v4.1: последнее слово — фамилия, все предшествующие — имена
/// («Джон Фиджеральд Кеннеди» → имя «Джон Фиджеральд», фамилия «Кеннеди»).
pub fn get_if_padeg_fs(if_: &str, sex: &str, padeg: i32) -> Result<String, PadegError> {
    let words: Vec<&str> = if_.split_whitespace().collect();
    let (firstname, lastname) = match words.as_slice() {
        [] => ("", ""),
        [first] => (*first, ""),
        _ => (
            // Имена через пробел; ядро склоняет только первое — остальное допишем.
            words[0],
            words[words.len() - 1],
        ),
    };
    let mut out = get_if_padeg(firstname, lastname, sex, padeg)?;
    if words.len() > 2 {
        // Средние имена padeg не трогает как отдельные части: вставляем как есть
        // между склонёнными первым именем и фамилией.
        let middle = words[1..words.len() - 1].join(" ");
        let parts: Vec<&str> = out.splitn(2, ' ').collect();
        if parts.len() == 2 {
            out = format!("{} {} {}", parts[0], middle, parts[1]);
        } else {
            out = format!("{out} {middle}");
        }
    }
    Ok(out)
}

/// `GetFIOParts(FIO) -> (LN, FN, MN)`: разбивка на составляющие.
pub fn get_fio_parts(fio: &str) -> (String, String, String) {
    let (ln, fn_, mn) = split_fio(fio);
    (
        ln.unwrap_or("").to_owned(),
        fn_.unwrap_or("").to_owned(),
        mn.unwrap_or("").to_owned(),
    )
}

/// `GetPadegID(FIO)`: номер падежа, в котором записано ФИО.
///
/// ⛔ Заглушка: требует обратной морфологии (её нет в ядре — см. PLAN-COM-API.md).
/// Возвращает ошибку, а не выдуманный номер.
pub fn get_padeg_id(_fio: &str) -> Result<i32, PadegError> {
    Err(PadegError {
        code: CODE_BAD_PADEG,
        message: "GetPadegID не поддерживается: нужно обратное склонение (см. PLAN-COM-API.md)"
            .to_owned(),
    })
}

/// `UpdateExceptions`: словаря нет (правила вшиты) — нечего перечитывать, успех.
pub fn update_exceptions() -> bool {
    true
}

/// `GetExceptionsFileName`: внешнего словаря нет — пустая строка.
pub fn get_exceptions_file_name() -> String {
    String::new()
}

/// `SetDictionary(path)`: внешний словарь не поддерживается.
pub fn set_dictionary(_path: &str) -> bool {
    false
}

// ---------------------------------------------------------------------------
// Должности и подразделения (M-tier, находки из примеров статьи Directum)
// ---------------------------------------------------------------------------

/// Мужские существительные на согласную, обозначающие лицо (одушевлённые),
/// чей винительный = родительному («вижу директора»). Точного списка у нас нет
/// (нужен словарь padeg), поэтому ядро HR-должностей — эвристика с тестами.
/// Остальные слова на согласную считаем неодушевлёнными (аккузатив = номинатив).
const ANIMATE_TITLES: &[&str] = &[
    "директор",
    "менеджер",
    "инженер",
    "бухгалтер",
    "секретарь",
    "заместитель",
    "руководитель",
    "начальник",
    "специалист",
    "продавец",
    "врач",
    "учитель",
    "мастер",
    "редактор",
    "конструктор",
    "технолог",
    "экономист",
    "юрист",
    "водитель",
    "охранник",
    "курьер",
    "агент",
    "кассир",
    "кладовщик",
    "грузчик",
    "слесарь",
    "токарь",
    "повар",
    "официант",
    "администратор",
    "оператор",
    "программист",
    "дизайнер",
    "тренер",
    "доктор",
    "профессор",
    "студент",
    "клиент",
    "пациент",
    "свидетель",
];

/// Одушевлённость head-слова для винительного падежа: прилагательные на
/// -ий/-ый/-ой (заведующий, генеральный) + лица из `ANIMATE_TITLES`.
/// Остальные мужские существительные — неодушевлённые (аккузатив = номинатив:
/// «Сектор»). Женский род склоняется без различий по одушевлённости.
fn is_animate_head(head_lower: &str) -> bool {
    head_lower.ends_with("ий")
        || head_lower.ends_with("ый")
        || head_lower.ends_with("ой")
        || ANIMATE_TITLES.contains(&head_lower)
}

/// Слова общего рода на -а/-я, которые в должности по умолчанию мужские
/// («судья вынес решение»). Без списка ушли бы в женский по окончанию.
const MASCULINE_A_WORDS: &[&str] = &[
    "судья",
    "коллега",
    "староста",
    "сирота",
    "папа",
    "дядя",
    "дедушка",
    "мужчина",
    "юноша",
];

/// Пол head-слова: детект ядра, при Androgynous — по окончанию (-а/-я, кроме
/// `MASCULINE_A_WORDS`, → женский; иначе мужской). Ядро эвристик common nouns
/// не знает (`начальница`, `медсестра` дают Androgynous), без fallback ушли бы
/// в мужские правила с мусором на выходе.
fn head_gender(word: &str) -> Gender {
    match detect_gender(Some(word), None, None) {
        Gender::Androgynous => {
            let lower = word.to_lowercase();
            if MASCULINE_A_WORDS.contains(&lower.as_str()) {
                Gender::Male
            } else if lower.ends_with('а') || lower.ends_with('я') {
                Gender::Female
            } else {
                Gender::Male
            }
        }
        g => g,
    }
}

/// Склонение одного head-слова должности/подразделения.
fn decline_head(word: &str, case: Case) -> String {
    let gender = head_gender(word);
    if case == Case::Accusative && gender == Gender::Male && !is_animate_head(&word.to_lowercase())
    {
        return word.to_owned();
    }
    lastname(gender, word, case)
}

/// Склонение составной строки: сегменты через `' - '`, в каждом склоняется
/// только первое слово (подтверждено примерами статьи), остальное без изменений.
fn decline_appointment_text(text: &str, case: Option<Case>) -> String {
    text.split(" - ")
        .map(|segment| {
            let mut words = segment.split_whitespace();
            match words.next() {
                None => String::new(),
                Some(head) => {
                    let mut out = vec![match case {
                        Some(case) => decline_head(head, case),
                        None => head.to_owned(),
                    }];
                    out.extend(words.map(str::to_owned));
                    out.join(" ")
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" - ")
}

/// `GetAppointmentPadeg(Appointment, Padeg) -> WideString`.
///
/// Склоняет первое слово каждого `' - '`-сегмента мужскими/авто правилами ядра,
/// остальное без изменений (см. PLAN-COM-API.md §3).
pub fn get_appointment_padeg(appointment: &str, padeg: i32) -> Result<String, PadegError> {
    let case = case_of(padeg)?;
    Ok(decline_appointment_text(appointment, case))
}

/// `GetOfficePadeg(Office, Padeg)`: та же логика, что должности
/// (пример статьи: `Сектор …` → `Сектора …`).
pub fn get_office_padeg(office: &str, padeg: i32) -> Result<String, PadegError> {
    let case = case_of(padeg)?;
    Ok(decline_appointment_text(office, case))
}

/// Нормализация слова для дедупликации: нижний регистр + срезанная конечная
/// гласная (`цеха`/`Цех` → `цех`). Приближение стемминга padeg, расхождения —
/// в дифф-харнес.
fn stem_word(word: &str) -> String {
    let lower = word.to_lowercase();
    lower
        .strip_suffix(|c: char| "аеёиоуыэюя".contains(c))
        .unwrap_or(&lower)
        .to_owned()
}

/// `GetFullAppointmentPadeg(Appointment, Office, Padeg)`: склейка с удалением
/// слов офиса, уже есть в должности (сравнение по стемам, регистронезависимо),
/// затем склонение как должность: `Начальник цеха` + `Цех …` → без дубля `Цех`.
pub fn get_full_appointment_padeg(
    appointment: &str,
    office: &str,
    padeg: i32,
) -> Result<String, PadegError> {
    let case = case_of(padeg)?;
    let app_words: Vec<&str> = appointment.split_whitespace().collect();
    let stems: std::collections::HashSet<String> = app_words.iter().map(|w| stem_word(w)).collect();
    let mut merged: Vec<&str> = app_words;
    for word in office.split_whitespace() {
        if !stems.contains(&stem_word(word)) {
            merged.push(word);
        }
    }
    Ok(decline_appointment_text(&merged.join(" "), case))
}

/// `GetNominativePadeg(FIO) -> WideString`.
///
/// Ограничение (см. PLAN-COM.md): ядро склоняет только из именительного,
/// обратного склонения нет — возвращается вход без изменений. Для штатного
/// потока (на входе уже именительный) это функциональный эквивалент.
pub fn get_nominative_padeg(fio: &str) -> String {
    fio.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn padeg_numbers_map_to_cases() {
        assert_eq!(case_of(1).unwrap(), None);
        assert_eq!(case_of(2).unwrap(), Some(Case::Genitive));
        assert_eq!(case_of(3).unwrap(), Some(Case::Dative));
        assert_eq!(case_of(4).unwrap(), Some(Case::Accusative));
        assert_eq!(case_of(5).unwrap(), Some(Case::Instrumental));
        assert_eq!(case_of(6).unwrap(), Some(Case::Prepositional));
        assert!(case_of(0).is_err());
        assert!(case_of(7).is_err());
        assert!(case_of(-1).is_err());
    }

    #[test]
    fn sex_parsing_rules() {
        assert_eq!(sex_of("").unwrap(), None);
        assert_eq!(sex_of("  ").unwrap(), None);
        assert_eq!(sex_of("auto").unwrap(), None);
        assert_eq!(sex_of("м").unwrap(), Some(Gender::Male));
        assert_eq!(sex_of("М").unwrap(), Some(Gender::Male));
        assert_eq!(sex_of("муж").unwrap(), Some(Gender::Male));
        assert_eq!(sex_of("male").unwrap(), Some(Gender::Male));
        assert_eq!(sex_of("ж").unwrap(), Some(Gender::Female));
        assert_eq!(sex_of("Ж").unwrap(), Some(Gender::Female));
        assert_eq!(sex_of("female").unwrap(), Some(Gender::Female));
        assert!(sex_of("x").is_err());
        assert!(sex_of("1").is_err());
    }

    #[test]
    fn directum_scenario_male() {
        let fio = "Иванов Иван Иванович";
        assert_eq!(get_fio_padeg_fs(fio, "", 1).unwrap(), fio);
        assert_eq!(
            get_fio_padeg_fs(fio, "", 3).unwrap(),
            "Иванову Ивану Ивановичу"
        );
        assert_eq!(
            get_fio_padeg_fs(fio, "м", 4).unwrap(),
            "Иванова Ивана Ивановича"
        );
        assert_eq!(
            get_fio_padeg_fs(fio, "", 5).unwrap(),
            "Ивановым Иваном Ивановичем"
        );
    }

    #[test]
    fn female_declension() {
        let fio = "Петрова Анна Сергеевна";
        assert_eq!(
            get_fio_padeg_fs(fio, "ж", 3).unwrap(),
            "Петровой Анне Сергеевне"
        );
        assert_eq!(
            get_fio_padeg_fs(fio, "", 2).unwrap(),
            "Петровой Анны Сергеевны"
        );
    }

    #[test]
    fn get_sex_values() {
        // Дока v4.1 §4.4: 1 мужской, 0 женский, -1 неизвестно.
        assert_eq!(get_sex("Иванов Иван Иванович"), 1);
        assert_eq!(get_sex("Иванович"), 1);
        assert_eq!(get_sex("Петрова Анна Сергеевна"), 0);
        assert_eq!(get_sex("Саша"), -1);
        assert_eq!(get_sex(""), -1);
        assert_eq!(get_sex("   "), -1);
    }

    #[test]
    fn empty_and_partial_fio() {
        assert_eq!(get_fio_padeg_fs("", "", 3).unwrap(), "");
        assert_eq!(get_fio_padeg_fs("   ", "м", 2).unwrap(), "");
        assert_eq!(get_fio_padeg_fs("Иванов", "м", 3).unwrap(), "Иванову");
        assert_eq!(
            get_fio_padeg_fs("Иванов Иван", "", 3).unwrap(),
            "Иванову Ивану"
        );
        // Лишние слова справа игнорируются.
        assert_eq!(
            get_fio_padeg_fs("Иванов Иван Иванович лишний", "", 3).unwrap(),
            "Иванову Ивану Ивановичу"
        );
        // Пробелы нормализуются.
        assert_eq!(
            get_fio_padeg_fs("  Иванов   Иван  ", "", 1).unwrap(),
            "Иванов Иван"
        );
    }

    #[test]
    fn errors_propagate() {
        assert!(get_fio_padeg_fs("Иванов Иван", "", 0).is_err());
        assert!(get_fio_padeg_fs("Иванов Иван", "", 7).is_err());
        assert!(get_fio_padeg_fs("Иванов Иван", "x", 3).is_err());
        assert!(get_appointment_padeg("директор", 9).is_err());
    }

    #[test]
    fn nominative_is_identity() {
        assert_eq!(
            get_nominative_padeg("Иванова Ивана Ивановича"),
            "Иванова Ивана Ивановича"
        );
    }

    #[test]
    fn appointment_first_word_only_article_golden() {
        // Примеры из статьи Directum (все 6 падежей).
        let dolg = "заведующий сектором";
        assert_eq!(get_appointment_padeg(dolg, 1).unwrap(), dolg);
        assert_eq!(
            get_appointment_padeg(dolg, 2).unwrap(),
            "заведующего сектором"
        );
        assert_eq!(
            get_appointment_padeg(dolg, 3).unwrap(),
            "заведующему сектором"
        );
        assert_eq!(
            get_appointment_padeg(dolg, 4).unwrap(),
            "заведующего сектором"
        );
        assert_eq!(
            get_appointment_padeg(dolg, 5).unwrap(),
            "заведующим сектором"
        );
        assert_eq!(
            get_appointment_padeg(dolg, 6).unwrap(),
            "заведующем сектором"
        );
        // Офис: неодушевлённое — винительный = именительный.
        assert_eq!(get_office_padeg("Сектор", 1).unwrap(), "Сектор");
        assert_eq!(get_office_padeg("Сектор", 2).unwrap(), "Сектора");
        assert_eq!(get_office_padeg("Сектор", 4).unwrap(), "Сектор");
        assert_eq!(get_office_padeg("Сектор", 5).unwrap(), "Сектором");
        assert_eq!(get_appointment_padeg("", 4).unwrap(), "");
        assert!(get_appointment_padeg("директор", 9).is_err());
    }

    #[test]
    fn appointment_feminine_and_animate() {
        // Женские head-слова: ядро детектит Female или fallback по -а/-я.
        assert_eq!(
            get_appointment_padeg("начальница цеха", 2).unwrap(),
            "начальницы цеха"
        );
        assert_eq!(get_appointment_padeg("медсестра", 3).unwrap(), "медсестре");
        // Лица на согласную: винительный = родительному.
        assert_eq!(
            get_appointment_padeg("директор завода", 4).unwrap(),
            "директора завода"
        );
        assert_eq!(get_appointment_padeg("инженер", 4).unwrap(), "инженера");
        // Неодушевлённые: винительный = именительный.
        assert_eq!(
            get_appointment_padeg("Сектор разработки", 4).unwrap(),
            "Сектор разработки"
        );
        // Общий род в должности по умолчанию мужской.
        assert_eq!(get_appointment_padeg("судья", 2).unwrap(), "судьи");
    }

    #[test]
    fn compound_appointment_dash_separator() {
        assert_eq!(
            get_appointment_padeg("инженер - конструктор", 2).unwrap(),
            "инженера - конструктора"
        );
    }

    #[test]
    fn full_appointment_merges_without_dupes() {
        // Пример статьи: «Цех» не дублируется.
        assert_eq!(
            get_full_appointment_padeg("Начальник цеха", "Цех нестандартного оборудования", 1)
                .unwrap(),
            "Начальник цеха нестандартного оборудования"
        );
        // Склоняется только первое слово склейки, дубли выкинуты.
        assert_eq!(
            get_full_appointment_padeg("директор", "дирекция", 2).unwrap(),
            "директора дирекция"
        );
    }

    #[test]
    fn if_order_variants() {
        assert_eq!(
            get_if_padeg("Иван", "Иванов", "м", 3).unwrap(),
            "Ивану Иванову"
        );
        assert_eq!(
            get_if_padeg_fs("Иван Иванов", "", 2).unwrap(),
            "Ивана Иванова"
        );
        assert_eq!(get_if_padeg("", "", "", 3).unwrap(), "");
        assert_eq!(
            get_fio_padeg("Иванов", "Иван", "Иванович", "auto", 3).unwrap(),
            "Иванову Ивану Ивановичу"
        );
    }

    #[test]
    fn fio_parts_split() {
        assert_eq!(
            get_fio_parts("Иванов Иван Иванович"),
            (
                "Иванов".to_owned(),
                "Иван".to_owned(),
                "Иванович".to_owned()
            )
        );
        assert_eq!(
            get_fio_parts("  Петрова  Анна "),
            ("Петрова".to_owned(), "Анна".to_owned(), String::new())
        );
    }

    #[test]
    fn dictionary_and_padegid_stubs() {
        assert!(update_exceptions());
        assert_eq!(get_exceptions_file_name(), "");
        assert!(!set_dictionary("C:\\dict.dic"));
        assert!(get_padeg_id("Иванову Ивану Ивановичу").is_err());
    }
}
