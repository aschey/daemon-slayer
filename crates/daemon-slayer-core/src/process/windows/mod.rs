use core::ffi;
use std::{io, ptr, slice};

use widestring::U16CString;
use windows_sys::Win32::Foundation::{
    self, CloseHandle, SetHandleInformation, HANDLE, HANDLE_FLAG_INHERIT, WAIT_FAILED,
};
use windows_sys::Win32::Security::{
    DuplicateTokenEx, SecurityImpersonation, TokenPrimary, SECURITY_ATTRIBUTES,
};
use windows_sys::Win32::Storage::FileSystem::ReadFile;
use windows_sys::Win32::System::Console::{GetStdHandle, STD_INPUT_HANDLE};
use windows_sys::Win32::System::Environment::{CreateEnvironmentBlock, DestroyEnvironmentBlock};
use windows_sys::Win32::System::Pipes::CreatePipe;
use windows_sys::Win32::System::RemoteDesktop::{
    WTSActive, WTSEnumerateSessionsW, WTSFreeMemory, WTSGetActiveConsoleSessionId,
    WTSQueryUserToken, WTS_CURRENT_SERVER_HANDLE,
};
use windows_sys::Win32::System::Threading::{
    CreateProcessAsUserW, GetExitCodeProcess, WaitForSingleObject, CREATE_NEW_CONSOLE,
    CREATE_NO_WINDOW, CREATE_UNICODE_ENVIRONMENT, INFINITE, PROCESS_INFORMATION,
    STARTF_USESTDHANDLES, STARTUPINFOW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{SW_HIDE, SW_SHOW};

use crate::Label;

// Largely ported from the C# implementation here https://github.com/murrayju/CreateProcessAsUser
// and the examples here
// https://stackoverflow.com/questions/35969730/how-to-read-output-from-cmd-exe-using-createprocess-and-createpipe

fn get_session_user_token() -> io::Result<Foundation::HANDLE> {
    unsafe {
        let session_id = get_active_session_id()?;
        let mut phtoken: Foundation::HANDLE = ptr::null_mut();
        check_err(|| WTSQueryUserToken(session_id, &mut phtoken))?;

        let mut user_token: Foundation::HANDLE = ptr::null_mut();

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

fn get_active_session_id() -> io::Result<u32> {
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

pub async fn run_process_as_current_user(
    _label: &Label,
    cmd: &str,
    visible: bool,
) -> io::Result<String> {
    let cmd = cmd.to_owned();
    tokio::task::spawn_blocking(move || run_process_as_current_user_blocking(&cmd, visible))
        .await
        .unwrap()
}

fn run_process_as_current_user_blocking(cmd: &str, visible: bool) -> io::Result<String> {
    unsafe {
        let mut h_out_read_pipe: HANDLE = ptr::null_mut();
        let mut h_out_write_pipe: HANDLE = ptr::null_mut();
        let security_attrs = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            bInheritHandle: 1,
            lpSecurityDescriptor: ptr::null_mut(),
        };
        check_err(|| {
            CreatePipe(
                &mut h_out_read_pipe,
                &mut h_out_write_pipe,
                &security_attrs,
                0,
            )
        })?;
        check_err(|| SetHandleInformation(h_out_read_pipe, HANDLE_FLAG_INHERIT, 0))?;

        let token = get_session_user_token()?;
        let mut p_env = ptr::null_mut();

        check_err(|| CreateEnvironmentBlock(&mut p_env, token, 0))?;

        let startup_info = STARTUPINFOW {
            cb: std::mem::size_of::<STARTUPINFOW>() as u32,
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
            dwFlags: STARTF_USESTDHANDLES,
            wShowWindow: if visible { SW_SHOW } else { SW_HIDE } as u16,
            cbReserved2: 0,
            lpReserved2: ptr::null_mut(),
            hStdInput: GetStdHandle(STD_INPUT_HANDLE),
            hStdOutput: h_out_write_pipe,
            hStdError: h_out_write_pipe,
        };
        let mut process_info = PROCESS_INFORMATION {
            hProcess: ptr::null_mut(),
            hThread: ptr::null_mut(),
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
                1,
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

        if WaitForSingleObject(process_info.hProcess, INFINITE) == WAIT_FAILED {
            return Err(io::Error::last_os_error());
        }

        let mut exit_code = 0;
        check_err(|| GetExitCodeProcess(process_info.hProcess, &mut exit_code))?;

        check_err(|| CloseHandle(h_out_write_pipe))?;
        let mut output = String::new();
        loop {
            const BUF_SIZE: usize = 4096;
            let mut ch_buf = Vec::with_capacity(BUF_SIZE);
            let mut bytes_read = 0;

            if let Err(e) = check_err(|| {
                ReadFile(
                    h_out_read_pipe,
                    ch_buf.as_mut_ptr(),
                    BUF_SIZE as u32,
                    &mut bytes_read,
                    ptr::null_mut(),
                )
            }) {
                if e.kind() != io::ErrorKind::BrokenPipe {
                    return Err(e);
                }
            }

            if bytes_read > 0 {
                ch_buf.set_len(bytes_read as usize);
                output += String::from_utf8(ch_buf).unwrap().as_str();
            } else {
                break;
            }
        }

        check_err(|| CloseHandle(h_out_read_pipe))?;
        check_err(|| CloseHandle(token))?;
        check_err(|| DestroyEnvironmentBlock(p_env))?;
        check_err(|| CloseHandle(process_info.hThread))?;
        check_err(|| CloseHandle(process_info.hProcess))?;

        Ok(output)
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
