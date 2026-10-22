//! Read-only Windows process environment and adapter discovery.
#[cfg(windows)]
pub fn environment(pid: u32) -> anyhow::Result<String> {
    use anyhow::ensure;
    use windows_sys::Win32::{
        Foundation::CloseHandle,
        System::{Diagnostics::Debug::ReadProcessMemory, Threading::*},
    };
    #[repr(C)]
    struct Basic {
        reserved1: *mut core::ffi::c_void,
        peb: *mut core::ffi::c_void,
        reserved2: [*mut core::ffi::c_void; 2],
        pid: usize,
        reserved3: *mut core::ffi::c_void,
    }
    #[link(name = "ntdll")]
    unsafe extern "system" {
        fn NtQueryInformationProcess(
            handle: *mut core::ffi::c_void,
            class: u32,
            info: *mut core::ffi::c_void,
            size: u32,
            returned: *mut u32,
        ) -> i32;
    }
    struct Handle(*mut core::ffi::c_void);
    impl Drop for Handle {
        fn drop(&mut self) {
            unsafe { CloseHandle(self.0) };
        }
    }
    let handle =
        Handle(unsafe { OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, 0, pid) });
    ensure!(!handle.0.is_null(), "{}", std::io::Error::last_os_error());
    let mut basic = unsafe { std::mem::zeroed::<Basic>() };
    let status = unsafe {
        NtQueryInformationProcess(
            handle.0,
            0,
            (&mut basic as *mut Basic).cast(),
            std::mem::size_of::<Basic>() as u32,
            std::ptr::null_mut(),
        )
    };
    ensure!(status >= 0, "NtQueryInformationProcess: {status:#x}");
    let mut wow64 = 0usize;
    let status = unsafe {
        NtQueryInformationProcess(
            handle.0,
            26,
            (&mut wow64 as *mut usize).cast(),
            std::mem::size_of::<usize>() as u32,
            std::ptr::null_mut(),
        )
    };
    let narrow = status >= 0 && wow64 != 0;
    let peb = if narrow { wow64 } else { basic.peb as usize };
    let read_ptr = |address: usize| -> anyhow::Result<usize> {
        let mut bytes = [0u8; 8];
        let size = if narrow {
            4
        } else {
            std::mem::size_of::<usize>()
        };
        let ok = unsafe {
            ReadProcessMemory(
                handle.0,
                address as *const _,
                bytes.as_mut_ptr().cast(),
                size,
                std::ptr::null_mut(),
            )
        };
        ensure!(ok != 0, "{}", std::io::Error::last_os_error());
        Ok(u64::from_le_bytes(bytes) as usize)
    };
    let params = read_ptr(peb + if narrow { 0x10 } else { 0x20 })?;
    let environment = read_ptr(params + if narrow { 0x48 } else { 0x80 })?;
    ensure!(environment != 0, "No environment block");
    let mut units = vec![];
    for offset in (0..1024 * 1024).step_by(256) {
        let mut bytes = [0u16; 128];
        let mut count = 0usize;
        let ok = unsafe {
            ReadProcessMemory(
                handle.0,
                (environment + offset) as *const _,
                bytes.as_mut_ptr().cast(),
                256,
                &mut count,
            )
        };
        if ok == 0 && count == 0 {
            break;
        }
        units.extend_from_slice(&bytes[..count / 2]);
        if let Some(end) = units.windows(2).position(|w| w == [0, 0]) {
            units.truncate(end);
            return Ok(String::from_utf16_lossy(&units).replace('\0', "\n"));
        }
    }
    anyhow::bail!(
        "Environment block could not be read completely (protected process or unsupported Windows ABI)"
    )
}
#[cfg(windows)]
pub fn adapters() -> serde_json::Value {
    use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIFactory1};
    let mut values = vec![];
    unsafe {
        if let Ok(factory) = CreateDXGIFactory1::<IDXGIFactory1>() {
            let mut index = 0;
            while let Ok(adapter) = factory.EnumAdapters1(index) {
                if let Ok(desc) = adapter.GetDesc1() {
                    values.push(serde_json::json!({"name":String::from_utf16_lossy(&desc.Description).trim_end_matches('\0'),"vendor_id":desc.VendorId,"dedicated_bytes":desc.DedicatedVideoMemory,"shared_bytes":desc.SharedSystemMemory,"luid":format!("luid_0x{:08x}_0x{:08x}",desc.AdapterLuid.HighPart as u32,desc.AdapterLuid.LowPart)}));
                }
                index += 1;
            }
        }
    }
    serde_json::json!(values)
}

/// Bound helper descendants to the lifetime of an external query.
#[cfg(windows)]
pub struct QueryJob(windows_sys::Win32::Foundation::HANDLE);
#[cfg(windows)]
impl QueryJob {
    pub fn attach(child: &std::process::Child) -> anyhow::Result<Self> {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::{Foundation::CloseHandle, System::JobObjects::*};
        let handle = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        anyhow::ensure!(
            !handle.is_null(),
            "Cannot create query job: {}",
            std::io::Error::last_os_error()
        );
        let mut limits = unsafe { std::mem::zeroed::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() };
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        let configured = unsafe {
            SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                std::mem::size_of_val(&limits) as u32,
            )
        };
        let assigned = configured != 0
            && unsafe { AssignProcessToJobObject(handle, child.as_raw_handle().cast()) } != 0;
        if !assigned {
            let error = std::io::Error::last_os_error();
            unsafe { CloseHandle(handle) };
            anyhow::bail!("Cannot assign query job: {error}");
        }
        Ok(Self(handle))
    }
}
#[cfg(windows)]
impl Drop for QueryJob {
    fn drop(&mut self) {
        unsafe { windows_sys::Win32::Foundation::CloseHandle(self.0) };
    }
}
