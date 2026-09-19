//! Minimal HTTP API over [`petrovich_core`], dependency-free (std only).
//!
//! Routes (all `GET`, UTF-8, `text/plain` unless noted):
//!
//! - `/health` → `ok`
//! - `/decline?lastname=&firstname=&middlename=&case=&sex=` → inflected FIO
//! - `/gender?lastname=&firstname=&middlename=` → `male|female|androgynous`
//! - `/cases?lastname=&firstname=&middlename=&sex=` → all six cases,
//!   tab-separated in nominative…prepositional order
//! - `/api/v1/decline?...` → JSON object with the three parts
//! - `/api/v1/gender?...` → `{"gender":"…"}` (`application/json`)
//! - `/appoint?appointment=&office=&case=` → declined job title
//!   (`office` optional, merged after the title)
//! - `/api/v1/appoint?...` → `{"appointment":"…"}` (`application/json`)
//!
//! `case` accepts English names (`genitive`, …) or padeg-style numbers
//! 1–6 (1 = nominative … 6 = prepositional). `sex` accepts
//! `male|female|auto` (`m|f|м|ж`, empty = auto).

use std::collections::HashMap;

use petrovich::{Case, Gender, detect_gender};

/// Fixed order used by `/cases`: nominative first, then the five [`Case`]s.
pub const CASE_ORDER: [Option<Case>; 6] = [
    None,
    Some(Case::Genitive),
    Some(Case::Dative),
    Some(Case::Accusative),
    Some(Case::Instrumental),
    Some(Case::Prepositional),
];

pub const CASE_NAMES: [&str; 6] = [
    "nominative",
    "genitive",
    "dative",
    "accusative",
    "instrumental",
    "prepositional",
];

/// Percent-decode `application/x-www-form-urlencoded` fragment (`+` → space).
pub fn url_decode(input: &str) -> Result<String, String> {
    let mut bytes = Vec::with_capacity(input.len());
    let mut chars = input.as_bytes().iter();
    while let Some(&b) = chars.next() {
        match b {
            b'%' => {
                let hex: Vec<u8> = chars.by_ref().take(2).copied().collect();
                if hex.len() != 2 {
                    return Err("bad percent-encoding".to_owned());
                }
                let digits =
                    std::str::from_utf8(&hex).map_err(|_| "bad percent-encoding".to_owned())?;
                let byte = u8::from_str_radix(digits, 16)
                    .map_err(|_| "bad percent-encoding".to_owned())?;
                bytes.push(byte);
            }
            b'+' => bytes.push(b' '),
            _ => bytes.push(b),
        }
    }
    String::from_utf8(bytes).map_err(|_| "query is not valid UTF-8".to_owned())
}

pub fn parse_query(query: &str) -> Result<HashMap<String, String>, String> {
    let mut map = HashMap::new();
    if query.is_empty() {
        return Ok(map);
    }
    for pair in query.split('&') {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        map.insert(url_decode(key)?, url_decode(value)?);
    }
    Ok(map)
}

fn param(params: &HashMap<String, String>, key: &str) -> Option<String> {
    params
        .get(key)
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
}

pub fn parse_case(value: &str) -> Result<Option<Case>, String> {
    match value.trim().to_lowercase().as_str() {
        "" | "nominative" | "1" | "им" => Ok(None),
        "genitive" | "2" | "рд" => Ok(Some(Case::Genitive)),
        "dative" | "3" | "дт" => Ok(Some(Case::Dative)),
        "accusative" | "4" | "вн" => Ok(Some(Case::Accusative)),
        "instrumental" | "5" | "тв" => Ok(Some(Case::Instrumental)),
        "prepositional" | "6" | "пр" => Ok(Some(Case::Prepositional)),
        other => Err(format!("unknown case {other:?} (want 1-6 or genitive/…)")),
    }
}

pub fn parse_sex(value: &str) -> Result<Option<Gender>, String> {
    match value.trim().to_lowercase().as_str() {
        "" | "auto" => Ok(None),
        "male" | "m" | "м" => Ok(Some(Gender::Male)),
        "female" | "f" | "ж" => Ok(Some(Gender::Female)),
        "androgynous" => Ok(Some(Gender::Androgynous)),
        other => Err(format!("unknown sex {other:?} (want male/female/auto)")),
    }
}

pub fn gender_name(gender: Gender) -> &'static str {
    match gender {
        Gender::Male => "male",
        Gender::Female => "female",
        Gender::Androgynous => "androgynous",
    }
}

