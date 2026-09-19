//! Плоские C-экспорты padeg (`stdcall`, UTF-16 `PChar` — дока v4.1 §4).
//!
//! Буферный протокол (дока §1, §2): вызывающий выделяет `pResult`, в `nLen`
//! кладёт размер буфера **в символах**; после вызова `nLen` = записано символов
//! (без терминатора), возврат — код (`0` успех, `-1` плохой падеж, `-2` плохой
//! род, `-3` мал буфер). Delphi-пример доки (§10.3) подтверждает: буфер 255,
//! `Left(tmpS, nLen)` при `ret = 0`.
//!
//! `bSex: Boolean` (1 байт, `True = 1`): читаем младший байт, ненулевое = мужской.
//! На x86 имена экспортируются без декорирования (как у padeg; проверено
//! `dumpbin /exports`).

use std::ptr;

use crate::adapter::{
    self, CODE_OK, CODE_SMALL_BUFFER, PadegError, get_appointment_padeg, get_fio_padeg,
    get_fio_parts, get_full_appointment_padeg, get_if_padeg, get_if_padeg_fs, get_nominative_padeg,
    get_office_padeg, get_sex,
};

/// Чтение null-terminated UTF-16 строки. NULL → пустая строка.
unsafe fn read_wide_str(ptr: *const u16) -> String {
    if ptr.is_null() {
        return String::new();
    }
    unsafe {
        let mut len = 0usize;
        while *ptr.add(len) != 0 {
            len += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len))
    }
}

/// Запись результата в буфер вызывающего. Возврат — код padeg.
unsafe fn write_wide_out(p_result: *mut u16, n_len: *mut i32, s: &str) -> i32 {
    if p_result.is_null() || n_len.is_null() {
        return CODE_SMALL_BUFFER;
    }
    let capacity = unsafe { *n_len } as usize;
    let wide: Vec<u16> = s.encode_utf16().collect();
    if capacity == 0 {
        return CODE_SMALL_BUFFER;
    }
    let n = wide.len().min(capacity - 1);
    unsafe {
        ptr::copy_nonoverlapping(wide.as_ptr(), p_result, n);
        *p_result.add(n) = 0;
        *n_len = n as i32;
    }
    if wide.len() > n {
        CODE_SMALL_BUFFER
    } else {
        CODE_OK
    }
}

fn finish(p_result: *mut u16, n_len: *mut i32, r: Result<String, PadegError>) -> i32 {
    match r {
        Ok(s) => unsafe { write_wide_out(p_result, n_len, &s) },
        Err(e) => {
            unsafe { write_wide_out(p_result, n_len, "") };
            e.code
        }
    }
}

fn sex_of_bool(b_sex: i32) -> &'static str {
    if b_sex as u8 != 0 { "м" } else { "ж" }
}

/// Точка входа padeg.
///
/// # Safety
///
/// Параметры — валидные указатели padeg-протокола (UTF-16, буфер `pResult`
/// размером `*nLen` символов).
#[unsafe(no_mangle)]
pub unsafe extern "system" fn GetFIOPadeg(
    p_last_name: *const u16,
    p_first_name: *const u16,
    p_middle_name: *const u16,
    b_sex: i32,
    n_padeg: i32,
    p_result: *mut u16,
    n_len: *mut i32,
) -> i32 {
    let r = get_fio_padeg(
        &unsafe { read_wide_str(p_last_name) },
        &unsafe { read_wide_str(p_first_name) },
        &unsafe { read_wide_str(p_middle_name) },
        sex_of_bool(b_sex),
        n_padeg,
    );
    finish(p_result, n_len, r)
}

/// Точка входа padeg.
///
/// # Safety
///
/// См. [`GetFIOPadeg`].
#[unsafe(no_mangle)]
pub unsafe extern "system" fn GetFIOPadegAS(
    p_last_name: *const u16,
    p_first_name: *const u16,
    p_middle_name: *const u16,
    n_padeg: i32,
    p_result: *mut u16,
    n_len: *mut i32,
) -> i32 {
    // Пол по отчеству: детект ядра приоритизирует отчество (дока §4.1).
    let r = get_fio_padeg(
        &unsafe { read_wide_str(p_last_name) },
        &unsafe { read_wide_str(p_first_name) },
        &unsafe { read_wide_str(p_middle_name) },
        "",
        n_padeg,
    );
    finish(p_result, n_len, r)
}

/// Точка входа padeg.
///
/// # Safety
///
/// См. [`GetFIOPadeg`].
#[unsafe(no_mangle)]
pub unsafe extern "system" fn GetFIOPadegFS(
    p_fio: *const u16,
    b_sex: i32,
    n_padeg: i32,
    p_result: *mut u16,
    n_len: *mut i32,
) -> i32 {
    let r = adapter::get_fio_padeg_fs(
        &unsafe { read_wide_str(p_fio) },
        sex_of_bool(b_sex),
        n_padeg,
    );
    finish(p_result, n_len, r)
}

