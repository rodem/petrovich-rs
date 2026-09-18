#[allow(dead_code)]
mod common;

use std::collections::BTreeSet;

use common::{GRAMMEME_ANDROGYNOUS, GenderTag, data_path, load_allowlist, load_gender};
use petrovich::{Gender, detect_gender};

const GENDER_DATASETS: [&str; 4] = [
    "firstnames.gender",
    "surnames.gender",
    "midnames.gender",
    "firstnames.popular.gender",
];

fn read_people_gender() -> Vec<(Vec<Option<String>>, GenderTag)> {
    let path = data_path("people.gender");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let mut rows = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        assert_eq!(
            fields.len(),
            4,
            "people.gender:{}: expected 4 columns",
            i + 1
        );
        if i == 0 {
            assert_eq!(fields, ["lastname", "firstname", "middlename", "gender"]);
            continue;
        }
        let tag = match fields[3] {
            "мр" => GenderTag::Male,
            "жр" => GenderTag::Female,
            GRAMMEME_ANDROGYNOUS => GenderTag::Androgynous,
            other => panic!("people.gender:{}: unknown gender tag {other:?}", i + 1),
        };
        let name = |s: &str| {
            if s.is_empty() {
                None
            } else {
                Some(s.to_string())
            }
        };
        rows.push((vec![name(fields[0]), name(fields[1]), name(fields[2])], tag));
    }
    rows
}

fn tag_to_gender(tag: GenderTag) -> Gender {
    match tag {
        GenderTag::Male => Gender::Male,
        GenderTag::Female => Gender::Female,
        GenderTag::Androgynous => Gender::Androgynous,
    }
}

fn tag_name(tag: GenderTag) -> &'static str {
    match tag {
        GenderTag::Male => "male",
        GenderTag::Female => "female",
        GenderTag::Androgynous => "androgynous",
    }
}

fn gender_name(gender: Gender) -> &'static str {
    match gender {
        Gender::Male => "male",
        Gender::Female => "female",
        Gender::Androgynous => "androgynous",
    }
}

#[test]
fn golden_gender() {
    run_gender();
}

fn run_gender() {
    for namepart in GENDER_DATASETS {
        let rows = load_gender(namepart);
        let mut total = 0usize;
        let mut correct = 0usize;
        let mut mismatches: BTreeSet<String> = BTreeSet::new();
        for row in &rows {
            let (ln, fn_, mn) = split_lemma(&row.lemma, namepart);
            let detected = detect_gender(ln, fn_, mn);
            let expected = tag_to_gender(row.gender);
            total += 1;
            if detected == expected {
                correct += 1;
            } else {
                mismatches.insert(format!(
                    "{}\t{}\t{}",
                    row.lemma,
                    tag_name(row.gender),
                    gender_name(detected)
                ));
            }
        }
        println!("{namepart}: {correct}/{total} matched");
        assert!(total > 100, "{namepart}: dataset too small ({total})");
        if !mismatches.is_empty() {
            println!("{namepart}: {} mismatches, first 50:", mismatches.len());
            for row in mismatches.iter().take(50) {
                println!("  {row:?}");
            }
        }
        // Ruby-parity gate (same semantics as declension; datasets without an
        // allowlist file must match exactly).
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
    people_gender_rows();
}

fn data_allow_path(namepart: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join(format!("{namepart}.allow.tsv"))
}

fn people_gender_rows() {
    let rows = read_people_gender();
    let mut total = 0usize;
    let mut correct = 0usize;
    let mut mismatches: Vec<(Vec<Option<String>>, GenderTag, Gender)> = Vec::new();
    for (parts, expected_tag) in &rows {
        let detected = detect_gender(
            parts[0].as_deref(),
            parts[1].as_deref(),
            parts[2].as_deref(),
        );
        let expected = tag_to_gender(*expected_tag);
        total += 1;
        if detected == expected {
            correct += 1;
        } else {
            mismatches.push((parts.clone(), *expected_tag, detected));
        }
    }
    println!("people.gender: {correct}/{total} matched");
    if !mismatches.is_empty() {
        println!("people.gender: {} mismatches:", mismatches.len());
        for (parts, expected, actual) in &mismatches {
            println!("  parts={parts:?} expected={expected:?} actual={actual:?}");
        }
    }
    assert_eq!(
        correct,
        total,
        "people.gender: {}/{total} matched. Mismatches (parts, expected, actual): {mismatches:#?}",
        total - mismatches.len()
    );
}

fn split_lemma<'a>(
    lemma: &'a str,
    namepart: &str,
) -> (Option<&'a str>, Option<&'a str>, Option<&'a str>) {
    match namepart {
        "firstnames.gender" | "firstnames.popular.gender" => (None, Some(lemma), None),
        "surnames.gender" => (Some(lemma), None, None),
        _ => (None, None, Some(lemma)),
    }
}
