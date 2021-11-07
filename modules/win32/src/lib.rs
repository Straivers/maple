pub use windows::Win32::{
    Foundation::{GetLastError, HINSTANCE, HWND, LPARAM, LRESULT, PSTR, PWSTR, RECT, WPARAM},
    System::{
        Diagnostics::Debug::{SetErrorMode, SEM_FAILCRITICALERRORS},
        LibraryLoader::{GetModuleHandleW, GetProcAddress, LoadLibraryW},
    },
    UI::{
        HiDpi::GetProcessDpiAwareness,
        WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, GetWindowLongPtrW,
            GetWindowRect, LoadCursorW, PeekMessageW, PostQuitMessage, RegisterClassW, SetWindowLongPtrW, ShowWindow,
            TranslateMessage, CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, GWLP_USERDATA, IDC_ARROW, MSG,
            PM_REMOVE, SW_SHOW, WHEEL_DELTA, WINDOW_EX_STYLE, WM_CHAR, WM_CLOSE, WM_CREATE, WM_DESTROY, WM_ERASEBKGND,
            WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEHWHEEL, WM_MOUSEMOVE, WM_MOUSEWHEEL,
            WM_NULL, WM_PAINT, WM_QUIT, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SIZE, WNDCLASSW, WS_OVERLAPPEDWINDOW,
        },
    },
};