/// Точка входа padeg.
///
/// # Safety
///
/// См. [`GetFIOPadeg`].
#[unsafe(no_mangle)]
pub unsafe extern "system" fn GetFIOPadegFSAS(
    p_fio: *const u16,
    n_padeg: i32,
    p_result: *mut u16,
    n_len: *mut i32,
) -> i32 {
    let r = adapter::get_fio_padeg_fs(&unsafe { read_wide_str(p_fio) }, "", n_padeg);
    finish(p_result, n_len, r)
}

/// Точка входа padeg.
///
/// # Safety
///
/// См. [`GetFIOPadeg`].
#[unsafe(no_mangle)]
pub unsafe extern "system" fn GetIFPadeg(
    p_first_name: *const u16,
    p_last_name: *const u16,
    b_sex: i32,
    n_padeg: i32,
    p_result: *mut u16,
    n_len: *mut i32,
) -> i32 {
    let r = get_if_padeg(
        &unsafe { read_wide_str(p_first_name) },
        &unsafe { read_wide_str(p_last_name) },
        sex_of_bool(b_sex),
        n_padeg,
    );
    finish(p_result, n_len, r)
}

/// Точка входа padeg.
///
/// # Safety
///
/// См. [`GetFIOPadeg`].
#[unsafe(no_mangle)]
pub unsafe extern "system" fn GetIFPadegFS(
    p_if: *const u16,
    b_sex: i32,
    n_padeg: i32,
    p_result: *mut u16,
    n_len: *mut i32,
) -> i32 {
    let r = get_if_padeg_fs(&unsafe { read_wide_str(p_if) }, sex_of_bool(b_sex), n_padeg);
    finish(p_result, n_len, r)
}

/// Точка входа padeg.
///
/// # Safety
///
/// См. [`GetFIOPadeg`].
#[unsafe(no_mangle)]
pub unsafe extern "system" fn GetNominativePadeg(
    p_fio: *const u16,
    p_result: *mut u16,
    n_len: *mut i32,
) -> i32 {
    finish(
        p_result,
        n_len,
        Ok(get_nominative_padeg(&unsafe { read_wide_str(p_fio) })),
    )
}

/// Точка входа padeg.
///
/// # Safety
///
/// См. [`GetFIOPadeg`].
#[unsafe(no_mangle)]
pub unsafe extern "system" fn GetAppointmentPadeg(
    p_appointment: *const u16,
    n_padeg: i32,
    p_result: *mut u16,
    n_len: *mut i32,
) -> i32 {
    let r = get_appointment_padeg(&unsafe { read_wide_str(p_appointment) }, n_padeg);
    finish(p_result, n_len, r)
}

/// Точка входа padeg.
///
/// # Safety
///
/// См. [`GetFIOPadeg`].
#[unsafe(no_mangle)]
pub unsafe extern "system" fn GetFullAppointmentPadeg(
    p_appointment: *const u16,
    p_office: *const u16,
    n_padeg: i32,
    p_result: *mut u16,
    n_len: *mut i32,
) -> i32 {
    let r = get_full_appointment_padeg(
        &unsafe { read_wide_str(p_appointment) },
        &unsafe { read_wide_str(p_office) },
        n_padeg,
    );
    finish(p_result, n_len, r)
}

/// Точка входа padeg.
///
/// # Safety
///
/// См. [`GetFIOPadeg`].
#[unsafe(no_mangle)]
pub unsafe extern "system" fn GetOfficePadeg(
    p_office: *const u16,
    n_padeg: i32,
    p_result: *mut u16,
    n_len: *mut i32,
) -> i32 {
    let r = get_office_padeg(&unsafe { read_wide_str(p_office) }, n_padeg);
    finish(p_result, n_len, r)
}

/// Точка входа padeg: 1 мужской, 0 женский, -1 неизвестно.
///
/// # Safety
///
/// `p_middle_name` — валидный указатель UTF-16 или NULL.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn GetSex(p_middle_name: *const u16) -> i32 {
    get_sex(&unsafe { read_wide_str(p_middle_name) })
}

/// Точка входа padeg. Обратной морфологии нет — всегда `0` (не определён).
///
/// # Safety
///
/// Параметр игнорируется.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn GetPadegID(_p_fio: *const u16) -> i32 {
    0
}

/// `TPartsFIO` из доки v4.1 §4.4: три буфера + их размеры.
#[repr(C)]
pub struct PartsFIO {
    pub p_last_name: *mut u16,
    pub p_first_name: *mut u16,
    pub p_middle_name: *mut u16,
    pub n_last_name: i32,
    pub n_first_name: i32,
    pub n_middle_name: i32,
}

fn write_part(ptr: *mut u16, cap: &mut i32, s: &str, small_code: i32) -> i32 {
    let rc = unsafe { write_wide_out(ptr, cap as *mut i32, s) };
    if rc == CODE_SMALL_BUFFER {
        small_code
    } else {
        CODE_OK
    }
}

