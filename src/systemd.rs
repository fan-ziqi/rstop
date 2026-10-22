//! Read-only sd-bus queries. Loading at runtime keeps non-systemd Linux portable.
use crate::model::Service;
use anyhow::{Result, ensure};
use libloading::Library;
use std::{
    collections::HashSet,
    ffi::{CStr, CString, c_char, c_int, c_void},
    ptr,
    time::{Duration, Instant},
};

type Pointer = *mut c_void;
type New = unsafe extern "C" fn(*mut Pointer) -> c_int;
type Unref = unsafe extern "C" fn(Pointer) -> Pointer;
type SetAddress = unsafe extern "C" fn(Pointer, *const c_char) -> c_int;
type SetBool = unsafe extern "C" fn(Pointer, c_int) -> c_int;
type Start = unsafe extern "C" fn(Pointer) -> c_int;
type Timeout = unsafe extern "C" fn(Pointer, u64) -> c_int;
type Call = unsafe extern "C" fn(
    Pointer,
    *const c_char,
    *const c_char,
    *const c_char,
    *const c_char,
    Pointer,
    *mut Pointer,
    *const c_char,
    ...
) -> c_int;
type Read = unsafe extern "C" fn(Pointer, *const c_char, ...) -> c_int;
type Enter = unsafe extern "C" fn(Pointer, c_char, *const c_char) -> c_int;
type Get = unsafe extern "C" fn(
    Pointer,
    *const c_char,
    *const c_char,
    *const c_char,
    *const c_char,
    Pointer,
    c_char,
    Pointer,
) -> c_int;
type GetString = unsafe extern "C" fn(
    Pointer,
    *const c_char,
    *const c_char,
    *const c_char,
    *const c_char,
    Pointer,
    *mut *mut c_char,
) -> c_int;

struct Api {
    new: New,
    unref: Unref,
    message_unref: Unref,
    address: SetAddress,
    client: SetBool,
    start: Start,
    timeout: Timeout,
    call: Call,
    read: Read,
    enter: Enter,
    get: Get,
    get_string: GetString,
    _library: Library,
}
impl Api {
    fn load() -> Result<Self> {
        // All copied symbols remain valid until the owning Library is dropped.
        unsafe {
            let library = Library::new("libsystemd.so.0")?;
            Ok(Self {
                new: *library.get(b"sd_bus_new\0")?,
                unref: *library.get(b"sd_bus_unref\0")?,
                message_unref: *library.get(b"sd_bus_message_unref\0")?,
                address: *library.get(b"sd_bus_set_address\0")?,
                client: *library.get(b"sd_bus_set_bus_client\0")?,
                start: *library.get(b"sd_bus_start\0")?,
                timeout: *library.get(b"sd_bus_set_method_call_timeout\0")?,
                call: *library.get(b"sd_bus_call_method\0")?,
                read: *library.get(b"sd_bus_message_read\0")?,
                enter: *library.get(b"sd_bus_message_enter_container\0")?,
                get: *library.get(b"sd_bus_get_property_trivial\0")?,
                get_string: *library.get(b"sd_bus_get_property_string\0")?,
                _library: library,
            })
        }
    }
}
struct Bus<'a> {
    raw: Pointer,
    api: &'a Api,
}
impl Drop for Bus<'_> {
    fn drop(&mut self) {
        unsafe {
            (self.api.unref)(self.raw);
        }
    }
}
struct Message<'a> {
    raw: Pointer,
    api: &'a Api,
}
impl Drop for Message<'_> {
    fn drop(&mut self) {
        unsafe {
            (self.api.message_unref)(self.raw);
        }
    }
}
fn check(value: c_int) -> Result<c_int> {
    ensure!(value >= 0, "{}", std::io::Error::from_raw_os_error(-value));
    Ok(value)
}
fn text(raw: *const c_char) -> String {
    if raw.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(raw) }
            .to_string_lossy()
            .into_owned()
    }
}

impl<'a> Bus<'a> {
    fn connect(api: &'a Api, scope: &str) -> Result<Self> {
        let address = if scope == "system" {
            "unix:path=/run/dbus/system_bus_socket".into()
        } else {
            std::env::var("DBUS_SESSION_BUS_ADDRESS").unwrap_or_else(|_| {
                format!("unix:path=/run/user/{}/bus", unsafe { libc::geteuid() })
            })
        };
        ensure!(
            address.split(';').all(|a| a.starts_with("unix:")),
            "Nonlocal service bus skipped"
        );
        let address = CString::new(address)?;
        let mut raw = ptr::null_mut();
        unsafe {
            check((api.new)(&mut raw))?;
        }
        let bus = Self { raw, api };
        unsafe {
            check((api.address)(raw, address.as_ptr()))?;
            check((api.client)(raw, 1))?;
            check((api.timeout)(raw, 250_000))?;
            check((api.start)(raw))?;
        }
        Ok(bus)
    }
    fn list(&self, method: &CStr) -> Result<Message<'a>> {
        let mut raw = ptr::null_mut();
        let result = unsafe {
            (self.api.call)(
                self.raw,
                c"org.freedesktop.systemd1".as_ptr(),
                c"/org/freedesktop/systemd1".as_ptr(),
                c"org.freedesktop.systemd1.Manager".as_ptr(),
                method.as_ptr(),
                ptr::null_mut(),
                &mut raw,
                c"".as_ptr(),
            )
        };
        let reply = Message { raw, api: self.api };
        check(result)?;
        Ok(reply)
    }
    fn number(&self, path: &CStr, key: &CStr, wide: bool) -> Option<u64> {
        let mut value = 0u64;
        let mut small = 0u32;
        let result = unsafe {
            (self.api.get)(
                self.raw,
                c"org.freedesktop.systemd1".as_ptr(),
                path.as_ptr(),
                c"org.freedesktop.systemd1.Service".as_ptr(),
                key.as_ptr(),
                ptr::null_mut(),
                if wide { b't' } else { b'u' } as c_char,
                if wide {
                    &mut value as *mut _ as Pointer
                } else {
                    &mut small as *mut _ as Pointer
                },
            )
        };
        (result >= 0).then_some(if wide { value } else { small as u64 })
    }
    fn string(&self, path: &CStr, interface: &CStr, key: &CStr) -> Option<String> {
        let mut value = ptr::null_mut();
        let result = unsafe {
            (self.api.get_string)(
                self.raw,
                c"org.freedesktop.systemd1".as_ptr(),
                path.as_ptr(),
                interface.as_ptr(),
                key.as_ptr(),
                ptr::null_mut(),
                &mut value,
            )
        };
        let copied = (result >= 0).then(|| text(value));
        unsafe {
            libc::free(value as Pointer);
        }
        copied
    }
}

