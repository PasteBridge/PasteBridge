// Windows OLE Drag-and-Drop implementation for dragging text out of PasteBridge
// to external applications (e.g., Notepad, browser text fields).
//
// Uses raw COM/Win32 FFI to avoid dependency on the windows crate's OLE bindings.

#![cfg(target_os = "windows")]

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::sync::Mutex;

// COM and Win32 types
type HRESULT = i32;
type BOOL = i32;
type LPVOID = *mut c_void;
type DWORD = u32;
type UINT = u32;
type WORD = u16;
type LONG = i32;
type HGLOBAL = *mut c_void;
type CLIPFORMAT = WORD;

// ── 全局状态：用于 OLE 拖拽中检测鼠标是否回到窗口 ──
/// 存储应用窗口句柄，query_continue_drag 中用于判断鼠标是否回到窗口内
static APP_HWND: AtomicIsize = AtomicIsize::new(0);
/// 标记 OLE 拖拽是否因鼠标回到窗口而被取消
static REENTRY_CANCELLED: AtomicBool = AtomicBool::new(false);

const S_OK: HRESULT = 0;
const S_FALSE: HRESULT = 1;
const E_NOTIMPL: HRESULT = -2147467263;
const E_OUTOFMEMORY: HRESULT = -2147024882;
const E_UNEXPECTED: HRESULT = -2147418113;
const DV_E_FORMATETC: HRESULT = -2147221404;
const DV_E_DVASPECT: HRESULT = -2147221403;
const DATA_S_SAMEFORMATETC: HRESULT = 0x00040130;
const DRAGDROP_S_DROP: HRESULT = 0x00040100;
const DRAGDROP_S_CANCEL: HRESULT = 0x00040101;
const DRAGDROP_S_USEDEFAULTCURSORS: HRESULT = 0x00040102;

const DROPEFFECT_NONE: DWORD = 0;
const DROPEFFECT_COPY: DWORD = 1;

const TYMED_HGLOBAL: DWORD = 1;
const TYMED_NULL: DWORD = 0;
const DVASPECT_CONTENT: DWORD = 1;
const GMEM_MOVEABLE: DWORD = 0x0002;
const GMEM_ZEROINIT: DWORD = 0x0040;
const CF_TEXT: CLIPFORMAT = 1;
const CF_UNICODETEXT: CLIPFORMAT = 13;

// FORMATETC
#[repr(C)]
#[derive(Clone, Copy)]
struct FORMATETC {
    cfFormat: CLIPFORMAT,
    ptd: *mut DVTARGETDEVICE,
    dwAspect: DWORD,
    lindex: LONG,
    tymed: DWORD,
}

impl Default for FORMATETC {
    fn default() -> Self {
        FORMATETC {
            cfFormat: 0,
            ptd: std::ptr::null_mut(),
            dwAspect: DVASPECT_CONTENT,
            lindex: -1,
            tymed: TYMED_HGLOBAL,
        }
    }
}

#[repr(C)]
struct DVTARGETDEVICE {
    tdSize: DWORD,
}

// STGMEDIUM
#[repr(C)]
struct STGMEDIUM {
    tymed: DWORD,
    u: STGMEDIUM_UNION,
    pUnkForRelease: *mut IUnknown,
}

#[repr(C)]
union STGMEDIUM_UNION {
    hGlobal: HGLOBAL,
    hBitmap: *mut c_void,
    hMetaFilePict: *mut c_void,
    hEnhMetaFile: *mut c_void,
    lpszFileName: *mut u16,
    pstm: *mut c_void,
    pstg: *mut c_void,
}

impl Default for STGMEDIUM {
    fn default() -> Self {
        STGMEDIUM {
            tymed: TYMED_NULL,
            u: STGMEDIUM_UNION { hGlobal: std::ptr::null_mut() },
            pUnkForRelease: std::ptr::null_mut(),
        }
    }
}

