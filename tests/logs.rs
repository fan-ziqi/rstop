use rstop::logs::read_tail;
use std::{
    fs::{self, File},
    io::{Seek, Write},
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "rstop-logs-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn file(&self) -> PathBuf {
        self.0.join("output.log")
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn tails_are_bounded_and_handle_append_truncation_and_replacement() {
    let temp = Temp::new();
    let path = temp.file();
    let mut writer = File::create(&path).unwrap();
    writer.write_all(b"old\nfirst\nlast\n").unwrap();
    let offset = writer.stream_position().unwrap();
    assert_eq!(read_tail(&path, 2).unwrap(), "first\nlast");
    assert_eq!(writer.stream_position().unwrap(), offset);
    writer.write_all(b"next\n").unwrap();
    assert_eq!(read_tail(&path, 1).unwrap(), "next");
    drop(writer);
    fs::write(&path, b"truncated\n").unwrap();
    assert_eq!(read_tail(&path, 10).unwrap(), "truncated");
    fs::rename(&path, temp.0.join("rotated.log")).unwrap();
    fs::write(&path, b"replacement\n").unwrap();
    assert_eq!(read_tail(&path, 10).unwrap(), "replacement");
    fs::write(&path, vec![b'x'; 2 * 1024 * 1024]).unwrap();
    let tail = read_tail(&path, 100_000).unwrap();
    assert!(tail.starts_with("[Tail limited to 1 MiB]"));
    assert!(tail.len() < 1024 * 1024 + 100);
    assert!(tail.ends_with("xxxx"));
    assert!(read_tail(&temp.0, 2).is_err());
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use rstop::logs::Target;
    use rstop::model::{Process, Snapshot};
    use std::{
        io::Read,
        process::{Child, Command, Stdio},
    };

    struct Owned(Child);
    impl Drop for Owned {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    fn target(child: &Child) -> Target {
        let id = sysinfo::Pid::from_u32(child.id());
        let mut system = sysinfo::System::new();
        system.refresh_processes_specifics(
            sysinfo::ProcessesToUpdate::Some(&[id]),
            true,
            sysinfo::ProcessRefreshKind::nothing(),
        );
        Target::process(
            &Process {
                pid: child.id(),
                start_time: system.process(id).unwrap().start_time(),
                ..Process::default()
            },
            &Snapshot::default(),
        )
    }

    #[test]
    fn process_output_reads_both_streams_and_follows_open_deleted_files() {
        let temp = Temp::new();
        let out = temp.file();
        let err = temp.0.join("error.log");
        fs::write(&out, b"stdout-marker\n").unwrap();
        fs::write(&err, b"stderr-marker\n").unwrap();
        let child = Owned(
            Command::new("sleep")
                .arg("30")
                .stdout(File::options().append(true).open(&out).unwrap())
                .stderr(File::options().append(true).open(&err).unwrap())
                .spawn()
                .unwrap(),
        );
        let target = target(&child.0);
        let body = target.read(5).unwrap();
        assert!(body.contains("stdout-marker") && body.contains("stderr-marker"));
        let before = fs::read_to_string(format!("/proc/{}/fdinfo/1", child.0.id())).unwrap();
        fs::remove_file(&out).unwrap();
        assert!(target.read(5).unwrap().contains("stdout-marker"));
        assert_eq!(
            fs::read_to_string(format!("/proc/{}/fdinfo/1", child.0.id())).unwrap(),
            before
        );
        let Target::Process { pid, start, .. } = target else {
            unreachable!()
        };
        let changed = Target::Process {
            pid,
            start: start + 1,
            containers: vec![],
            service: None,
        };
        assert!(
            changed
                .read(5)
                .unwrap_err()
                .to_string()
                .contains("identity changed")
        );
    }

    #[test]
    fn pipes_and_terminals_are_rejected_without_consuming_output() {
        let mut child = Owned(
            Command::new("sh")
                .args(["-c", "printf 'pipe-marker'; read line"])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        );
        let fd = PathBuf::from(format!("/proc/{}/fd/1", child.0.id()));
        assert!(read_tail(&fd, 5).is_err());
        let body = target(&child.0).read(5).unwrap();
        assert!(body.contains("not a readable log file"));
        let mut marker = [0; 11];
        child
            .0
            .stdout
            .as_mut()
            .unwrap()
            .read_exact(&mut marker)
            .unwrap();
        assert_eq!(&marker, b"pipe-marker");
        assert!(read_tail(PathBuf::from("/dev/null").as_path(), 5).is_err());
    }
}
