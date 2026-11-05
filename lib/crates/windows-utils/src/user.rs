use crate::error::{ComputerNameKind, WindowsUtilsError};
use std::ptr;
use windows::{
    Win32::{
        Foundation::{HLOCAL, LocalFree, NTSTATUS},
        Security::{
            Authentication::Identity::{
                GetUserNameExW, LSA_HANDLE, LSA_OBJECT_ATTRIBUTES, LsaClose, LsaFreeMemory,
                LsaNtStatusToWinError, LsaOpenPolicy, LsaQueryInformationPolicy, NameUserPrincipal,
                POLICY_ACCOUNT_DOMAIN_INFO, POLICY_VIEW_LOCAL_INFORMATION,
                PolicyAccountDomainInformation,
            },
            Authorization::ConvertSidToStringSidA,
        },
        System::SystemInformation::{
            COMPUTER_NAME_FORMAT, ComputerNameDnsDomain, ComputerNameDnsHostname,
            GetComputerNameExW,
        },
    },
    core::{HRESULT, PSTR, PWSTR},
};

#[derive(Debug)]
pub struct DomainUserInfo {
    pub domain: String,
    pub userid: String,
    pub host_machine: String,
    pub sid: String,
}

pub fn get_user_info() -> Result<DomainUserInfo, WindowsUtilsError> {
    let domain = get_domain_info()?;
    let userid_domain = get_user_id_info()?;
    let host_machine = get_host_machine_info()?;
    let sid = get_user_sid()?;

    let userid = userid_domain
        .split("@")
        .next()
        .unwrap_or_default()
        .to_owned();

    Ok(DomainUserInfo {
        domain,
        userid,
        host_machine,
        sid,
    })
}

fn get_user_id_info() -> Result<String, WindowsUtilsError> {
    let userid = NameUserPrincipal;
    let mut size: u32 = 0;
    let _ = unsafe { GetUserNameExW(userid, Some(PWSTR::null()), &mut size) };

    if size == 0 {
        return Ok(String::new());
    }

    let mut buffer = vec![0u16; size as usize];
    let result = unsafe { GetUserNameExW(userid, Some(PWSTR(buffer.as_mut_ptr())), &mut size) };

    if result {
        let name = String::from_utf16_lossy(&buffer[..size as usize]);
        Ok(name)
    } else {
        Err(WindowsUtilsError::UserName(
            windows::core::Error::from_thread(),
        ))
    }
}

fn get_domain_info() -> Result<String, WindowsUtilsError> {
    parse_computer_name(ComputerNameDnsDomain, ComputerNameKind::Domain)
}

fn get_host_machine_info() -> Result<String, WindowsUtilsError> {
    parse_computer_name(ComputerNameDnsHostname, ComputerNameKind::Hostname)
}

fn get_user_sid() -> Result<String, WindowsUtilsError> {
    unsafe {
        let mut policy_handle = LSA_HANDLE::default();
        let object_attributes = LSA_OBJECT_ATTRIBUTES::default();

        let status = LsaOpenPolicy(
            None,
            &object_attributes,
            POLICY_VIEW_LOCAL_INFORMATION as u32,
            &mut policy_handle,
        );

        if status.is_err() {
            return Err(WindowsUtilsError::LsaOpenPolicy(lsa_error(status)));
        }

        let mut buffer: *mut std::ffi::c_void = ptr::null_mut();

        let status =
            LsaQueryInformationPolicy(policy_handle, PolicyAccountDomainInformation, &mut buffer);

        let _ = LsaClose(policy_handle);

        if status.is_err() {
            return Err(WindowsUtilsError::LsaQueryPolicy(lsa_error(status)));
        }

        if buffer.is_null() {
            return Err(WindowsUtilsError::MissingDomainSid);
        }

        // The SID is owned by `buffer`, so it must be read and copied before
        // the buffer is released below.
        let sid = sid_to_string(&*(buffer as *const POLICY_ACCOUNT_DOMAIN_INFO));

        let _ = LsaFreeMemory(Some(buffer));

        sid
    }
}

/// Converts the domain SID of `info` into its string form (`S-1-5-...`).
///
/// # Safety
///
/// `info.DomainSid` must be a valid SID pointer for the duration of the call.
unsafe fn sid_to_string(info: &POLICY_ACCOUNT_DOMAIN_INFO) -> Result<String, WindowsUtilsError> {
    if info.DomainSid.is_invalid() {
        return Err(WindowsUtilsError::MissingDomainSid);
    }

    let mut sid_string = PSTR::null();
    unsafe { ConvertSidToStringSidA(info.DomainSid, &mut sid_string) }
        .map_err(WindowsUtilsError::ConvertSid)?;

    // `ConvertSidToStringSidA` allocates with `LocalAlloc`; free it on every path.
    let sid = unsafe { sid_string.to_string() }.map_err(WindowsUtilsError::SidNotUtf8);
    let _ = unsafe { LocalFree(Some(HLOCAL(sid_string.as_ptr().cast()))) };

    sid
}

/// Maps an `NTSTATUS` returned by an LSA call to the matching Win32 error.
fn lsa_error(status: NTSTATUS) -> windows::core::Error {
    let win32 = unsafe { LsaNtStatusToWinError(status) };
    windows::core::Error::from_hresult(HRESULT::from_win32(win32))
}

fn parse_computer_name(
    format: COMPUTER_NAME_FORMAT,
    kind: ComputerNameKind,
) -> Result<String, WindowsUtilsError> {
    let mut size: u32 = 0;
    let _ = unsafe { GetComputerNameExW(format, Some(PWSTR::null()), &mut size) };

    if size == 0 {
        return Ok(String::new());
    }

    let mut buffer = vec![0u16; size as usize];
    match unsafe { GetComputerNameExW(format, Some(PWSTR(buffer.as_mut_ptr())), &mut size) } {
        Ok(()) => {
            let name = String::from_utf16_lossy(&buffer[..size as usize]);
            Ok(name)
        }
        Err(source) => Err(WindowsUtilsError::ComputerName { kind, source }),
    }
}

#[cfg(test)]
mod tests {

    #[cfg(target_os = "windows")]
    mod windows_tests {
        use super::super::*;

        #[test]
        fn test_get_user_info() -> Result<(), Box<dyn std::error::Error>> {
            let info = get_user_info().unwrap();
            assert_ne!(info.domain, "");
            assert_ne!(info.userid, "");
            assert_ne!(info.host_machine, "");
            assert!(
                info.sid.starts_with("S-1-"),
                "unexpected SID form: {}",
                info.sid
            );
            Ok(())
        }
    }
}
