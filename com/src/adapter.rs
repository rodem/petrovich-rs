//! Чистая (без Windows API) реализация 4 методов `Padeg.Declension` поверх
//! `petrovich-core`. Собирается и тестируется на любом таргете.
//!
//! Контракт зафиксирован в `PLAN-COM.md` по COM-примеру Directum:
//! `Decl.GetFIOPadegFS(FIO, "", 3)`.

use petrovich::{Case, Gender, detect_gender, firstname, lastname, middlename};

/// Ошибка адаптера; COM-слой отображает её в `HRESULT + EXCEPINFO`.
#[derive(Debug, PartialEq, Eq)]
pub struct PadegError(pub String);

impl std::fmt::Display for PadegError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
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
        other => Err(PadegError(format!(
            "nPadeg должен быть 1..6, получен {other}"
        ))),
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
    Err(PadegError(format!(
        "cSex должен быть пустым/auto, начинаться с м/m или ж/f/w, получено {sex:?}"
    )))
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

/// `GetSex(FIO) -> Integer`: `1` = мужской (для `IIf(Gender,"м","ж")`),
/// `0` = женский или не определён.
pub fn get_sex(fio: &str) -> i32 {
    let (ln, fn_, mn) = split_fio(fio);
    match detect_gender(ln, fn_, mn) {
        Gender::Male => 1,
        _ => 0,
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

/// `GetNominativePadeg(FIO) -> WideString`.
///
/// Ограничение (см. PLAN-COM.md): ядро склоняет только из именительного,
/// обратного склонения нет — возвращается вход без изменений. Для штатного
/// потока (на входе уже именительный) это функциональный эквивалент.
pub fn get_nominative_padeg(fio: &str) -> String {
    fio.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `GetAppointmentPadeg(Appointment, Padeg) -> WideString`.
///
/// Ограничение (см. PLAN-COM.md): в ядре нет правил склонения должностей —
/// возвращается вход без изменений (нормализованные пробелы). Падеж при этом
/// валидируется, неверный `nPadeg` — ошибка, а не молчаливое согласие.
pub fn get_appointment_padeg(appointment: &str, padeg: i32) -> Result<String, PadegError> {
    case_of(padeg)?;
    Ok(appointment.split_whitespace().collect::<Vec<_>>().join(" "))
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
        assert_eq!(get_sex("Иванов Иван Иванович"), 1);
        assert_eq!(get_sex("Петрова Анна Сергеевна"), 0);
        assert_eq!(get_sex("Саша"), 0);
        assert_eq!(get_sex(""), 0);
        assert_eq!(get_sex("   "), 0);
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
    fn nominative_and_appointment_are_identity() {
        assert_eq!(
            get_nominative_padeg("Иванова Ивана Ивановича"),
            "Иванова Ивана Ивановича"
        );
        assert_eq!(
            get_appointment_padeg("генеральный директор", 3).unwrap(),
            "генеральный директор"
        );
        assert_eq!(get_appointment_padeg("", 4).unwrap(), "");
    }
}
