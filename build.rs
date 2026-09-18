#![allow(unexpected_cfgs, non_local_definitions)]

use serde::Deserialize;
use std::io::{BufReader, BufWriter, Write};

#[derive(Deserialize, Debug)]
enum Gender {
    #[serde(rename(deserialize = "male"))]
    Male,
    #[serde(rename(deserialize = "female"))]
    Female,
    #[serde(rename(deserialize = "androgynous"))]
    Androgynous,
}

#[derive(Deserialize)]
struct Rule {
    gender: Gender,
    test: Vec<String>,
    mods: [String; 5],
    // NOTE: `tags` (e.g. `first_word`) are intentionally ignored: the Ruby
    // etalon discards them as well (`@tags = []`), so rules apply to every
    // hyphen part. Serde skips unknown fields by default.
}

#[derive(Deserialize)]
struct RuleList {
    exceptions: Vec<Rule>,
    suffixes: Vec<Rule>,
}

#[derive(Deserialize)]
struct Rules {
    lastname: RuleList,
    firstname: RuleList,
    middlename: RuleList,
}

fn generate_rule(rule: &Rule, output: &mut impl Write) -> std::io::Result<()> {
    writeln!(output, "            Rule {{")?;
    writeln!(output, "                gender: Gender::{:?},", rule.gender)?;
    writeln!(output, "                test: &[")?;
    for test in &rule.test {
        writeln!(output, "                    {:?},", test)?;
    }
    writeln!(output, "                ],")?;
    writeln!(output, "                mods: [")?;
    for modifier in rule.mods.iter() {
        if modifier == "." {
            writeln!(output, "                    None,")?;
        } else {
            let dashes: usize = modifier
                .chars()
                .fold(0, |acc, c| if c == '-' { acc + 1 } else { acc });
            let ending = modifier.chars().skip(dashes).collect::<String>();
            writeln!(
                output,
                "                    Some(({}, {:?})),",
                dashes, ending
            )?;
        }
    }
    writeln!(output, "                ],")?;
    writeln!(output, "            }},")
}

fn generate_rule_list(list: &RuleList, output: &mut impl Write) -> std::io::Result<()> {
    writeln!(output, "RuleList {{")?;
    writeln!(output, "        exceptions: &[")?;
    for exception in &list.exceptions {
        generate_rule(exception, output)?;
    }
    writeln!(output, "        ],")?;
    writeln!(output, "        suffixes: &[")?;
    for suffix in &list.suffixes {
        generate_rule(suffix, output)?;
    }
    writeln!(output, "        ],")?;
    writeln!(output, "    }},")
}

fn generate_rules(rules: &Rules, output: &mut impl Write) -> std::io::Result<()> {
    writeln!(output, "Rules {{")?;
    write!(output, "    lastname: ")?;
    generate_rule_list(&rules.lastname, output)?;
    write!(output, "    firstname: ")?;
    generate_rule_list(&rules.firstname, output)?;
    write!(output, "    middlename: ")?;
    generate_rule_list(&rules.middlename, output)?;
    writeln!(output, "}}")
}

#[derive(Deserialize)]
struct GenderMapping {
    #[serde(default = "Vec::new")]
    androgynous: Vec<String>,
    #[serde(default = "Vec::new")]
    male: Vec<String>,
    #[serde(default = "Vec::new")]
    female: Vec<String>,
}

#[derive(Deserialize)]
struct GenderHeuristic {
    exceptions: Option<GenderMapping>,
    suffixes: GenderMapping,
}

#[derive(Deserialize)]
struct GenderHeuristics {
    lastname: GenderHeuristic,
    firstname: GenderHeuristic,
    middlename: GenderHeuristic,
}

#[derive(Deserialize)]
struct GenderHeuristicsList {
    gender: GenderHeuristics,
}

