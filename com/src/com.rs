//! COM-слой `Padeg.Declension`: ручной IDispatch-сервер без внешних crate.
//!
//! Совместимость по вызовам: `CreateObject("Padeg.Declension")` + 4 метода
//! (DISPID 1–4, имена регистронезависимые — VBScript не различает регистр).
//! Ошибки адаптера → `DISP_E_EXCEPTION` + `EXCEPINFO` с текстом.
//! Регистрация — **только HKCU** (без прав администратора), см. `register.rs`.

#![allow(non_snake_case, non_upper_case_globals)]

use std::ffi::c_void;
use std::panic::AssertUnwindSafe;
use std::ptr;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::adapter;

// ---------------------------------------------------------------------------
// GUID / ProgID
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct GUID {
    pub Data1: u32,
    pub Data2: u16,
    pub Data3: u16,
    pub Data4: [u8; 8],
}

pub const CLSID_DECLENSION: GUID = GUID {
    Data1: 0xE4B2C1A0,
    Data2: 0x5D6F,
    Data3: 0x4A7B,
    Data4: [0x8C, 0x9D, 0x0E, 0x1F, 0x2A, 0x3B, 0x4C, 0x5D],
};
pub const CLSID_DECLENSION_STR: &str = "{E4B2C1A0-5D6F-4A7B-8C9D-0E1F2A3B4C5D}";

pub const PROGID_PADEG: &str = "Padeg.Declension";
pub const PROGID_PETROVICH: &str = "Petrovich.Declension";
pub const FRIENDLY_NAME: &str = "Petrovich Declension (padeg-compatible)";