// IUnknown
#[repr(C)]
struct IUnknown {
    lpVtbl: *const IUnknownVtbl,
}

#[repr(C)]
struct IUnknownVtbl {
    query_interface: unsafe extern "system" fn(This: *mut IUnknown, riid: *const GUID, ppvObject: *mut *mut c_void) -> HRESULT,
    add_ref: unsafe extern "system" fn(This: *mut IUnknown) -> u32,
    release: unsafe extern "system" fn(This: *mut IUnknown) -> u32,
}

// GUID
#[repr(C)]
struct GUID {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}

// IDataObject VTable
#[repr(C)]
struct IDataObjectVtbl {
    query_interface: unsafe extern "system" fn(This: *mut IDataObject, riid: *const GUID, ppvObject: *mut *mut c_void) -> HRESULT,
    add_ref: unsafe extern "system" fn(This: *mut IDataObject) -> u32,
    release: unsafe extern "system" fn(This: *mut IDataObject) -> u32,
    get_data: unsafe extern "system" fn(This: *mut IDataObject, pformatetc: *const FORMATETC, pmedium: *mut STGMEDIUM) -> HRESULT,
    get_data_here: unsafe extern "system" fn(This: *mut IDataObject, pformatetc: *const FORMATETC, pmedium: *mut STGMEDIUM) -> HRESULT,
    query_get_data: unsafe extern "system" fn(This: *mut IDataObject, pformatetc: *const FORMATETC) -> HRESULT,
    get_canonical_format_etc: unsafe extern "system" fn(This: *mut IDataObject, pformatectIn: *const FORMATETC, pformatetcOut: *mut FORMATETC) -> HRESULT,
    set_data: unsafe extern "system" fn(This: *mut IDataObject, pformatetc: *const FORMATETC, pmedium: *const STGMEDIUM, fRelease: BOOL) -> HRESULT,
    enum_format_etc: unsafe extern "system" fn(This: *mut IDataObject, dwDirection: DWORD, ppenumFormatEtc: *mut *mut c_void) -> HRESULT,
    d_advise: unsafe extern "system" fn(This: *mut IDataObject, pformatetc: *const FORMATETC, advf: DWORD, pAdvSink: *mut IUnknown, pdwConnection: *mut DWORD) -> HRESULT,
    d_unadvise: unsafe extern "system" fn(This: *mut IDataObject, dwConnection: DWORD) -> HRESULT,
    enum_d_advise: unsafe extern "system" fn(This: *mut IDataObject, ppenumAdvise: *mut *mut c_void) -> HRESULT,
}

// IDataObject interface pointer (the ptr passed to COM)
#[repr(C)]
struct IDataObject {
    lpVtbl: *const IDataObjectVtbl,
}

// IDropSource VTable
#[repr(C)]
struct IDropSourceVtbl {
    query_interface: unsafe extern "system" fn(This: *mut IDropSource, riid: *const GUID, ppvObject: *mut *mut c_void) -> HRESULT,
    add_ref: unsafe extern "system" fn(This: *mut IDropSource) -> u32,
    release: unsafe extern "system" fn(This: *mut IDropSource) -> u32,
    query_continue_drag: unsafe extern "system" fn(This: *mut IDropSource, fEscapePressed: BOOL, grfKeyState: DWORD) -> HRESULT,
    give_feedback: unsafe extern "system" fn(This: *mut IDropSource, dwEffect: DWORD) -> HRESULT,
}

// IDropSource interface pointer
#[repr(C)]
struct IDropSource {
    lpVtbl: *const IDropSourceVtbl,
}

// DragDataObject - the COM object layout.
// CRITICAL: lpVtbl MUST be the first field so that a pointer to this struct
// is also a valid IDataObject* pointer.
#[repr(C)]
struct DragDataObject {
    lpVtbl: *const IDataObjectVtbl,
    ref_count: Mutex<u32>,
    text: Mutex<String>,
}

