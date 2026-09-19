//! Склонение должностей и подразделений.
//!
//! Находки из примеров padeg (см. PLAN-COM-API.md §4): склоняется **только
//! первое слово** каждого `' - '`-сегмента, остальное без изменений; составные
//! должности делятся разделителем `' - '`; склейка «должность + подразделение»
//! удаляет дублирующиеся слова.
//!
//! Используется и CLI (`appoint`), и COM-адаптером — зависимости строго
//! `cli → lib`, `com → lib`, длинных цепочек нет.

use std::collections::HashSet;

use crate::{Case, Gender, detect_gender, lastname};

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
/// только первое слово, остальное без изменений. `None` = именительный.
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

/// Склонение должности (`None` = именительный, без изменений).
///
/// # Examples
///
/// ```
/// use petrovich::{Case, decline_appointment};
///
/// assert_eq!(
///     decline_appointment("заведующий сектором", Some(Case::Dative)),
///     "заведующему сектором"
/// );
/// ```
pub fn decline_appointment(appointment: &str, case: Option<Case>) -> String {
    decline_appointment_text(appointment, case)
}

/// Склонение подразделения (`None` = именительный). Та же логика, что должности.
pub fn decline_office(office: &str, case: Option<Case>) -> String {
    decline_appointment_text(office, case)
}

/// Нормализация слова для дедупликации: нижний регистр + срезанная конечная
/// гласная (`цеха`/`Цех` → `цех`). Приближение стемминга padeg.
fn stem_word(word: &str) -> String {
    let lower = word.to_lowercase();
    lower
        .strip_suffix(|c: char| "аеёиоуыэюя".contains(c))
        .unwrap_or(&lower)
        .to_owned()
}

/// Склейка «должность + подразделение» с удалением слов офиса, уже есть
/// в должности (сравнение по стемам, регистронезависимо):
/// `Начальник цеха` + `Цех …` → без дубля `Цех`.
pub fn merge_appointment(appointment: &str, office: &str) -> String {
    let app_words: Vec<&str> = appointment.split_whitespace().collect();
    let stems: HashSet<String> = app_words.iter().map(|w| stem_word(w)).collect();
    let mut merged: Vec<&str> = app_words;
    for word in office.split_whitespace() {
        if !stems.contains(&stem_word(word)) {
            merged.push(word);
        }
    }
    merged.join(" ")
}

/// Склонение склейки «должность + подразделение».
pub fn decline_full_appointment(appointment: &str, office: &str, case: Option<Case>) -> String {
    decline_appointment_text(&merge_appointment(appointment, office), case)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appointment_first_word_only() {
        assert_eq!(
            decline_appointment("заведующий сектором", Some(Case::Genitive)),
            "заведующего сектором"
        );
        assert_eq!(
            decline_appointment("заведующий сектором", Some(Case::Dative)),
            "заведующему сектором"
        );
        assert_eq!(
            decline_appointment("заведующий сектором", Some(Case::Accusative)),
            "заведующего сектором"
        );
        assert_eq!(
            decline_appointment("заведующий сектором", Some(Case::Instrumental)),
            "заведующим сектором"
        );
        assert_eq!(
            decline_appointment("заведующий сектором", Some(Case::Prepositional)),
            "заведующем сектором"
        );
        assert_eq!(
            decline_appointment("заведующий сектором", None),
            "заведующий сектором"
        );
        // Неодушевлённое: винительный = именительный.
        assert_eq!(decline_office("Сектор", Some(Case::Genitive)), "Сектора");
        assert_eq!(decline_office("Сектор", Some(Case::Accusative)), "Сектор");
        assert_eq!(
            decline_office("Сектор", Some(Case::Instrumental)),
            "Сектором"
        );
        assert_eq!(decline_appointment("", Some(Case::Dative)), "");
    }

    #[test]
    fn appointment_feminine_and_animate() {
        assert_eq!(
            decline_appointment("начальница цеха", Some(Case::Genitive)),
            "начальницы цеха"
        );
        assert_eq!(
            decline_appointment("медсестра", Some(Case::Dative)),
            "медсестре"
        );
        assert_eq!(
            decline_appointment("директор завода", Some(Case::Accusative)),
            "директора завода"
        );
        assert_eq!(
            decline_appointment("инженер", Some(Case::Accusative)),
            "инженера"
        );
        assert_eq!(
            decline_appointment("Сектор разработки", Some(Case::Accusative)),
            "Сектор разработки"
        );
        assert_eq!(decline_appointment("судья", Some(Case::Genitive)), "судьи");
    }

    #[test]
    fn compound_appointment_dash_separator() {
        assert_eq!(
            decline_appointment("инженер - конструктор", Some(Case::Genitive)),
            "инженера - конструктора"
        );
    }

    #[test]
    fn full_appointment_merges_without_dupes() {
        assert_eq!(
            merge_appointment("Начальник цеха", "Цех нестандартного оборудования"),
            "Начальник цеха нестандартного оборудования"
        );
        assert_eq!(
            decline_full_appointment("директор", "дирекция", Some(Case::Genitive)),
            "директора дирекция"
        );
    }
}