const IID_IUNKNOWN: GUID = GUID {
    Data1: 0x00000000,
    Data2: 0x0000,
    Data3: 0x0000,
    Data4: [0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
};
const IID_IDISPATCH: GUID = GUID {
    Data1: 0x00020400,
    Data2: 0x0000,
    Data3: 0x0000,
    Data4: [0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
};
const IID_ICLASSFACTORY: GUID = GUID {
    Data1: 0x00000001,
    Data2: 0x0000,
    Data3: 0x0000,
    Data4: [0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
};

// DISPID 1–4 (нумерация наша; late-binding клиенты ходят по именам).
const DISPID_GETSEX: i32 = 1;
const DISPID_GETFIOPADEGFS: i32 = 2;
const DISPID_GETNOMINATIVEPADEG: i32 = 3;
const DISPID_GETAPPOINTMENTPADEG: i32 = 4;

// ---------------------------------------------------------------------------
// HRESULT / VARTYPE
// ---------------------------------------------------------------------------

pub const S_OK: i32 = 0;
#[allow(dead_code)]
pub const S_FALSE: i32 = 1;
pub const E_NOINTERFACE: i32 = 0x80004002u32 as i32;
pub const E_POINTER: i32 = 0x80004003u32 as i32;
pub const E_NOTIMPL: i32 = 0x80004001u32 as i32;
pub const E_INVALIDARG: i32 = 0x80070057u32 as i32;
pub const E_OUTOFMEMORY: i32 = 0x8007000Eu32 as i32;
pub const E_FAIL: i32 = 0x80004005u32 as i32;
pub const CLASS_E_CLASSNOTAVAILABLE: i32 = 0x80040111u32 as i32;
pub const CLASS_E_NOAGGREGATION: i32 = 0x80040110u32 as i32;
pub const DISP_E_UNKNOWNNAME: i32 = 0x80020006u32 as i32;
pub const DISP_E_BADPARAMCOUNT: i32 = 0x8002000Eu32 as i32;
pub const DISP_E_MEMBERNOTFOUND: i32 = 0x80020003u32 as i32;
pub const DISP_E_EXCEPTION: i32 = 0x80020009u32 as i32;
pub const DISP_E_TYPEMISMATCH: i32 = 0x80020005u32 as i32;

const DISPATCH_METHOD: u16 = 1;

const VT_EMPTY: u16 = 0;
const VT_NULL: u16 = 1;
const VT_I2: u16 = 2;
const VT_I4: u16 = 3;
const VT_R8: u16 = 5;
const VT_BOOL: u16 = 11;
const VT_BSTR: u16 = 8;
const VT_UI4: u16 = 19;
const VT_VARIANT: u16 = 12;

// ---------------------------------------------------------------------------
// COM-структуры
// ---------------------------------------------------------------------------

/// Бинарно совместим с `VARIANT` (16 байт): tag + 3 reserved + union 8 байт.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VARIANT {
    pub vt: u16,
    pub wReserved1: u16,
    pub wReserved2: u16,
    pub wReserved3: u16,
    pub data1: usize,
    pub data2: usize,
}

#[repr(C)]
pub struct DISPPARAMS {
    pub rgvarg: *mut VARIANT,
    pub rgdispidNamedArgs: *mut i32,
    pub cArgs: u32,
    pub cNamedArgs: u32,
}

#[repr(C)]
pub struct EXCEPINFO {
    pub wCode: u16,
    pub wReserved: u16,
    pub bstrSource: *mut u16,
    pub bstrDescription: *mut u16,
    pub bstrHelpFile: *mut u16,
    pub dwHelpContext: u32,
    pub pvReserved: *mut c_void,
    pub pfnDeferredFillIn: *mut c_void,
    pub scode: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct IUnknownVtbl {
    pub QueryInterface:
        unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> i32,
    pub AddRef: unsafe extern "system" fn(*mut c_void) -> u32,
    pub Release: unsafe extern "system" fn(*mut c_void) -> u32,
}

#[repr(C)]
pub struct IDispatchVtbl {
    pub base: IUnknownVtbl,
    pub GetTypeInfoCount: unsafe extern "system" fn(*mut c_void, *mut u32) -> i32,
    pub GetTypeInfo: unsafe extern "system" fn(*mut c_void, u32, u32, *mut *mut c_void) -> i32,
    pub GetIDsOfNames: unsafe extern "system" fn(
        *mut c_void,
        *const GUID,
        *mut *mut u16,
        u32,
        u32,
        *mut i32,
    ) -> i32,
    pub Invoke: unsafe extern "system" fn(
        *mut c_void,
        i32,
        *const GUID,
        u32,
        u16,
        *mut DISPPARAMS,
        *mut VARIANT,
        *mut EXCEPINFO,
        *mut u32,
    ) -> i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct IClassFactoryVtbl {
    pub base: IUnknownVtbl,
    pub CreateInstance:
        unsafe extern "system" fn(*mut c_void, *mut c_void, *const GUID, *mut *mut c_void) -> i32,
    pub LockServer: unsafe extern "system" fn(*mut c_void, i32) -> i32,
}

#[repr(C)]
struct Declension {
    vtbl: *const IDispatchVtbl,
    refs: AtomicU32,
}

// ---------------------------------------------------------------------------
// Импорт ole32/oleaut32/kernel32/advapi32 (MSVC: SDK .lib; raw-FFI, без crate)
// ---------------------------------------------------------------------------

#[link(name = "oleaut32")]
#[cfg_attr(not(test), allow(dead_code))]
unsafe extern "system" {
    fn SysAllocStringLen(psz: *const u16, len: u32) -> *mut u16;
    fn SysFreeString(bstr: *mut u16);
    fn SysStringLen(bstr: *mut u16) -> u32;
    fn VariantInit(pvarg: *mut VARIANT);
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetModuleFileNameW(hModule: *mut c_void, lpFilename: *mut u16, nSize: u32) -> u32;
    fn DisableThreadLibraryCalls(hModule: *mut c_void) -> i32;
}

#[link(name = "advapi32")]
unsafe extern "system" {
    fn RegCreateKeyExW(
        hKey: *mut c_void,
        lpSubKey: *const u16,
        Reserved: u32,
        lpClass: *mut u16,
        dwOptions: u32,
        samDesired: u32,
        lpSecurityAttributes: *mut c_void,
        phkResult: *mut *mut c_void,
        lpdwDisposition: *mut u32,
    ) -> i32;
    fn RegSetValueExW(
        hKey: *mut c_void,
        lpValueName: *const u16,
        Reserved: u32,
        dwType: u32,
        lpData: *const u8,
        cbData: u32,
    ) -> i32;
    fn RegCloseKey(hKey: *mut c_void) -> i32;
    fn RegDeleteTreeW(hKey: *mut c_void, lpSubKey: *const u16) -> i32;
}

// ---------------------------------------------------------------------------
// BSTR-хелперы
// ---------------------------------------------------------------------------

fn bstr_to_string(bstr: *mut u16) -> String {
    if bstr.is_null() {
        return String::new();
    }
    unsafe {
        let len = SysStringLen(bstr) as usize;
        String::from_utf16_lossy(std::slice::from_raw_parts(bstr, len))
    }
}

fn string_to_bstr(s: &str) -> *mut u16 {
    let wide: Vec<u16> = s.encode_utf16().collect();
    unsafe { SysAllocStringLen(wide.as_ptr(), wide.len() as u32) }
}

fn wide_nul(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

// ---------------------------------------------------------------------------
// VARIANT-хелперы
// ---------------------------------------------------------------------------

fn variant_init(v: *mut VARIANT) {
    unsafe { VariantInit(v) };
}

fn variant_set_bstr(v: *mut VARIANT, s: &str) -> i32 {
    let bstr = string_to_bstr(s);
    if bstr.is_null() {
        return E_OUTOFMEMORY;
    }
    unsafe {
        (*v).vt = VT_BSTR;
        (*v).data1 = bstr as usize;
        (*v).data2 = 0;
    }
    S_OK
}

fn variant_set_i4(v: *mut VARIANT, value: i32) {
    unsafe {
        (*v).vt = VT_I4;
        (*v).data1 = (value as u32) as usize;
        (*v).data2 = 0;
    }
}

const VT_BYREF: u16 = 0x4000;

/// Снятие VT_BYREF-алиасов (VBScript/1C передают переменные как
/// `VT_VARIANT|VT_BYREF` или `VT_<T>|VT_BYREF`). Возвращает (vt, data1, data2).
/// Все наши параметры — только [in], запись через ссылку не нужна.
fn resolve_alias(v: &VARIANT) -> (u16, usize, usize) {
    let (mut vt, mut d1, mut d2) = (v.vt, v.data1, v.data2);
    for _ in 0..8 {
        if vt & VT_BYREF == 0 || d1 == 0 {
            break;
        }
        let base = vt & !VT_BYREF;
        unsafe {
            match base {
                VT_VARIANT => {
                    let inner = &*(d1 as *const VARIANT);
                    vt = inner.vt;
                    d1 = inner.data1;
                    d2 = inner.data2;
                }
                VT_BSTR => {
                    vt = VT_BSTR;
                    d1 = *(d1 as *const usize);
                    d2 = 0;
                }
                VT_I2 | VT_BOOL => {
                    vt = base;
                    d1 = *(d1 as *const u16) as usize;
                    d2 = 0;
                }
                VT_I4 | VT_UI4 => {
                    vt = base;
                    d1 = *(d1 as *const u32) as usize;
                    d2 = 0;
                }
                VT_R8 => {
                    let x = *(d1 as *const u64);
                    vt = base;
                    d1 = x as usize;
                    d2 = (x >> 32) as usize;
                }
                _ => break,
            }
        }
    }
    if d1 == 0 && vt & VT_BYREF != 0 {
        return (VT_EMPTY, 0, 0);
    }
    (vt, d1, d2)
}

/// Строка из VARIANT (BSTR/числа/bool; EMPTY/NULL → "").
fn variant_to_string(v: &VARIANT) -> Result<String, i32> {
    let (vt, d1, d2) = resolve_alias(v);
    let s = match vt {
        VT_EMPTY | VT_NULL => String::new(),
        VT_BSTR => bstr_to_string(d1 as *mut u16),
        VT_I2 => (d1 as u16 as i16).to_string(),
        VT_I4 => (d1 as u32 as i32).to_string(),
        VT_UI4 => (d1 as u32).to_string(),
        VT_R8 => {
            let f = f64::from_bits((d1 as u64) | ((d2 as u64) << 32));
            if f.fract() == 0.0 && f.is_finite() {
                (f as i64).to_string()
            } else {
                f.to_string()
            }
        }
        VT_BOOL => {
            if (d1 as u16) as i16 == 0 {
                "False".to_owned()
            } else {
                "True".to_owned()
            }
        }
        _ => return Err(DISP_E_TYPEMISMATCH),
    };
    Ok(s)
}

/// Целое из VARIANT (числа + разбор BSTR; 1C присылает числа как VT_R8).
fn variant_to_i32(v: &VARIANT) -> Result<i32, i32> {
    let (vt, d1, d2) = resolve_alias(v);
    match vt {
        VT_I2 => Ok(d1 as u16 as i16 as i32),
        VT_I4 => Ok(d1 as u32 as i32),
        VT_UI4 => Ok(d1 as u32 as i32),
        VT_R8 => {
            let f = f64::from_bits((d1 as u64) | ((d2 as u64) << 32));
            Ok(f as i32)
        }
        VT_BOOL => Ok(if (d1 as u16) as i16 == 0 { 0 } else { 1 }),
        VT_BSTR => {
            let s = bstr_to_string(d1 as *mut u16);
            let t = s.trim();
            t.parse::<i32>()
                .or_else(|_| t.parse::<f64>().map(|f| f as i32))
                .map_err(|_| DISP_E_TYPEMISMATCH)
        }
        _ => Err(DISP_E_TYPEMISMATCH),
    }
}

// ---------------------------------------------------------------------------
// IUnknown / IDispatch / IClassFactory
// ---------------------------------------------------------------------------

static mut DLL_INSTANCE: *mut c_void = ptr::null_mut();

static DECLENSION_VTBL: IDispatchVtbl = IDispatchVtbl {
    base: IUnknownVtbl {
        QueryInterface: declension_query_interface,
        AddRef: declension_add_ref,
        Release: declension_release,
    },
    GetTypeInfoCount: dispatch_get_type_info_count,
    GetTypeInfo: dispatch_get_type_info,
    GetIDsOfNames: dispatch_get_ids_of_names,
    Invoke: dispatch_invoke,
};

static FACTORY_VTBL: IClassFactoryVtbl = IClassFactoryVtbl {
    base: IUnknownVtbl {
        QueryInterface: factory_query_interface,
        AddRef: factory_add_ref,
        Release: factory_release,
    },
    CreateInstance: factory_create_instance,
    LockServer: factory_lock_server,
};

/// Статическая фабрика (без состояния; AddRef/Release — no-op).
/// ВАЖНО: указатель интерфейса обязан указывать на ОБЪЕКТ, чьё первое поле —
/// указатель vtable (ole32 разыменовывает ppv как объект). Возврат адреса
/// самой vtable напрямую даёт wild jump — отловлено VEH-отладкой.
#[repr(C)]
struct ClassFactory {
    vtbl: *const IClassFactoryVtbl,
}

// Безопасно: vtable только читается (указатели на функции), мутабельного
// состояния у фабрики нет.
unsafe impl Sync for ClassFactory {}

static FACTORY_OBJ: ClassFactory = ClassFactory {
    vtbl: &FACTORY_VTBL,
};

fn declension_refs(this: *mut c_void) -> &'static AtomicU32 {
    unsafe { &(*(this as *mut Declension)).refs }
}

unsafe extern "system" fn declension_query_interface(
    this: *mut c_void,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> i32 {
    if ppv.is_null() || riid.is_null() {
        return E_POINTER;
    }
    unsafe {
        if *riid == IID_IUNKNOWN || *riid == IID_IDISPATCH {
            declension_add_ref(this);
            *ppv = this;
            S_OK
        } else {
            *ppv = ptr::null_mut();
            E_NOINTERFACE
        }
    }
}

unsafe extern "system" fn declension_add_ref(this: *mut c_void) -> u32 {
    declension_refs(this).fetch_add(1, Ordering::Relaxed) + 1
}

unsafe extern "system" fn declension_release(this: *mut c_void) -> u32 {
    let refs = declension_refs(this).fetch_sub(1, Ordering::Release) - 1;
    if refs == 0 {
        unsafe {
            let _ = Box::from_raw(this as *mut Declension);
        }
    }
    refs
}

unsafe extern "system" fn dispatch_get_type_info_count(
    _this: *mut c_void,
    pctinfo: *mut u32,
) -> i32 {
    if pctinfo.is_null() {
        return E_POINTER;
    }
    unsafe {
        *pctinfo = 0;
    }
    S_OK
}

unsafe extern "system" fn dispatch_get_type_info(
    _this: *mut c_void,
    _i: u32,
    _lcid: u32,
    _pp: *mut *mut c_void,
) -> i32 {
    E_NOTIMPL
}

unsafe extern "system" fn dispatch_get_ids_of_names(
    _this: *mut c_void,
    _riid: *const GUID,
    rgsz_names: *mut *mut u16,
    c_names: u32,
    _lcid: u32,
    rgdispid: *mut i32,
) -> i32 {
    if rgsz_names.is_null() || rgdispid.is_null() {
        return E_POINTER;
    }
    if c_names != 1 {
        return DISP_E_UNKNOWNNAME;
    }
    // rgszNames — это LPWSTR (не BSTR!): читаем до нулевого символа.
    let name = unsafe {
        let base = *rgsz_names;
        let mut len = 0usize;
        while len < 256 && *base.add(len) != 0 {
            len += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(base, len))
    }
    .to_uppercase();
    let dispid = match name.as_str() {
        "GETSEX" => DISPID_GETSEX,
        "GETFIOPADEGFS" => DISPID_GETFIOPADEGFS,
        "GETNOMINATIVEPADEG" => DISPID_GETNOMINATIVEPADEG,
        "GETAPPOINTMENTPADEG" => DISPID_GETAPPOINTMENTPADEG,
        _ => return DISP_E_UNKNOWNNAME,
    };
    unsafe {
        *rgdispid = dispid;
    }
    S_OK
}

fn fill_excepinfo(excep: *mut EXCEPINFO, message: &str) {
    if excep.is_null() {
        return;
    }
    unsafe {
        (*excep).wCode = 1001;
        (*excep).wReserved = 0;
        (*excep).bstrSource = string_to_bstr(PROGID_PADEG);
        (*excep).bstrDescription = string_to_bstr(message);
        (*excep).bstrHelpFile = ptr::null_mut();
        (*excep).dwHelpContext = 0;
        (*excep).pvReserved = ptr::null_mut();
        (*excep).pfnDeferredFillIn = ptr::null_mut();
        (*excep).scode = E_INVALIDARG;
    }
}

fn invoke_arg(params: *mut DISPPARAMS, index_from_left: usize, c_args: usize) -> *const VARIANT {
    // rgvarg хранит аргументы в обратном порядке.
    unsafe { (*params).rgvarg.add(c_args - 1 - index_from_left) }
}

unsafe extern "system" fn dispatch_invoke(
    _this: *mut c_void,
    dispid: i32,
    _riid: *const GUID,
    _lcid: u32,
    flags: u16,
    params: *mut DISPPARAMS,
    result: *mut VARIANT,
    excep: *mut EXCEPINFO,
    _arg_err: *mut u32,
) -> i32 {
    if flags & DISPATCH_METHOD == 0 {
        return DISP_E_MEMBERNOTFOUND;
    }
    if params.is_null() {
        return E_POINTER;
    }
    let c_args = unsafe { (*params).cArgs } as usize;
    // Именованные аргументы не поддерживаем (VBScript/1C их не шлют).
    if unsafe { (*params).cNamedArgs } != 0 {
        return DISP_E_MEMBERNOTFOUND;
    }

    enum InvokeOk {
        I4(i32),
        Str(String),
    }
    // Внутренний результат: Ok | (HRESULT, Option<текст для EXCEPINFO>).
    type Outcome = Result<InvokeOk, (i32, Option<String>)>;
    let plain = |hr: i32| (hr, None);
    let fail = |e: adapter::PadegError| (DISP_E_EXCEPTION, Some(e.0));

    let outcome: Outcome = match std::panic::catch_unwind(AssertUnwindSafe(|| {
        let get = |i: usize| unsafe { &*invoke_arg(params, i, c_args) };
        match dispid {
            DISPID_GETSEX => {
                if c_args != 1 {
                    return Err(plain(DISP_E_BADPARAMCOUNT));
                }
                let fio = variant_to_string(get(0)).map_err(plain)?;
                Ok(InvokeOk::I4(adapter::get_sex(&fio)))
            }
            DISPID_GETNOMINATIVEPADEG => {
                if c_args != 1 {
                    return Err(plain(DISP_E_BADPARAMCOUNT));
                }
                let fio = variant_to_string(get(0)).map_err(plain)?;
                Ok(InvokeOk::Str(adapter::get_nominative_padeg(&fio)))
            }
            DISPID_GETFIOPADEGFS => {
                if c_args != 3 {
                    return Err(plain(DISP_E_BADPARAMCOUNT));
                }
                let fio = variant_to_string(get(0)).map_err(plain)?;
                let sex = variant_to_string(get(1)).map_err(plain)?;
                let padeg = variant_to_i32(get(2)).map_err(plain)?;
                adapter::get_fio_padeg_fs(&fio, &sex, padeg)
                    .map(InvokeOk::Str)
                    .map_err(fail)
            }
            DISPID_GETAPPOINTMENTPADEG => {
                if c_args != 2 {
                    return Err(plain(DISP_E_BADPARAMCOUNT));
                }
                let app = variant_to_string(get(0)).map_err(plain)?;
                let padeg = variant_to_i32(get(1)).map_err(plain)?;
                adapter::get_appointment_padeg(&app, padeg)
                    .map(InvokeOk::Str)
                    .map_err(fail)
            }
            _ => Err(plain(DISP_E_MEMBERNOTFOUND)),
        }
    })) {
        Ok(outcome) => outcome,
        Err(_) => Err((E_FAIL, Some("внутренняя ошибка (panic)".to_owned()))),
    };

    match outcome {
        Err((hr, msg)) => {
            if hr == DISP_E_EXCEPTION {
                fill_excepinfo(excep, msg.as_deref().unwrap_or("ошибка склонения"));
            }
            hr
        }
        Ok(InvokeOk::I4(value)) => {
            if !result.is_null() {
                variant_init(result);
                variant_set_i4(result, value);
            }
            S_OK
        }
        Ok(InvokeOk::Str(s)) => {
            if !result.is_null() {
                variant_init(result);
                return variant_set_bstr(result, &s);
            }
            S_OK
        }
    }
}

unsafe extern "system" fn factory_query_interface(
    _this: *mut c_void,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> i32 {
    if ppv.is_null() || riid.is_null() {
        return E_POINTER;
    }
    unsafe {
        if *riid == IID_IUNKNOWN || *riid == IID_ICLASSFACTORY {
            *ppv = std::ptr::addr_of!(FACTORY_OBJ) as *mut c_void;
            S_OK
        } else {
            *ppv = ptr::null_mut();
            E_NOINTERFACE
        }
    }
}

unsafe extern "system" fn factory_add_ref(_this: *mut c_void) -> u32 {
    1
}

unsafe extern "system" fn factory_release(_this: *mut c_void) -> u32 {
    1
}

unsafe extern "system" fn factory_create_instance(
    _this: *mut c_void,
    punk_outer: *mut c_void,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> i32 {
    if ppv.is_null() || riid.is_null() {
        return E_POINTER;
    }
    if !punk_outer.is_null() {
        return CLASS_E_NOAGGREGATION;
    }
    let obj = Box::new(Declension {
        vtbl: &DECLENSION_VTBL,
        refs: AtomicU32::new(0),
    });
    let raw = Box::into_raw(obj) as *mut c_void;
    let hr = unsafe { declension_query_interface(raw, riid, ppv) };
    if hr != S_OK {
        unsafe {
            let _ = Box::from_raw(raw as *mut Declension);
        }
    }
    hr
}

unsafe extern "system" fn factory_lock_server(_this: *mut c_void, _lock: i32) -> i32 {
    S_OK
}

// ---------------------------------------------------------------------------
// Точки входа DLL
// ---------------------------------------------------------------------------

/// Точка входа COM.
///
/// # Safety
///
/// Вызывается загрузчиком COM с валидными указателями.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn DllGetClassObject(
    rclsid: *const GUID,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> i32 {
    if rclsid.is_null() || riid.is_null() || ppv.is_null() {
        return E_POINTER;
    }
    unsafe {
        if *rclsid != CLSID_DECLENSION {
            *ppv = ptr::null_mut();
            return CLASS_E_CLASSNOTAVAILABLE;
        }
        factory_query_interface(std::ptr::addr_of!(FACTORY_OBJ) as *mut c_void, riid, ppv)
    }
}

/// Точка входа COM.
///
/// # Safety
///
/// Вызывается загрузчиком COM, параметров нет.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn DllCanUnloadNow() -> i32 {
    S_FALSE
}

/// Точка входа DLL.
///
/// # Safety
///
/// Вызывается загрузчиком Windows с валидным `hinst`.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn DllMain(
    hinst: *mut c_void,
    reason: u32,
    _reserved: *mut c_void,
) -> i32 {
    const DLL_PROCESS_ATTACH: u32 = 1;
    if reason == DLL_PROCESS_ATTACH {
        unsafe {
            DLL_INSTANCE = hinst;
            DisableThreadLibraryCalls(hinst);
        }
    }
    1
}

pub fn dll_instance() -> *mut c_void {
    unsafe { DLL_INSTANCE }
}

pub fn module_path() -> Option<String> {
    let hinst = dll_instance();
    if hinst.is_null() {
        return None;
    }
    let mut buf = vec![0u16; 32768];
    let len = unsafe { GetModuleFileNameW(hinst, buf.as_mut_ptr(), buf.len() as u32) };
    if len == 0 {
        return None;
    }
    String::from_utf16(&buf[..len as usize]).ok()
}

// ---------------------------------------------------------------------------
// Регистрация — ТОЛЬКО HKCU (без прав администратора)
// ---------------------------------------------------------------------------

const HKEY_CURRENT_USER: *mut c_void = 0x80000001 as *mut c_void;
const KEY_WRITE: u32 = 0x20006;
const REG_SZ: u32 = 1;

fn reg_write_value(key_path: &str, value_name: Option<&str>, data: &str) -> Result<(), String> {
    let subkey = wide_nul(key_path);
    let mut hkey: *mut c_void = ptr::null_mut();
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            subkey.as_ptr(),
            0,
            ptr::null_mut(),
            0,
            KEY_WRITE,
            ptr::null_mut(),
            &mut hkey,
            ptr::null_mut(),
        )
    };
    if status != 0 {
        return Err(format!("RegCreateKeyExW({key_path}) = {status:#X}"));
    }
    let name_buf;
    let name_ptr = match value_name {
        Some(n) => {
            name_buf = wide_nul(n);
            name_buf.as_ptr()
        }
        None => ptr::null(),
    };
    let data_wide = wide_nul(data);
    let data_bytes = unsafe {
        std::slice::from_raw_parts(
            data_wide.as_ptr() as *const u8,
            data_wide.len() * std::mem::size_of::<u16>(),
        )
    };
    let status = unsafe {
        RegSetValueExW(
            hkey,
            name_ptr,
            0,
            REG_SZ,
            data_bytes.as_ptr(),
            data_bytes.len() as u32,
        )
    };
    unsafe {
        RegCloseKey(hkey);
    }
    if status != 0 {
        return Err(format!("RegSetValueExW({key_path}) = {status:#X}"));
    }
    Ok(())
}

fn reg_delete_tree(key_path: &str) -> Result<(), String> {
    let subkey = wide_nul(key_path);
    let status = unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, subkey.as_ptr()) };
    // 2 = ERROR_FILE_NOT_FOUND — ключа уже нет, это тоже успех.
    if status != 0 && status != 2 {
        return Err(format!("RegDeleteTreeW({key_path}) = {status:#X}"));
    }
    Ok(())
}

/// Запись всех ключей под `HKCU\Software\Classes`.
/// 32-битный вид (`Wow6432Node`) для x86-клиентов пишет установщик отдельно.
pub fn register_inproc_server(dll_path: &str) -> Result<(), String> {
    let clsid_key = format!("Software\\Classes\\CLSID\\{CLSID_DECLENSION_STR}");
    reg_write_value(&clsid_key, None, FRIENDLY_NAME)?;
    reg_write_value(&format!("{clsid_key}\\InprocServer32"), None, dll_path)?;
    reg_write_value(
        &format!("{clsid_key}\\InprocServer32"),
        Some("ThreadingModel"),
        "Apartment",
    )?;
    reg_write_value(&format!("{clsid_key}\\ProgID"), None, PROGID_PADEG)?;
    for progid in [PROGID_PADEG, PROGID_PETROVICH] {
        let progid_key = format!("Software\\Classes\\{progid}");
        reg_write_value(&progid_key, None, FRIENDLY_NAME)?;
        reg_write_value(&format!("{progid_key}\\CLSID"), None, CLSID_DECLENSION_STR)?;
    }
    Ok(())
}

pub fn unregister_inproc_server() -> Result<(), String> {
    reg_delete_tree(&format!("Software\\Classes\\CLSID\\{CLSID_DECLENSION_STR}"))?;
    // ProgID удаляем безусловно: оба принадлежат нам (см. register_inproc_server).
    for progid in [PROGID_PADEG, PROGID_PETROVICH] {
        reg_delete_tree(&format!("Software\\Classes\\{progid}"))?;
    }
    Ok(())
}

/// Точка входа COM.
///
/// # Safety
///
/// Вызывается `regsvr32`-совместимым установщиком без параметров.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn DllRegisterServer() -> i32 {
    match module_path() {
        Some(path) => match register_inproc_server(&path) {
            Ok(()) => S_OK,
            Err(_) => E_FAIL,
        },
        None => E_FAIL,
    }
}