// DragDropSource - the COM object layout.
#[repr(C)]
struct DragDropSource {
    lpVtbl: *const IDropSourceVtbl,
    ref_count: Mutex<u32>,
}

// External functions
extern "system" {
    fn OleInitialize(pvReserved: LPVOID) -> HRESULT;
    fn OleUninitialize();
    fn DoDragDrop(
        pDataObj: *mut IDataObject,
        pDropSource: *mut IDropSource,
        dwOKEffects: DWORD,
        pdwEffect: *mut DWORD,
    ) -> HRESULT;
    fn GlobalAlloc(uFlags: UINT, dwBytes: usize) -> HGLOBAL;
    fn GlobalLock(hMem: HGLOBAL) -> LPVOID;
    fn GlobalUnlock(hMem: HGLOBAL) -> BOOL;
    fn GlobalFree(hMem: HGLOBAL) -> HGLOBAL;
    fn PostMessageW(hWnd: isize, Msg: u32, wParam: usize, lParam: isize) -> BOOL;
    fn GetCursorPos(lpPoint: *mut POINT) -> BOOL;
    fn ScreenToClient(hWnd: isize, lpPoint: *mut POINT) -> BOOL;
    fn GetClientRect(hWnd: isize, lpRect: *mut i32) -> BOOL;
}

#[repr(C)]
#[derive(Clone, Copy)]
struct POINT {
    x: LONG,
    y: LONG,
}

const WM_LBUTTONUP: u32 = 0x0202;