/// Port of `RuleSet#load_gender_rules!`: exceptions merged into one ordered
/// list (sections androgynous, male, female — last write wins on lookup),
/// suffixes flattened and stable-sorted by length descending (`-accuracy`).
fn generate_gender_heuristic(
    heuristic: &GenderHeuristic,
    output: &mut impl Write,
) -> std::io::Result<()> {
    writeln!(output, "GenderHeuristic {{")?;

    let empty = GenderMapping {
        androgynous: Vec::new(),
        male: Vec::new(),
        female: Vec::new(),
    };
    let exceptions = heuristic.exceptions.as_ref().unwrap_or(&empty);
    let ordered: &[(&[String], &str)] = &[
        (&exceptions.androgynous, "Androgynous"),
        (&exceptions.male, "Male"),
        (&exceptions.female, "Female"),
    ];
    // Last write wins in Ruby's dict; dedupe here keeping the last occurrence.
    let mut seen = std::collections::HashSet::new();
    let mut merged: Vec<(&str, &str)> = Vec::new();
    for (list, gender) in ordered {
        for name in list.iter() {
            merged.push((name, gender));
        }
    }
    let deduped: Vec<(&str, &str)> = merged
        .iter()
        .rev()
        .filter(|(name, _)| seen.insert(*name))
        .rev()
        .copied()
        .collect();
    writeln!(output, "        exceptions: &[")?;
    for (name, gender) in &deduped {
        writeln!(output, "            ({name:?}, Gender::{gender}),")?;
    }
    writeln!(output, "        ],")?;

    let mut suffixes: Vec<(&str, &str)> = Vec::new();
    for (list, gender) in [
        (&heuristic.suffixes.androgynous, "Androgynous"),
        (&heuristic.suffixes.male, "Male"),
        (&heuristic.suffixes.female, "Female"),
    ] {
        for suffix in list {
            suffixes.push((suffix, gender));
        }
    }
    // Stable sort by char length descending = Ruby's `sort_by! { -accuracy }`.
    suffixes.sort_by_key(|(suffix, _)| std::cmp::Reverse(suffix.chars().count()));
    writeln!(output, "        suffixes: &[")?;
    for (suffix, gender) in &suffixes {
        writeln!(
            output,
            "            GenderRule {{ gender: Gender::{gender}, suffix: {suffix:?} }},"
        )?;
    }
    writeln!(output, "        ],")?;
    writeln!(output, "    }},")
}

fn generate_gender(gender: &GenderHeuristics, output: &mut impl Write) -> std::io::Result<()> {
    writeln!(output, "GenderHeuristics {{")?;
    write!(output, "    lastname: ")?;
    generate_gender_heuristic(&gender.lastname, output)?;
    write!(output, "    firstname: ")?;
    generate_gender_heuristic(&gender.firstname, output)?;
    write!(output, "    middlename: ")?;
    generate_gender_heuristic(&gender.middlename, output)?;
    writeln!(output, "}}")
}

struct YamlError(serde_yaml::Error);

impl From<YamlError> for std::io::Error {
    fn from(YamlError(error): YamlError) -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::InvalidData, error)
    }
}

fn main() -> std::io::Result<()> {
    use std::path::Path;

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/rules.yml");
    println!("cargo:rerun-if-changed=src/gender.yml");

    let out_dir = std::env::var_os("OUT_DIR").unwrap();

    let rules_json = std::fs::File::open("src/rules.yml")?;
    let rules = serde_yaml::from_reader(BufReader::new(rules_json)).map_err(YamlError)?;
    let rules_file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(Path::new(&out_dir).join("rules.inc"))?;
    generate_rules(&rules, &mut BufWriter::new(rules_file))?;

    let gender_json = std::fs::File::open("src/gender.yml")?;
    let gender: GenderHeuristicsList =
        serde_yaml::from_reader(BufReader::new(gender_json)).map_err(YamlError)?;
    let gender_file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(Path::new(&out_dir).join("gender.inc"))?;
    generate_gender(&gender.gender, &mut BufWriter::new(gender_file))
}
