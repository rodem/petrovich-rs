#[allow(dead_code)]
mod common;

use std::collections::BTreeSet;

use common::{
    GRAMMEME_APTOTIC, GRAMMEME_SINGULAR, case_from_grammemes, gender_from_grammemes,
    load_allowlist, load_declension, load_people, run_case,
};

const DATASETS: [&str; 3] = ["firstnames", "surnames", "midnames"];

#[test]
fn golden_declension() {
    golden_declension_cases();
}

#[test]
fn golden_people() {
    people_declension();
}

fn people_declension() {
    let rows = load_people("people");
    let mut checked = 0usize;
    let mut mismatches: Vec<(String, String, String, String)> = Vec::new();
    for row in &rows {
        let Some(case) = case_from_grammemes(&row.grammemes) else {
            continue;
        };
        let Some(gender) = gender_from_grammemes(&row.grammemes) else {
            continue;
        };
        let parts: [(&str, &str, &str); 3] = [
            (&row.lastname, &row.lastname_expected, "surnames"),
            (&row.firstname, &row.firstname_expected, "firstnames"),
            (&row.middlename, &row.middlename_expected, "midnames"),
        ];
        for (lemma, expected, namepart) in parts {
            if lemma.is_empty() {
                continue;
            }
            checked += 1;
            let actual = run_case(namepart, gender, lemma, case);
            let actual_upper = actual.to_uppercase();
            let expected_upper = expected.to_uppercase();
            if actual_upper != expected_upper && mismatches.len() < 50 {
                mismatches.push((
                    format!("{namepart}:{case:?}:{gender:?}"),
                    lemma.to_string(),
                    expected.to_string(),
                    format!("{actual} (upper {actual_upper})"),
                ));
            }
        }
    }
    println!("people: {checked} name parts checked");
    assert!(checked > 20, "people: too few parts checked ({checked})");
    assert!(
        mismatches.is_empty(),
        "people: mismatches (namepart:case:gender, lemma, expected, actual): {mismatches:#?}"
    );
}

fn golden_declension_cases() {
    for namepart in DATASETS {
        let rows = load_declension(namepart);
        let mut total = 0usize;
        let mut correct = 0usize;
        let mut mismatches: BTreeSet<String> = BTreeSet::new();
        for row in &rows {
            if !row.grammemes.split(',').any(|g| g == GRAMMEME_SINGULAR) {
                continue;
            }
            if row.grammemes.split(',').any(|g| g == GRAMMEME_APTOTIC) {
                continue;
            }
            let Some(case) = case_from_grammemes(&row.grammemes) else {
                continue;
            };
            let Some(gender) = gender_from_grammemes(&row.grammemes) else {
                continue;
            };
            let actual = run_case(namepart, gender, &row.lemma, case);
            total += 1;
            let actual_upper = actual.to_uppercase();
            if actual_upper == row.word {
                correct += 1;
            } else {
                // Lowercased tag matches the allowlist dump format
                // (`firstnames:genitive:male`).
                let tag = format!("{namepart}:{case:?}:{gender:?}").to_lowercase();
                mismatches.insert(format!(
                    "{tag}\t{}\t{}\t{actual_upper}",
                    row.lemma, row.word
                ));
            }
        }
        println!("{namepart}: {correct}/{total} matched");
        assert!(total > 1000, "{namepart}: dataset too small ({total})");
        if !mismatches.is_empty() {
            println!("{namepart}: {} mismatches, first 50:", mismatches.len());
            for row in mismatches.iter().take(50) {
                println!("  {row:?}");
            }
        }
        // Ruby-parity gate: the mismatch set must equal the etalon's own
        // mismatch set. `midnames` has no allowlist (etalon is exact there).
        let allowed: BTreeSet<String> = match std::fs::metadata(data_allow_path(namepart)) {
            Ok(_) => load_allowlist(namepart),
            Err(_) => BTreeSet::new(),
        };
        let extra: Vec<&String> = mismatches.difference(&allowed).collect();
        let fixed: Vec<&String> = allowed.difference(&mismatches).collect();
        assert!(
            extra.is_empty() && fixed.is_empty(),
            "{namepart}: parity drift: {} new mismatches, {} fixed-but-allowlisted. New: {extra:#?}. Fixed: {fixed:#?}",
            extra.len(),
            fixed.len(),
        );
    }
}

fn data_allow_path(namepart: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join(format!("{namepart}.allow.tsv"))
}
