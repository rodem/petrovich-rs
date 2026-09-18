use std::borrow::Cow;
use std::path::{Path, PathBuf};

use petrovich::{Case, Gender};

pub const GRAMMEME_APTOTIC: &str = "0";
pub const GRAMMEME_SINGULAR: &str = "ед";
pub const GRAMMEME_MALE: &str = "мр";
pub const GRAMMEME_FEMALE: &str = "жр";
pub const GRAMMEME_ANDROGYNOUS: &str = "мр-жр";

pub fn data_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join(format!("{name}.tsv"))
}

fn read_lines(name: &str) -> Vec<String> {
    let path = data_path(name);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let mut lines = text.lines().map(|line| line.trim_start_matches('\u{feff}'));
    let expected_header = if name == "people" {
        "lastname\tfirstname\tmidname\tlastname_expected\tfirstname_expected\tmiddlename_expected\tgrammemes"
    } else if name.ends_with("gender") {
        "lemma\tgender"
    } else {
        "lemma\tword\tgrammemes"
    };
    assert_eq!(
        lines.next(),
        Some(expected_header),
        "{name}: invalid header"
    );
    lines
        .map(|line| line.trim_end_matches(['\r', '\n']).to_string())
        .filter(|line| !line.is_empty())
        .collect()
}

fn split_row<'a>(
    line: &'a str,
    name: &str,
    line_no: usize,
    expected_columns: usize,
) -> Vec<&'a str> {
    let fields: Vec<&str> = line.split('\t').collect();
    assert_eq!(
        fields.len(),
        expected_columns,
        "{name}:{line_no}: expected {expected_columns} columns, got {}: {line:?}",
        fields.len()
    );
    fields
}

pub fn case_from_grammemes(grammemes: &str) -> Option<Case> {
    for (i, tag) in ["рд", "дт", "вн", "тв", "пр"].iter().enumerate() {
        if grammemes.split(',').any(|g| g == *tag) {
            return Some(match i {
                0 => Case::Genitive,
                1 => Case::Dative,
                2 => Case::Accusative,
                3 => Case::Instrumental,
                _ => Case::Prepositional,
            });
        }
    }
    None
}

pub fn gender_from_grammemes(grammemes: &str) -> Option<Gender> {
    grammemes.split(',').find_map(|g| match g {
        GRAMMEME_MALE => Some(Gender::Male),
        GRAMMEME_FEMALE => Some(Gender::Female),
        _ => None,
    })
}

pub struct DeclensionRow {
    pub lemma: String,
    pub word: String,
    pub grammemes: String,
}

pub fn load_declension(name: &str) -> Vec<DeclensionRow> {
    read_lines(name)
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let fields = split_row(line, name, i + 2, 3);
            DeclensionRow {
                lemma: unescape_field(fields[0]).into_owned(),
                word: unescape_field(fields[1]).into_owned(),
                grammemes: fields[2].to_string(),
            }
        })
        .collect()
}

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub enum GenderTag {
    Male,
    Female,
    Androgynous,
}

pub struct GenderRow {
    pub lemma: String,
    pub gender: GenderTag,
}

pub fn load_gender(name: &str) -> Vec<GenderRow> {
    read_lines(name)
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let fields = split_row(line, name, i + 2, 2);
            let gender = match fields[1] {
                GRAMMEME_MALE => GenderTag::Male,
                GRAMMEME_FEMALE => GenderTag::Female,
                GRAMMEME_ANDROGYNOUS => GenderTag::Androgynous,
                other => panic!("{name}:{}: unknown gender tag {other:?}", i + 2),
            };
            GenderRow {
                lemma: unescape_field(fields[0]).into_owned(),
                gender,
            }
        })
        .collect()
}

pub struct PersonRow {
    pub lastname: String,
    pub firstname: String,
    pub middlename: String,
    pub lastname_expected: String,
    pub firstname_expected: String,
    pub middlename_expected: String,
    pub grammemes: String,
}

pub fn load_people(name: &str) -> Vec<PersonRow> {
    read_lines(name)
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let fields = split_row(line, name, i + 2, 7);
            PersonRow {
                lastname: unescape_field(fields[0]).into_owned(),
                firstname: unescape_field(fields[1]).into_owned(),
                middlename: unescape_field(fields[2]).into_owned(),
                lastname_expected: unescape_field(fields[3]).into_owned(),
                firstname_expected: unescape_field(fields[4]).into_owned(),
                middlename_expected: unescape_field(fields[5]).into_owned(),
                grammemes: fields[6].to_string(),
            }
        })
        .collect()
}

fn unescape_field(field: &str) -> Cow<'_, str> {
    if field.contains('\\') {
        Cow::Owned(
            field
                .replace("\\n", "\n")
                .replace("\\t", "\t")
                .replace("\\\\", "\\"),
        )
    } else {
        Cow::Borrowed(field)
    }
}

/// Ruby-parity allowlist: rows the etalon itself mismatches on this dataset.
///
/// File `tests/data/<name>.allow.tsv` has the same columns as the dump
/// (`tag/lemma/expected/actual` for declension, `lemma/expected/actual` for
/// gender). The test asserts the engine mismatch set EQUALS this set: any new
/// mismatch or any silently fixed row fails the test and forces a deliberate
/// allowlist update.
pub fn load_allowlist(name: &str) -> std::collections::BTreeSet<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join(format!("{name}.allow.tsv"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!("cannot read {}: {e}", path.display());
    });
    let mut lines = text.lines();
    lines.next(); // header
    lines
        .map(|line| line.trim_end_matches(['\r', '\n']).to_string())
        .filter(|line| !line.is_empty())
        .collect()
}

pub fn run_case(namepart: &str, gender: Gender, lemma: &str, case: Case) -> String {
    match namepart {
        "firstnames" => petrovich::firstname(gender, lemma, case),
        "surnames" => petrovich::lastname(gender, lemma, case),
        "midnames" => petrovich::middlename(gender, lemma, case),
        other => panic!("unknown namepart {other:?}"),
    }
}