/// Точка входа padeg: 0 успех, -3/-4/-5 — мал буфер фамилии/имени/отчества.
///
/// # Safety
///
/// `parts` валиден; буферы размером `n_*` символов.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn GetFIOParts(p_fio: *const u16, parts: *mut PartsFIO) -> i32 {
    if parts.is_null() {
        return CODE_SMALL_BUFFER;
    }
    let (ln, fn_, mn) = get_fio_parts(&unsafe { read_wide_str(p_fio) });
    unsafe {
        let rc = write_part((*parts).p_last_name, &mut (*parts).n_last_name, &ln, -3);
        if rc != CODE_OK {
            return rc;
        }
        let rc = write_part((*parts).p_first_name, &mut (*parts).n_first_name, &fn_, -4);
        if rc != CODE_OK {
            return rc;
        }
        write_part((*parts).p_middle_name, &mut (*parts).n_middle_name, &mn, -5)
    }
}

/// Точка входа padeg. Правилa вшиты — перечитывать нечего.
///
/// # Safety
///
/// Без параметров.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn UpdateExceptions() -> bool {
    adapter::update_exceptions()
}

/// Точка входа padeg. Внешнего словаря нет — пустая строка.
///
/// # Safety
///
/// См. [`GetFIOPadeg`].
#[unsafe(no_mangle)]
pub unsafe extern "system" fn GetExceptionsFileName(p_result: *mut u16, n_len: *mut i32) -> i32 {
    unsafe { write_wide_out(p_result, n_len, "") }
}

/// Точка входа padeg. Внешний словарь не поддерживается.
///
/// # Safety
///
/// Параметр игнорируется.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn SetDictionary(_file_name: *const u16) -> bool {
    adapter::set_dictionary("")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    #[test]
    fn buffer_protocol_roundtrip() {
        let fio = wide("Иванов Иван Иванович");
        let mut buf = vec![0u16; 100];
        let mut len = 100i32;
        let rc =
            unsafe { GetFIOPadegFSAS(fio.as_ptr(), 3, buf.as_mut_ptr(), &mut len as *mut i32) };
        assert_eq!(rc, CODE_OK);
        let s = String::from_utf16_lossy(&buf[..len as usize]);
        assert_eq!(s, "Иванову Ивану Ивановичу");
    }

    #[test]
    fn buffer_protocol_codes() {
        let fio = wide("Иванов Иван Иванович");
        // Мал буфер → -3 + усечение.
        let mut buf = vec![0u16; 5];
        let mut len = 5i32;
        let rc =
            unsafe { GetFIOPadegFSAS(fio.as_ptr(), 3, buf.as_mut_ptr(), &mut len as *mut i32) };
        assert_eq!(rc, CODE_SMALL_BUFFER);
        // Плохой падеж → -1.
        let mut buf = vec![0u16; 100];
        let mut len = 100i32;
        let rc =
            unsafe { GetFIOPadegFSAS(fio.as_ptr(), 9, buf.as_mut_ptr(), &mut len as *mut i32) };
        assert_eq!(rc, adapter::CODE_BAD_PADEG);
    }

    #[test]
    fn sex_and_fio_parts_c_calls() {
        let mn = wide("Иванович");
        assert_eq!(unsafe { GetSex(mn.as_ptr()) }, 1);
        assert_eq!(unsafe { GetPadegID(mn.as_ptr()) }, 0);

        let fio = wide("Иванов Иван Иванович");
        let mut ln_b = vec![0u16; 50];
        let mut fn_b = vec![0u16; 50];
        let mut mn_b = vec![0u16; 50];
        let mut parts = PartsFIO {
            p_last_name: ln_b.as_mut_ptr(),
            p_first_name: fn_b.as_mut_ptr(),
            p_middle_name: mn_b.as_mut_ptr(),
            n_last_name: 50,
            n_first_name: 50,
            n_middle_name: 50,
        };
        let rc = unsafe { GetFIOParts(fio.as_ptr(), &mut parts as *mut PartsFIO) };
        assert_eq!(rc, CODE_OK);
        let s = |b: &[u16], n: i32| String::from_utf16_lossy(&b[..n as usize]);
        assert_eq!(s(&ln_b, parts.n_last_name), "Иванов");
        assert_eq!(s(&fn_b, parts.n_first_name), "Иван");
        assert_eq!(s(&mn_b, parts.n_middle_name), "Иванович");

        // Мал буфер фамилии → -3.
        let mut tiny = vec![0u16; 3];
        let mut parts2 = PartsFIO {
            p_last_name: tiny.as_mut_ptr(),
            p_first_name: fn_b.as_mut_ptr(),
            p_middle_name: mn_b.as_mut_ptr(),
            n_last_name: 3,
            n_first_name: 50,
            n_middle_name: 50,
        };
        let rc = unsafe { GetFIOParts(fio.as_ptr(), &mut parts2 as *mut PartsFIO) };
        assert_eq!(rc, -3);
    }

    #[test]
    fn bool_exports() {
        assert!(unsafe { UpdateExceptions() });
        assert!(!unsafe { SetDictionary(wide("x").as_ptr()) });
    }
}
