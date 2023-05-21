use core::ffi;
use std::{io, ptr, slice};
use widestring::U16CString;
use windows_sys::Win32::{
    Foundation::{self, CloseHandle},
    Security::{DuplicateTokenEx, SecurityImpersonation, TokenPrimary},
    System::{
        Environment::{CreateEnvironmentBlock, DestroyEnvironmentBlock},
        RemoteDesktop::{
            WTSActive, WTSEnumerateSessionsW, WTSFreeMemory, WTSGetActiveConsoleSessionId,
            WTSQueryUserToken, WTS_CURRENT_SERVER_HANDLE,
        },
        Threading::{
            CreateProcessAsUserW, CREATE_NEW_CONSOLE, CREATE_NO_WINDOW, CREATE_UNICODE_ENVIRONMENT,
            PROCESS_INFORMATION, STARTUPINFOW,
        },
    },
    UI::WindowsAndMessaging::{SW_HIDE, SW_SHOW},
};

// Largely ported from the C# implementation here https://github.com/murrayju/CreateProcessAsUser

fn get_session_user_token() -> Result<isize, io::Error> {
    unsafe {
        let session_id = get_active_session_id()?;
        let mut phtoken: Foundation::HANDLE = 0;
        check_err(|| WTSQueryUserToken(session_id, &mut phtoken))?;

        let mut user_token = 0;

        check_err(|| {
            DuplicateTokenEx(
                phtoken,
                0,
                ptr::null(),
                SecurityImpersonation,
                TokenPrimary,
                &mut user_token,
            )
        })?;
        check_err(|| CloseHandle(phtoken))?;
        Ok(user_token)
    }
}

fn get_active_session_id() -> Result<u32, io::Error> {
    unsafe {
        let mut p_session_info = ptr::null_mut();
        let mut count = 0u32;
        check_err(|| {
            WTSEnumerateSessionsW(
                WTS_CURRENT_SERVER_HANDLE,
                0,
                1,
                &mut p_session_info,
                &mut count,
            )
        })?;

        let sessions = slice::from_raw_parts(p_session_info, count as usize);
        let active_session = sessions.iter().find(|s| s.State == WTSActive);
        let session_id =
            active_session.map_or_else(|| WTSGetActiveConsoleSessionId(), |s| s.SessionId);

        WTSFreeMemory(p_session_info as *mut ffi::c_void);
        Ok(session_id)
    }
}

pub fn start_process_as_current_user(cmd: &str, visible: bool) -> io::Result<()> {
    unsafe {
        let token = get_session_user_token()?;
        let mut p_env = ptr::null_mut();

        check_err(|| CreateEnvironmentBlock(&mut p_env, token, 0))?;

        let startup_info = STARTUPINFOW {
            cb: 0,
            lpReserved: ptr::null_mut(),
            lpDesktop: U16CString::from_str("winsta0\\default")
                .unwrap()
                .as_mut_ptr(),
            lpTitle: ptr::null_mut(),
            dwX: 0,
            dwY: 0,
            dwXSize: 0,
            dwYSize: 0,
            dwXCountChars: 0,
            dwYCountChars: 0,
            dwFillAttribute: 0,
            dwFlags: 0,
            wShowWindow: if visible { SW_SHOW } else { SW_HIDE } as u16,
            cbReserved2: 0,
            lpReserved2: ptr::null_mut(),
            hStdInput: 0,
            hStdOutput: 0,
            hStdError: 0,
        };
        let mut process_info = PROCESS_INFORMATION {
            hProcess: 0,
            hThread: 0,
            dwProcessId: 0,
            dwThreadId: 0,
        };

        check_err(|| {
            CreateProcessAsUserW(
                token,
                ptr::null(),
                U16CString::from_str(cmd).unwrap().as_mut_ptr(),
                ptr::null(),
                ptr::null(),
                0,
                CREATE_UNICODE_ENVIRONMENT
                    | if visible {
                        CREATE_NEW_CONSOLE
                    } else {
                        CREATE_NO_WINDOW
                    },
                p_env,
                ptr::null(),
                &startup_info,
                &mut process_info,
            )
        })?;

        check_err(|| CloseHandle(token))?;
        check_err(|| DestroyEnvironmentBlock(p_env))?;
        check_err(|| CloseHandle(process_info.hThread))?;
        check_err(|| CloseHandle(process_info.hProcess))?;
        Ok(())
    }
}

fn check_err<F>(f: F) -> io::Result<()>
where
    F: FnOnce() -> Foundation::BOOL,
{
    if f() == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
