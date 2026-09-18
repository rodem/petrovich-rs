//! Сквозной пример из README: склонение ФИО по падежам.
//!
//! Запуск: `cargo run --release --example decline_fio`
//! Бинарь: `target/release/examples/decline_fio.exe` (или
//! `target/<triple>/release/examples/` при явном `--target`).

use petrovich::{Case, Gender, firstname, lastname, middlename};

fn main() {
    // Сквозной пример: дательный от «Иванов Иван Иванович».
    assert_eq!(lastname(Gender::Male, "Иванов", Case::Dative), "Иванову");
    assert_eq!(firstname(Gender::Male, "Иван", Case::Dative), "Ивану");
    assert_eq!(
        middlename(Gender::Male, "Иванович", Case::Dative),
        "Ивановичу"
    );

    // Женский род склоняется иначе.
    assert_eq!(
        firstname(Gender::Female, "Изабель", Case::Genitive),
        "Изабель"
    );
    assert_eq!(
        lastname(Gender::Female, "Станкевич", Case::Prepositional),
        "Станкевич"
    );
    assert_eq!(
        lastname(Gender::Male, "Станкевич", Case::Prepositional),
        "Станкевиче"
    );

    // Составные фамилии/имена через дефис — по частям.
    assert_eq!(
        lastname(Gender::Male, "Иванов-Сидоров", Case::Dative),
        "Иванову-Сидорову"
    );

    // Все 5 падежей ядра для наглядности.
    for case in [
        Case::Genitive,
        Case::Dative,
        Case::Accusative,
        Case::Instrumental,
        Case::Prepositional,
    ] {
        println!(
            "{case:?}: {} {} {}",
            lastname(Gender::Male, "Иванов", case),
            firstname(Gender::Male, "Иван", case),
            middlename(Gender::Male, "Иванович", case),
        );
    }
}
