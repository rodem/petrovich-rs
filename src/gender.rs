/// Возможные рода
#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub enum Gender {
    /// Мужской род
    Male,
    /// Женский род
    Female,
    /// Средний род
    Androgynous,
}

struct GenderRule {
    gender: Gender,
    suffix: &'static str,
}

struct GenderHeuristic {
    /// Exact-match exceptions, load order (androgynous, male, female);
    /// lookup scans from the end so the last write wins, like Ruby's dict.
    exceptions: &'static [(&'static str, Gender)],
    /// Suffix rules, stable-sorted by length descending (Ruby `-accuracy`).
    suffixes: &'static [GenderRule],
}

impl GenderHeuristic {
    /// Vote of a single hyphen part (lowercased): exact exception first,
    /// then the first suffix in length order. `None` means "no rule fired".
    fn match_part(&self, part: &str) -> Option<Gender> {
        if let Some((_, gender)) = self.exceptions.iter().rev().find(|(name, _)| *name == part) {
            return Some(*gender);
        }
        self.suffixes
            .iter()
            .find(|rule| part.ends_with(rule.suffix))
            .map(|rule| rule.gender)
    }

    /// Port of `find_all_gender_rules` folding: every hyphen part votes, the
    /// last part wins; a part with no rule votes `Androgynous`.
    fn detect_gender(&self, name: &str) -> Gender {
        let mut vote = Gender::Androgynous;
        for part in name.split('-') {
            vote = self
                .match_part(&part.to_lowercase())
                .unwrap_or(Gender::Androgynous);
        }
        vote
    }
}

struct GenderHeuristics {
    lastname: GenderHeuristic,
    firstname: GenderHeuristic,
    middlename: GenderHeuristic,
}

const GENDER: GenderHeuristics = include!(concat!(env!("OUT_DIR"), "/gender.inc"));

/// Port of `Petrovich::Gender.detect`: per-part votes, middlename priority,
/// then the firstname/lastname disambiguation, then unanimity; otherwise
/// `Gender::Androgynous` (covers Ruby's `nil`).
pub fn detect_gender(
    lastname: Option<&str>,
    firstname: Option<&str>,
    middlename: Option<&str>,
) -> Gender {
    // Empty strings vote nothing: Ruby's `"".split('-')` is `[]`, so no rule
    // fires and the part never enters the vote.
    fn present(name: Option<&str>) -> Option<&str> {
        name.filter(|n| !n.is_empty())
    }
    let ln = present(lastname).map(|n| GENDER.lastname.detect_gender(&n.to_lowercase()));
    let fn_ = present(firstname).map(|n| GENDER.firstname.detect_gender(&n.to_lowercase()));
    let mn = present(middlename).map(|n| GENDER.middlename.detect_gender(&n.to_lowercase()));

    // Return gender if middlename is specified and gender is determined.
    if let Some(Gender::Male | Gender::Female) = mn {
        return mn.unwrap();
    }

    let mut uniq: Vec<Gender> = Vec::with_capacity(3);
    for vote in [ln, fn_, mn].into_iter().flatten() {
        if !uniq.contains(&vote) {
            uniq.push(vote);
        }
    }

    if uniq.len() > 1 {
        // Absent parts vote nothing here (Ruby's `nil == :androgynous` is false).
        if matches!(fn_, Some(Gender::Male) | Some(Gender::Female))
            && ln == Some(Gender::Androgynous)
        {
            return fn_.unwrap();
        }
        if matches!(ln, Some(Gender::Male) | Some(Gender::Female))
            && fn_ == Some(Gender::Androgynous)
        {
            return ln.unwrap();
        }
    }

    if uniq.len() == 1 {
        return uniq[0];
    }

    Gender::Androgynous
}
