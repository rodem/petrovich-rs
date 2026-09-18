//! `petrovich-com`: drop-in замена `padeg.dll` (`Padeg.Declension`).
//!
//! - [`adapter`] — чистая логика 4 методов поверх `petrovich-core`
//!   (без Windows API, тестируется везде).
//! - COM-слой (этап C2): `DllGetClassObject`, `IDispatch` с DISPID 1–4,
//!   регистрация только в HKCU (без прав администратора).

pub mod adapter;

#[cfg(windows)]
pub mod com;

pub use adapter::{
    PadegError, case_of, get_appointment_padeg, get_fio_padeg_fs, get_nominative_padeg, get_sex,
    sex_of, split_fio,
};
