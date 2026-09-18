//! Пример из README: определение пола по частям ФИО.
//!
//! Запуск: `cargo run --release --example detect_gender`
//! Бинарь: `target/release/examples/detect_gender.exe` (или
//! `target/<triple>/release/examples/` при явном `--target`).

use petrovich::{Gender, detect_gender};

fn show(lastname: Option<&str>, firstname: Option<&str>, middlename: Option<&str>) {
    println!(
        "{lastname:?} {firstname:?} {middlename:?} -> {:?}",
        detect_gender(lastname, firstname, middlename)
    );
}

fn main() {
    assert_eq!(detect_gender(None, Some("Александр"), None), Gender::Male);
    assert_eq!(
        detect_gender(None, Some("Александра"), None),
        Gender::Female
    );
    // «Саша» само по себе андрогинно, но фамилия/отчество решают.
    assert_eq!(detect_gender(None, Some("Саша"), None), Gender::Androgynous);
    assert_eq!(
        detect_gender(Some("Иванов"), Some("Саша"), None),
        Gender::Male
    );
    assert_eq!(
        detect_gender(Some("Склифасовская"), Some("Александра"), None),
        Gender::Female
    );
    // Пустые строки не голосуют (как в Ruby-эталоне).
    assert_eq!(
        detect_gender(Some("Склифасовская"), Some("Александра"), Some("")),
        Gender::Female
    );

    show(None, Some("Александр"), None);
    show(None, Some("Александра"), None);
    show(None, Some("Саша"), None);
    show(Some("Иванов"), Some("Саша"), None);
    show(Some("Склифасовская"), Some("Александра"), None);
}