// ── IID constants ──
const IID_IUNKNOWN: GUID = GUID {
    data1: 0x00000000, data2: 0x0000, data3: 0x0000,
    data4: [0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
};
const IID_IDATAOBJECT: GUID = GUID {
    data1: 0x0000010E, data2: 0x0000, data3: 0x0000,
    data4: [0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
};
const IID_IDROPSOURCE: GUID = GUID {
    data1: 0x00000121, data2: 0x0000, data3: 0x0000,
    data4: [0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
};

fn guid_eq(a: &GUID, b: &GUID) -> bool {
    a.data1 == b.data1 && a.data2 == b.data2 && a.data3 == b.data3 && a.data4 == b.data4
}

// ── Static vtables ──
static DATA_OBJECT_VTBL: IDataObjectVtbl = IDataObjectVtbl {
    query_interface: data_object_query_interface,
    add_ref: data_object_add_ref,
    release: data_object_release,
    get_data: data_object_get_data,
    get_data_here: data_object_get_data_here,
    query_get_data: data_object_query_get_data,
    get_canonical_format_etc: data_object_get_canonical_format_etc,
    set_data: data_object_set_data,
    enum_format_etc: data_object_enum_format_etc,
    d_advise: data_object_d_advise,
    d_unadvise: data_object_d_unadvise,
    enum_d_advise: data_object_enum_d_advise,
};

static DROP_SOURCE_VTBL: IDropSourceVtbl = IDropSourceVtbl {
    query_interface: drop_source_query_interface,
    add_ref: drop_source_add_ref,
    release: drop_source_release,
    query_continue_drag: drop_source_query_continue_drag,
    give_feedback: drop_source_give_feedback,
};

// ── IDataObject methods ──

unsafe extern "system" fn data_object_query_interface(
    this: *mut IDataObject,
    riid: *const GUID,
    ppvObject: *mut *mut c_void,
) -> HRESULT {
    if ppvObject.is_null() {
        return E_UNEXPECTED;
    }
    let riid = &*riid;
    if guid_eq(riid, &IID_IUNKNOWN) || guid_eq(riid, &IID_IDATAOBJECT) {
        *ppvObject = this as *mut c_void;
        data_object_add_ref(this);
        S_OK
    } else {
        *ppvObject = std::ptr::null_mut();
        0x80004002u32 as i32 // E_NOINTERFACE
    }
}

unsafe extern "system" fn data_object_add_ref(this: *mut IDataObject) -> u32 {
    let obj = &*(this as *mut DragDataObject);
    let mut count = obj.ref_count.lock().unwrap();
    *count += 1;
    *count
}

unsafe extern "system" fn data_object_release(this: *mut IDataObject) -> u32 {
    let obj = &*(this as *mut DragDataObject);
    let mut count = obj.ref_count.lock().unwrap();
    *count -= 1;
    let c = *count;
    if c == 0 {
        drop(count);
        let _ = Box::from_raw(this as *mut DragDataObject);
    }
    c
}

unsafe extern "system" fn data_object_get_data(
    this: *mut IDataObject,
    pformatetc: *const FORMATETC,
    pmedium: *mut STGMEDIUM,
) -> HRESULT {
    if pformatetc.is_null() || pmedium.is_null() {
        return E_UNEXPECTED;
    }
    let formatetc = &*pformatetc;
    let obj = &*(this as *mut DragDataObject);

    if formatetc.cfFormat != CF_UNICODETEXT && formatetc.cfFormat != CF_TEXT {
        return DV_E_FORMATETC;
    }
    if formatetc.dwAspect != DVASPECT_CONTENT {
        return DV_E_DVASPECT;
    }

    let text = obj.text.lock().unwrap();
    let is_unicode = formatetc.cfFormat == CF_UNICODETEXT;

    let (data_ptr, _byte_len) = if is_unicode {
        let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        let byte_len = wide.len() * 2;
        let hglobal = GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, byte_len);
        if hglobal.is_null() {
            return E_OUTOFMEMORY;
        }
        let ptr = GlobalLock(hglobal);
        if ptr.is_null() {
            GlobalFree(hglobal);
            return E_OUTOFMEMORY;
        }
        std::ptr::copy_nonoverlapping(wide.as_ptr() as *const u8, ptr as *mut u8, byte_len);
        GlobalUnlock(hglobal);
        (hglobal, byte_len)
    } else {
        let ansi: Vec<u8> = text.bytes().chain(std::iter::once(0)).collect();
        let byte_len = ansi.len();
        let hglobal = GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, byte_len);
        if hglobal.is_null() {
            return E_OUTOFMEMORY;
        }
        let ptr = GlobalLock(hglobal);
        if ptr.is_null() {
            GlobalFree(hglobal);
            return E_OUTOFMEMORY;
        }
        std::ptr::copy_nonoverlapping(ansi.as_ptr(), ptr as *mut u8, byte_len);
        GlobalUnlock(hglobal);
        (hglobal, byte_len)
    };

    let medium = &mut *pmedium;
    medium.tymed = TYMED_HGLOBAL;
    medium.u.hGlobal = data_ptr;
    medium.pUnkForRelease = std::ptr::null_mut();
    S_OK
}

unsafe extern "system" fn data_object_get_data_here(
    _this: *mut IDataObject,
    _pformatetc: *const FORMATETC,
    _pmedium: *mut STGMEDIUM,
) -> HRESULT {
    E_NOTIMPL
}

unsafe extern "system" fn data_object_query_get_data(
    _this: *mut IDataObject,
    pformatetc: *const FORMATETC,
) -> HRESULT {
    if pformatetc.is_null() {
        return E_UNEXPECTED;
    }
    let fmt = &*pformatetc;
    if fmt.cfFormat == CF_UNICODETEXT || fmt.cfFormat == CF_TEXT {
        S_OK
    } else {
        S_FALSE
    }
}

unsafe extern "system" fn data_object_get_canonical_format_etc(
    _this: *mut IDataObject,
    _pformatectIn: *const FORMATETC,
    pformatetcOut: *mut FORMATETC,
) -> HRESULT {
    if !pformatetcOut.is_null() {
        *pformatetcOut = FORMATETC::default();
    }
    DATA_S_SAMEFORMATETC
}

unsafe extern "system" fn data_object_set_data(
    _this: *mut IDataObject,
    _pformatetc: *const FORMATETC,
    _pmedium: *const STGMEDIUM,
    _fRelease: BOOL,
) -> HRESULT {
    E_NOTIMPL
}

unsafe extern "system" fn data_object_enum_format_etc(
    _this: *mut IDataObject,
    _dwDirection: DWORD,
    _ppenumFormatEtc: *mut *mut c_void,
) -> HRESULT {
    E_NOTIMPL
}

unsafe extern "system" fn data_object_d_advise(
    _this: *mut IDataObject,
    _pformatetc: *const FORMATETC,
    _advf: DWORD,
    _pAdvSink: *mut IUnknown,
    _pdwConnection: *mut DWORD,
) -> HRESULT {
    E_NOTIMPL
}

unsafe extern "system" fn data_object_d_unadvise(
    _this: *mut IDataObject,
    _dwConnection: DWORD,
) -> HRESULT {
    E_NOTIMPL
}

unsafe extern "system" fn data_object_enum_d_advise(
    _this: *mut IDataObject,
    _ppenumAdvise: *mut *mut c_void,
) -> HRESULT {
    E_NOTIMPL
}

// ── IDropSource methods ──

unsafe extern "system" fn drop_source_query_interface(
    this: *mut IDropSource,
    riid: *const GUID,
    ppvObject: *mut *mut c_void,
) -> HRESULT {
    if ppvObject.is_null() {
        return E_UNEXPECTED;
    }
    let riid = &*riid;
    if guid_eq(riid, &IID_IUNKNOWN) || guid_eq(riid, &IID_IDROPSOURCE) {
        *ppvObject = this as *mut c_void;
        drop_source_add_ref(this);
        S_OK
    } else {
        *ppvObject = std::ptr::null_mut();
        0x80004002u32 as i32
    }
}

unsafe extern "system" fn drop_source_add_ref(this: *mut IDropSource) -> u32 {
    let obj = &*(this as *mut DragDropSource);
    let mut count = obj.ref_count.lock().unwrap();
    *count += 1;
    *count
}

unsafe extern "system" fn drop_source_release(this: *mut IDropSource) -> u32 {
    let obj = &*(this as *mut DragDropSource);
    let mut count = obj.ref_count.lock().unwrap();
    *count -= 1;
    let c = *count;
    if c == 0 {
        drop(count);
        let _ = Box::from_raw(this as *mut DragDropSource);
    }
    c
}

unsafe extern "system" fn drop_source_query_continue_drag(
    _this: *mut IDropSource,
    fEscapePressed: BOOL,
    grfKeyState: DWORD,
) -> HRESULT {
    // Escape key pressed → cancel
    if fEscapePressed != 0 {
        return DRAGDROP_S_CANCEL;
    }
    // Right mouse button → cancel (standard Windows behavior)
    if grfKeyState & 0x0002 != 0 {
        return DRAGDROP_S_CANCEL;
    }
    // Middle mouse button → cancel
    if grfKeyState & 0x0010 != 0 {
        return DRAGDROP_S_CANCEL;
    }

    // ── 检测鼠标是否回到应用窗口内 ──
    // 如果用户拖出窗口后立即拖回，取消 OLE 拖拽，让 Slint 继续处理
    let hwnd = APP_HWND.load(Ordering::Relaxed);
    if hwnd != 0 {
        let mut pt = POINT { x: 0, y: 0 };
        if GetCursorPos(&mut pt) != 0 {
            let mut client_pt = pt;
            if ScreenToClient(hwnd, &mut client_pt) != 0 {
                let mut rect = [0i32; 4];
                if GetClientRect(hwnd, rect.as_mut_ptr()) != 0 {
                    let in_window = client_pt.x >= 0 && client_pt.x < rect[2]
                        && client_pt.y >= 0 && client_pt.y < rect[3];

                    if in_window {
                        eprintln!(
                            "[drag-out] QC reentry: screen=({},{}), client=({},{}), rect=({},{}), keys={:x}",
                            pt.x, pt.y, client_pt.x, client_pt.y,
                            rect[2], rect[3], grfKeyState
                        );
                        REENTRY_CANCELLED.store(true, Ordering::Relaxed);
                        return DRAGDROP_S_CANCEL;
                    }
                }
            }
        }
    }

    // Left mouse button released → drop
    // MK_LBUTTON = 0x0001 — this is the reliable state from DoDragDrop's message loop
    if grfKeyState & 0x0001 == 0 {
        return DRAGDROP_S_DROP;
    }
    S_OK
}

unsafe extern "system" fn drop_source_give_feedback(
    _this: *mut IDropSource,
    _dwEffect: DWORD,
) -> HRESULT {
    DRAGDROP_S_USEDEFAULTCURSORS
}

// ── Public API ──

/// Start a drag-and-drop operation with the given text.
/// Blocking call - runs its own message loop, returns when drag completes.
pub fn start_drag_drop(text: &str) {
    unsafe {
        // Use OleInitialize (required for DoDragDrop) instead of plain CoInitializeEx
        let hr = OleInitialize(std::ptr::null_mut());
        let com_initialized = hr >= 0;

        // Create DataObject
        let data_obj = Box::new(DragDataObject {
            lpVtbl: &DATA_OBJECT_VTBL as *const IDataObjectVtbl,
            ref_count: Mutex::new(1),
            text: Mutex::new(text.to_string()),
        });
        let data_obj_ptr = Box::into_raw(data_obj) as *mut IDataObject;

        // Create DropSource
        let drop_src = Box::new(DragDropSource {
            lpVtbl: &DROP_SOURCE_VTBL as *const IDropSourceVtbl,
            ref_count: Mutex::new(1),
        });
        let drop_src_ptr = Box::into_raw(drop_src) as *mut IDropSource;

        // Perform the drag
        let mut effect: DWORD = 0;
        let result = DoDragDrop(data_obj_ptr, drop_src_ptr, DROPEFFECT_COPY, &mut effect);

        match result {
            DRAGDROP_S_DROP => eprintln!("[drag-out] Drop completed, effect: {:x}", effect),
            DRAGDROP_S_CANCEL => eprintln!("[drag-out] Drag cancelled by user"),
            S_OK => eprintln!("[drag-out] Drag completed, effect: {:x}", effect),
            _ => eprintln!("[drag-out] Drag finished: {:x}", result),
        }

        // Release references
        data_object_release(data_obj_ptr);
        drop_source_release(drop_src_ptr);

        if com_initialized {
            OleUninitialize();
        }
    }
}

/// Reset Slint's internal mouse state after DoDragDrop.
/// DoDragDrop's modal message loop consumes WM_LBUTTONUP, leaving
/// Slint with ta.pressed = true.  Posting a synthetic WM_LBUTTONUP
/// lets Slint properly detect the button release and restore hover.
pub fn reset_mouse_state(hwnd: isize) {
    unsafe {
        let mut pt = POINT { x: 0, y: 0 };
        if GetCursorPos(&mut pt) == 0 {
            return;
        }
        if ScreenToClient(hwnd, &mut pt) == 0 {
            return;
        }
        let lparam = ((pt.y as i64 & 0xFFFF) << 16) | (pt.x as i64 & 0xFFFF);
        PostMessageW(hwnd, WM_LBUTTONUP, 0, lparam as isize);
    }
}

/// 设置应用窗口句柄，供 query_continue_drag 检测鼠标是否回到窗口
pub fn set_app_hwnd(hwnd: isize) {
    APP_HWND.store(hwnd, Ordering::Relaxed);
}

/// 检查 OLE 拖拽是否因鼠标回到窗口而被取消
pub fn was_reentry_cancelled() -> bool {
    REENTRY_CANCELLED.load(Ordering::Relaxed)
}

/// 清除重入取消标记
pub fn clear_reentry_flag() {
    REENTRY_CANCELLED.store(false, Ordering::Relaxed);
}