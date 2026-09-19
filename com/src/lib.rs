//! `petrovich-com`: drop-in замена `padeg.dll` (`Padeg.Declension`).
//!
//! - [`adapter`] — чистая логика методов padeg поверх `petrovich-core`
//!   (без Windows API, тестируется везде). Полный перечень — PLAN-COM-API.md.
//! - COM-слой: `DllGetClassObject`, `IDispatch` (DISPID 1–4 базовые,
//!   5+ добавочные), регистрация только в HKCU (без прав администратора).

pub mod adapter;

#[cfg(windows)]
pub mod capi;

#[cfg(windows)]
pub mod com;

pub use adapter::{
    PadegError, case_of, get_appointment_padeg, get_exceptions_file_name, get_fio_padeg,
    get_fio_padeg_fs, get_fio_parts, get_full_appointment_padeg, get_if_padeg, get_if_padeg_fs,
    get_nominative_padeg, get_office_padeg, get_padeg_id, get_sex, set_dictionary, sex_of,
    split_fio, update_exceptions,
};