struct Name {
    lastname: Option<String>,
    firstname: Option<String>,
    middlename: Option<String>,
}

impl Name {
    fn from_params(params: &HashMap<String, String>) -> Self {
        Self {
            lastname: param(params, "lastname"),
            firstname: param(params, "firstname"),
            middlename: param(params, "middlename"),
        }
    }

    fn is_empty(&self) -> bool {
        self.lastname.is_none() && self.firstname.is_none() && self.middlename.is_none()
    }

    fn gender(&self, sex: Option<Gender>) -> Gender {
        sex.unwrap_or_else(|| {
            detect_gender(
                self.lastname.as_deref(),
                self.firstname.as_deref(),
                self.middlename.as_deref(),
            )
        })
    }
}

/// HTTP response before serialization.
pub struct Response {
    pub status: u16,
    pub content_type: &'static str,
    pub body: String,
}

impl Response {
    fn ok(body: String) -> Self {
        Self {
            status: 200,
            content_type: "text/plain; charset=utf-8",
            body,
        }
    }

    fn json(body: String) -> Self {
        Self {
            status: 200,
            content_type: "application/json; charset=utf-8",
            body,
        }
    }

    fn bad(message: String) -> Self {
        Self {
            status: 400,
            content_type: "text/plain; charset=utf-8",
            body: message,
        }
    }

    fn not_found() -> Self {
        Self {
            status: 404,
            content_type: "text/plain; charset=utf-8",
            body: "not found".to_owned(),
        }
    }
}