pub(crate) fn services(scope: &str) -> Result<Vec<Service>> {
    let api = Api::load()?;
    let bus = Bus::connect(&api, scope)?;
    let began = Instant::now();
    let reply = bus.list(c"ListUnits")?;
    unsafe {
        check((api.enter)(
            reply.raw,
            b'a' as c_char,
            c"(ssssssouso)".as_ptr(),
        ))?;
    }
    let mut rows = Vec::new();
    loop {
        let mut values = [ptr::null::<c_char>(); 9];
        let mut job = 0u32;
        let target = values.as_mut_ptr();
        let found = unsafe {
            (api.read)(
                reply.raw,
                c"(ssssssouso)".as_ptr(),
                target,
                target.add(1),
                target.add(2),
                target.add(3),
                target.add(4),
                target.add(5),
                target.add(6),
                &mut job as *mut u32,
                target.add(7),
                target.add(8),
            )
        };
        if check(found)? == 0 {
            break;
        }
        let name = text(values[0]);
        if !name.ends_with(".service") {
            continue;
        }
        ensure!(
            began.elapsed() < Duration::from_secs(3),
            "Service query deadline"
        );
        let path = unsafe { CStr::from_ptr(values[6]) };
        let mut service = Service {
            name,
            scope: scope.into(),
            description: text(values[1]),
            load: text(values[2]),
            active: text(values[3]),
            sub: text(values[4]),
            ..Default::default()
        };
        for (key, wide) in [
            (c"MainPID", false),
            (c"MemoryCurrent", true),
            (c"CPUUsageNSec", true),
            (c"TasksCurrent", true),
            (c"NRestarts", false),
        ] {
            if let Some(value) = bus.number(path, key, wide) {
                service.extra.insert(
                    key.to_string_lossy().into_owned(),
                    serde_json::json!(value.to_string()),
                );
                match key.to_bytes() {
                    b"MainPID" => service.pid = (value > 0).then_some(value as u32),
                    b"MemoryCurrent" => service.memory = (value < u64::MAX).then_some(value),
                    b"CPUUsageNSec" => service.cpu_time_ns = (value < u64::MAX).then_some(value),
                    b"TasksCurrent" => service.tasks = (value < u64::MAX).then_some(value),
                    _ => {}
                }
            }
        }
        for (interface, key) in [
            (c"org.freedesktop.systemd1.Service", c"ControlGroup"),
            (c"org.freedesktop.systemd1.Unit", c"FragmentPath"),
        ] {
            if let Some(value) = bus.string(path, interface, key) {
                service
                    .extra
                    .insert(key.to_string_lossy().into_owned(), serde_json::json!(value));
            }
        }
        service
            .extra
            .insert("Id".into(), serde_json::json!(service.name));
        rows.push(service);
    }
    let names: HashSet<_> = rows.iter().map(|s| s.name.clone()).collect();
    let reply = bus.list(c"ListUnitFiles")?;
    unsafe {
        check((api.enter)(reply.raw, b'a' as c_char, c"(ss)".as_ptr()))?;
    }
    loop {
        let mut path = ptr::null::<c_char>();
        let mut state = ptr::null::<c_char>();
        let found = unsafe {
            (api.read)(
                reply.raw,
                c"(ss)".as_ptr(),
                &mut path as *mut *const c_char,
                &mut state as *mut *const c_char,
            )
        };
        if check(found)? == 0 {
            break;
        }
        let path = text(path);
        let name = path.rsplit('/').next().unwrap_or(&path);
        if name.ends_with(".service") && !names.contains(name) {
            rows.push(Service {
                name: name.into(),
                scope: scope.into(),
                load: "installed".into(),
                active: "not-loaded".into(),
                sub: text(state),
                description: "Unit file; not currently loaded".into(),
                ..Default::default()
            });
        }
    }
    ensure!(
        began.elapsed() < Duration::from_secs(3),
        "Service query deadline"
    );
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "Requires a responsive local systemd manager; live service acceptance"]
    fn local_service_sample_keeps_loaded_and_installed_units_and_known_properties() {
        if !std::path::Path::new("/run/systemd/system").exists() {
            return;
        }
        let rows = services("system").unwrap();
        assert!(rows.iter().any(|s| s.load == "installed"));
        assert!(rows.iter().any(|s| s.active == "active"));
        let service = rows
            .iter()
            .find(|s| s.pid.is_some())
            .expect("No active MainPID");
        assert!(service.extra.contains_key("ControlGroup"));
        assert!(service.extra.contains_key("FragmentPath"));
    }
}