/// Точка входа COM.
///
/// # Safety
///
/// Вызывается `regsvr32`-совместимым установщиком без параметров.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn DllUnregisterServer() -> i32 {
    match unregister_inproc_server() {
        Ok(()) => S_OK,
        Err(_) => E_FAIL,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Создать объект через фабрику и вернуть (`obj`, указатель vtbl).
    /// Хелпер тестов: `obj` указывает на структуру, vtbl лежит в её 1-м поле.
    unsafe fn make_object() -> (*mut c_void, *const IDispatchVtbl) {
        let mut factory: *mut c_void = ptr::null_mut();
        assert_eq!(
            unsafe { DllGetClassObject(&CLSID_DECLENSION, &IID_ICLASSFACTORY, &mut factory) },
            S_OK
        );
        assert!(!factory.is_null());
        let fvtbl = unsafe { (*(factory as *mut ClassFactory)).vtbl };
        let create = unsafe { (*fvtbl).CreateInstance };
        let mut obj: *mut c_void = ptr::null_mut();
        assert_eq!(
            unsafe { create(factory, ptr::null_mut(), &IID_IDISPATCH, &mut obj) },
            S_OK
        );
        assert!(!obj.is_null());
        let vtbl = unsafe { (*(obj as *mut Declension)).vtbl };
        (obj, vtbl)
    }

    unsafe fn release_object(obj: *mut c_void, vtbl: *const IDispatchVtbl) {
        let release = unsafe { (*vtbl).base.Release };
        unsafe { release(obj) };
    }

    fn empty_variant() -> VARIANT {
        VARIANT {
            vt: 0,
            wReserved1: 0,
            wReserved2: 0,
            wReserved3: 0,
            data1: 0,
            data2: 0,
        }
    }

    fn empty_excepinfo() -> EXCEPINFO {
        EXCEPINFO {
            wCode: 0,
            wReserved: 0,
            bstrSource: ptr::null_mut(),
            bstrDescription: ptr::null_mut(),
            bstrHelpFile: ptr::null_mut(),
            dwHelpContext: 0,
            pvReserved: ptr::null_mut(),
            pfnDeferredFillIn: ptr::null_mut(),
            scode: 0,
        }
    }

    #[test]
    fn byref_variants_dereference() {
        unsafe {
            // VT_VARIANT|VT_BYREF -> BSTR (как VBScript передаёт переменные).
            let text = string_to_bstr("Пр");
            let mut inner = empty_variant();
            inner.vt = VT_BSTR;
            inner.data1 = text as usize;
            let outer = VARIANT {
                vt: VT_VARIANT | VT_BYREF,
                wReserved1: 0,
                wReserved2: 0,
                wReserved3: 0,
                data1: &inner as *const VARIANT as usize,
                data2: 0,
            };
            assert_eq!(variant_to_string(&outer).unwrap(), "Пр");

            // VT_I4|VT_BYREF.
            let num: i32 = 5;
            let byref = VARIANT {
                vt: VT_I4 | VT_BYREF,
                wReserved1: 0,
                wReserved2: 0,
                wReserved3: 0,
                data1: &num as *const i32 as usize,
                data2: 0,
            };
            assert_eq!(variant_to_i32(&byref).unwrap(), 5);
            SysFreeString(text);
        }
    }

    #[test]
    fn clsid_string_is_well_formed() {
        assert_eq!(CLSID_DECLENSION_STR.len(), 38);
        assert!(CLSID_DECLENSION_STR.starts_with('{'));
        assert!(CLSID_DECLENSION_STR.ends_with('}'));
    }

    #[test]
    fn class_object_roundtrip_in_process() {
        unsafe {
            let (obj, vtbl) = make_object();

            // GetIDsOfNames регистронезависимо (как VBScript).
            let name = wide_nul("getfiopadegfs");
            let mut dispid = -1;
            let get_ids = (*vtbl).GetIDsOfNames;
            let hr = get_ids(
                obj,
                &IID_IUNKNOWN,
                [name.as_ptr() as *mut u16].as_mut_ptr(),
                1,
                0,
                &mut dispid,
            );
            assert_eq!(hr, S_OK);
            assert_eq!(dispid, DISPID_GETFIOPADEGFS);

            // Неизвестное имя → DISP_E_UNKNOWNNAME.
            let bad = wide_nul("NoSuchMethod");
            let mut dispid_bad = -1;
            let hr = get_ids(
                obj,
                &IID_IUNKNOWN,
                [bad.as_ptr() as *mut u16].as_mut_ptr(),
                1,
                0,
                &mut dispid_bad,
            );
            assert_eq!(hr, DISP_E_UNKNOWNNAME);

            // Чужой CLSID → CLASS_E_CLASSNOTAVAILABLE.
            let mut nope: *mut c_void = ptr::null_mut();
            let hr = DllGetClassObject(&IID_IUNKNOWN, &IID_ICLASSFACTORY, &mut nope);
            assert_eq!(hr, CLASS_E_CLASSNOTAVAILABLE);

            release_object(obj, vtbl);
        }
    }

    #[test]
    fn invoke_get_sex_end_to_end() {
        unsafe {
            let (obj, vtbl) = make_object();

            // GetSex("Иванов Иван Иванович") == 1
            let fio = string_to_bstr("Иванов Иван Иванович");
            let mut arg = empty_variant();
            VariantInit(&mut arg);
            arg.vt = VT_BSTR;
            arg.data1 = fio as usize;
            let mut params = DISPPARAMS {
                rgvarg: &mut arg,
                rgdispidNamedArgs: ptr::null_mut(),
                cArgs: 1,
                cNamedArgs: 0,
            };
            let mut result = empty_variant();
            let mut excep = empty_excepinfo();
            let invoke = (*vtbl).Invoke;
            let hr = invoke(
                obj,
                DISPID_GETSEX,
                &IID_IUNKNOWN,
                0,
                DISPATCH_METHOD,
                &mut params,
                &mut result,
                &mut excep,
                ptr::null_mut(),
            );
            assert_eq!(hr, S_OK);
            assert_eq!(result.vt, VT_I4);
            assert_eq!(result.data1 as u32 as i32, 1);
            SysFreeString(fio);

            // GetFIOPadegFS с неверным числом аргументов → BADPARAMCOUNT.
            let hr = invoke(
                obj,
                DISPID_GETFIOPADEGFS,
                &IID_IUNKNOWN,
                0,
                DISPATCH_METHOD,
                &mut params,
                &mut result,
                &mut excep,
                ptr::null_mut(),
            );
            assert_eq!(hr, DISP_E_BADPARAMCOUNT);

            release_object(obj, vtbl);
        }
    }

    #[test]
    fn invoke_fio_and_appointment_strings() {
        unsafe {
            let (obj, vtbl) = make_object();
            let invoke = (*vtbl).Invoke;

            // GetFIOPadegFS("Иванов Иван Иванович", "", 3) → дательный.
            let fio = string_to_bstr("Иванов Иван Иванович");
            let sex = string_to_bstr("");
            let mut args = [empty_variant(), empty_variant(), empty_variant()];
            // rgvarg — в обратном порядке: [padeg, sex, fio].
            args[0].vt = VT_I4;
            args[0].data1 = 3;
            args[1].vt = VT_BSTR;
            args[1].data1 = sex as usize;
            args[2].vt = VT_BSTR;
            args[2].data1 = fio as usize;
            let mut params = DISPPARAMS {
                rgvarg: args.as_mut_ptr(),
                rgdispidNamedArgs: ptr::null_mut(),
                cArgs: 3,
                cNamedArgs: 0,
            };
            let mut result = empty_variant();
            let mut excep = empty_excepinfo();
            let hr = invoke(
                obj,
                DISPID_GETFIOPADEGFS,
                &IID_IUNKNOWN,
                0,
                DISPATCH_METHOD,
                &mut params,
                &mut result,
                &mut excep,
                ptr::null_mut(),
            );
            assert_eq!(hr, S_OK);
            assert_eq!(result.vt, VT_BSTR);
            assert_eq!(
                bstr_to_string(result.data1 as *mut u16),
                "Иванову Ивану Ивановичу"
            );
            SysFreeString(result.data1 as *mut u16);
            SysFreeString(fio);
            SysFreeString(sex);

            release_object(obj, vtbl);
        }
    }
}