fn escape_json(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// `/appoint` / `/api/v1/appoint`: job titles via the core
/// (`petrovich::decline_appointment` / `decline_full_appointment`).
/// Only the title head-word inflects (Directum article semantics);
/// `office`, when present, is merged unchanged after the title.
fn appoint(params: &HashMap<String, String>, json: bool) -> Response {
    let Some(appointment) = param(params, "appointment") else {
        return Response::bad("appointment is required".to_owned());
    };
    let case = match parse_case(params.get("case").map(String::as_str).unwrap_or("")) {
        Ok(case) => case,
        Err(message) => return Response::bad(message),
    };
    let value = match param(params, "office") {
        Some(office) => petrovich::decline_full_appointment(&appointment, &office, case),
        None => petrovich::decline_appointment(&appointment, case),
    };
    if json {
        Response::json(format!("{{\"appointment\":\"{}\"}}", escape_json(&value)))
    } else {
        Response::ok(value)
    }
}

fn decline_value(part: &str, kind: u8, gender: Gender, case: Case) -> String {
    match kind {
        0 => petrovich::lastname(gender, part, case),
        1 => petrovich::firstname(gender, part, case),
        _ => petrovich::middlename(gender, part, case),
    }
}

/// Pure router, unit-tested without sockets.
pub fn route(path: &str, params: &HashMap<String, String>) -> Response {
    if path == "/health" {
        return Response::ok("ok".to_owned());
    }
    if path == "/appoint" || path == "/api/v1/appoint" {
        return appoint(params, path == "/api/v1/appoint");
    }
    let json = path == "/api/v1/decline" || path == "/api/v1/gender";
    if !matches!(
        path,
        "/decline" | "/gender" | "/cases" | "/api/v1/decline" | "/api/v1/gender"
    ) {
        return Response::not_found();
    }

    let name = Name::from_params(params);
    if name.is_empty() {
        return Response::bad(
            "at least one of lastname/firstname/middlename is required".to_owned(),
        );
    }
    let sex = match parse_sex(params.get("sex").map(String::as_str).unwrap_or("")) {
        Ok(sex) => sex,
        Err(message) => return Response::bad(message),
    };
    let gender = name.gender(sex);
    let parts: [(Option<&String>, u8); 3] = [
        (name.lastname.as_ref(), 0),
        (name.firstname.as_ref(), 1),
        (name.middlename.as_ref(), 2),
    ];

    if path == "/gender" || path == "/api/v1/gender" {
        let value = gender_name(gender).to_owned();
        return if json {
            Response::json(format!("{{\"gender\":\"{value}\"}}"))
        } else {
            Response::ok(value)
        };
    }

    if path == "/cases" {
        let mut cases = Vec::with_capacity(6);
        for case in CASE_ORDER {
            let values: Vec<String> = parts
                .iter()
                .filter_map(|(part, kind)| {
                    part.map(|p| match case {
                        Some(case) => decline_value(p, *kind, gender, case),
                        None => (*p).clone(),
                    })
                })
                .collect();
            cases.push(values.join(" "));
        }
        return Response::ok(cases.join("\t"));
    }

    // /decline and /api/v1/decline
    let case = match parse_case(params.get("case").map(String::as_str).unwrap_or("")) {
        Ok(case) => case,
        Err(message) => return Response::bad(message),
    };
    if json {
        let ln = name
            .lastname
            .as_ref()
            .map(|p| match case {
                Some(case) => petrovich::lastname(gender, p, case),
                None => p.clone(),
            })
            .unwrap_or_default();
        let fn_ = name
            .firstname
            .as_ref()
            .map(|p| match case {
                Some(case) => petrovich::firstname(gender, p, case),
                None => p.clone(),
            })
            .unwrap_or_default();
        let mn = name
            .middlename
            .as_ref()
            .map(|p| match case {
                Some(case) => petrovich::middlename(gender, p, case),
                None => p.clone(),
            })
            .unwrap_or_default();
        Response::json(format!(
            "{{\"lastname\":\"{}\",\"firstname\":\"{}\",\"middlename\":\"{}\"}}",
            escape_json(&ln),
            escape_json(&fn_),
            escape_json(&mn)
        ))
    } else {
        let values: Vec<String> = parts
            .iter()
            .filter_map(|(part, kind)| {
                part.map(|p| match case {
                    Some(case) => decline_value(p, *kind, gender, case),
                    None => (*p).clone(),
                })
            })
            .collect();
        Response::ok(values.join(" "))
    }
}

pub fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        _ => "Error",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get(path: &str) -> Response {
        let (path, query) = path.split_once('?').unwrap_or((path, ""));
        route(path, &parse_query(query).unwrap())
    }

    #[test]
    fn health() {
        let response = get("/health");
        assert_eq!((response.status, response.body.as_str()), (200, "ok"));
    }

    #[test]
    fn decline_dative() {
        let response = get(
            "/decline?lastname=%D0%98%D0%B2%D0%B0%D0%BD%D0%BE%D0%B2&firstname=%D0%98%D0%B2%D0%B0%D0%BD&case=dative&sex=male",
        );
        assert_eq!(response.status, 200);
        assert_eq!(response.body, "Иванову Ивану");
    }

    #[test]
    fn decline_case_number_and_auto_sex() {
        let response = get("/decline?firstname=Александра&case=2");
        assert_eq!(response.body, "Александры");
    }

    #[test]
    fn gender_plain() {
        let response = get("/gender?firstname=Саша");
        assert_eq!(response.body, "androgynous");
    }

    #[test]
    fn cases_tab_order() {
        let response = get("/cases?firstname=Иван&sex=male");
        assert_eq!(response.status, 200);
        let fields: Vec<&str> = response.body.split('\t').collect();
        assert_eq!(
            fields,
            ["Иван", "Ивана", "Ивану", "Ивана", "Иваном", "Иване"]
        );
    }

    #[test]
    fn json_decline() {
        let response = get("/api/v1/decline?lastname=Иванов&case=3&sex=male");
        assert_eq!(response.content_type, "application/json; charset=utf-8");
        assert_eq!(
            response.body,
            "{\"lastname\":\"Иванову\",\"firstname\":\"\",\"middlename\":\"\"}"
        );
    }

    #[test]
    fn appoint_title_and_office() {
        let response = get("/appoint?appointment=генеральный директор&case=dative");
        assert_eq!(response.status, 200);
        assert_eq!(response.body, "генеральному директор");
        let response = get(
            "/appoint?appointment=генеральный директор&office=департамента продаж&case=genitive",
        );
        assert_eq!(response.body, "генерального директор департамента продаж");
    }

    #[test]
    fn appoint_json_and_errors() {
        let response = get("/api/v1/appoint?appointment=начальник отдела&case=instrumental");
        assert_eq!(response.content_type, "application/json; charset=utf-8");
        assert_eq!(response.body, "{\"appointment\":\"начальником отдела\"}");
        assert_eq!(get("/appoint").status, 400);
        assert_eq!(get("/appoint?appointment=директор&case=9").status, 400);
    }

    #[test]
    fn errors_are_400() {
        assert_eq!(get("/decline").status, 400);
        assert_eq!(get("/decline?firstname=Иван&case=9").status, 400);
        assert_eq!(get("/decline?firstname=Иван&sex=x").status, 400);
        assert_eq!(get("/nope").status, 404);
        assert!(url_decode("%ZZ").is_err());
    }
}
