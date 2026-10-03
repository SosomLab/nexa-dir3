//! 가짜 플랫폼(시험 · CI-103): 모든 포트 호출을 [`FakeLog`]에 기록하고, 주입한 값을 돌려준다. OS·프로세스·클립보드를 건드리지 않는다.

// 가짜는 시험이 쓰고 운영 빌드에서는 미사용 — 모듈 단위로 허용.
#![allow(dead_code)]

use super::*;

#[derive(Debug, Default)]
pub(crate) struct FakeLog {
    /// 호출 기록(`"open:<path>"` · `"trash:<n>"` · `"clip.write:<n>:cut"` …).
    pub calls: Vec<String>,
    /// 주입: 셸 후보 · 드라이브 용량 · 클립보드 내용 · 감시 변경 큐 · 열기 실패 여부.
    pub shells: Vec<ShellSpec>,
    pub space: Option<(u64, u64)>,
    pub clipboard: Option<(Vec<PathBuf>, bool)>,
    pub changed: Vec<PathBuf>,
    pub open_fails: bool,
    pub watched: Vec<PathBuf>,
}

type Log = Rc<RefCell<FakeLog>>;

struct FakeShell(Log);
struct FakePty(Log);
struct FakeMenu(Log);
struct FakeTrash(Log);
struct FakeClip(Log);
struct FakeDrag(Log);
struct FakeWatch(Log);
struct FakeOpen(Log);
struct FakeDisk(Log);
struct FakeTemplates(Log);

fn note(log: &Log, s: String) {
    log.borrow_mut().calls.push(s);
}

impl Shell for FakeShell {
    fn candidates(&self) -> Vec<ShellSpec> {
        let l = self.0.borrow();
        if l.shells.is_empty() {
            vec![ShellSpec {
                program: PathBuf::from("fake-sh"),
                args: vec![],
                label: "fake".into(),
            }]
        } else {
            l.shells.clone()
        }
    }
}

/// 되돌림(echo) 세션 — 쓴 바이트를 그대로 읽힌다.
struct EchoSession {
    buf: Vec<u8>,
    killed: bool,
}

impl PtySession for EchoSession {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        self.buf.extend_from_slice(bytes);
        Ok(())
    }
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        let n = self.buf.len().min(out.len());
        out[..n].copy_from_slice(&self.buf[..n]);
        self.buf.drain(..n);
        Ok(n)
    }
    fn resize(&mut self, _cols: u16, _rows: u16) -> std::io::Result<()> {
        Ok(())
    }
    fn kill(&mut self) {
        self.killed = true;
    }
}

impl Pty for FakePty {
    fn spawn(
        &self,
        shell: &ShellSpec,
        cwd: &Path,
        cols: u16,
        rows: u16,
    ) -> Result<Box<dyn PtySession>, PlatformError> {
        note(
            &self.0,
            format!("pty:{}:{}:{cols}x{rows}", shell.label, cwd.display()),
        );
        Ok(Box::new(EchoSession {
            buf: Vec::new(),
            killed: false,
        }))
    }
}

impl ContextMenuProvider for FakeMenu {
    fn items(&self, paths: &[PathBuf]) -> Result<Vec<ShellMenuItem>, PlatformError> {
        note(&self.0, format!("menu.items:{}", paths.len()));
        Ok(vec![ShellMenuItem {
            id: "fake.open".into(),
            label: "Fake Open".into(),
            enabled: true,
            ..Default::default()
        }])
    }
    fn invoke(&self, id: &str, paths: &[PathBuf]) -> Result<(), PlatformError> {
        note(&self.0, format!("menu.invoke:{id}:{}", paths.len()));
        Ok(())
    }
    fn bg_items(&self, dir: &Path) -> Result<Vec<ShellMenuItem>, PlatformError> {
        note(&self.0, format!("menu.bg_items:{}", dir.display()));
        Ok(vec![ShellMenuItem {
            id: "fake.bgopen".into(),
            label: "Fake Background".into(),
            enabled: true,
            ..Default::default()
        }])
    }
    fn invoke_bg(&self, id: &str, dir: &Path) -> Result<Option<PathBuf>, PlatformError> {
        note(&self.0, format!("menu.invoke_bg:{id}"));
        // `fake.bgnew` = 새로 만들기 흉내(파일 하나 생성 → 생성 경로 보고).
        if id == "fake.bgnew" {
            let p = dir.join("New Fake.txt");
            std::fs::write(&p, b"").map_err(|e| PlatformError::Failed(e.to_string()))?;
            return Ok(Some(p));
        }
        Ok(None)
    }
}

impl Trash for FakeTrash {
    fn trash(&self, paths: &[PathBuf]) -> Result<usize, PlatformError> {
        note(&self.0, format!("trash:{}", paths.len()));
        Ok(paths.len())
    }
    fn restore(&self, original: &[PathBuf]) -> Result<usize, PlatformError> {
        note(&self.0, format!("trash.restore:{}", original.len()));
        Ok(original.len())
    }
}

impl FileClipboard for FakeClip {
    fn read_files(&self) -> Option<(Vec<PathBuf>, bool)> {
        note(&self.0, "clip.read".into());
        self.0.borrow().clipboard.clone()
    }
    fn write_files(&self, paths: &[PathBuf], cut: bool) -> Result<(), PlatformError> {
        note(
            &self.0,
            format!(
                "clip.write:{}:{}",
                paths.len(),
                if cut { "cut" } else { "copy" }
            ),
        );
        self.0.borrow_mut().clipboard = Some((paths.to_vec(), cut));
        Ok(())
    }
}

impl DragSource for FakeDrag {
    fn begin_drag(&self, paths: &[PathBuf]) -> Result<DragOutcome, PlatformError> {
        note(&self.0, format!("drag:{}", paths.len()));
        Ok(DragOutcome::Cancelled)
    }
}

impl Watcher for FakeWatch {
    fn watch(&mut self, dirs: &[PathBuf]) {
        self.0.borrow_mut().watched = dirs.to_vec();
    }
    fn poll(&mut self) -> Vec<PathBuf> {
        std::mem::take(&mut self.0.borrow_mut().changed)
    }
}

impl Opener for FakeOpen {
    fn open(&self, path: &Path) -> Result<(), PlatformError> {
        note(&self.0, format!("open:{}", path.display()));
        if self.0.borrow().open_fails {
            Err(PlatformError::Failed("fake open failed".into()))
        } else {
            Ok(())
        }
    }
    fn reveal(&self, path: &Path) -> Result<(), PlatformError> {
        note(&self.0, format!("reveal:{}", path.display()));
        Ok(())
    }
}

impl Disk for FakeDisk {
    fn space(&self, root: &Path) -> Option<(u64, u64)> {
        note(&self.0, format!("disk:{}", root.display()));
        self.0.borrow().space
    }
}

impl Templates for FakeTemplates {
    fn list(&self) -> Vec<NewTemplate> {
        note(&self.0, "templates.list".into());
        vec![NewTemplate {
            label: "Fake Doc".into(),
            ext: "fdoc".into(),
            source: TemplateSource::Data(b"fake-template".to_vec()),
        }]
    }
}

pub(crate) fn platform() -> Platform {
    let log: Log = Rc::new(RefCell::new(FakeLog::default()));
    Platform {
        shell: Box::new(FakeShell(log.clone())),
        pty: Box::new(FakePty(log.clone())),
        ctxmenu: Box::new(FakeMenu(log.clone())),
        trash: Rc::new(FakeTrash(log.clone())),
        clipboard: Box::new(FakeClip(log.clone())),
        drag: Box::new(FakeDrag(log.clone())),
        watcher: Box::new(FakeWatch(log.clone())),
        opener: Box::new(FakeOpen(log.clone())),
        disk: Box::new(FakeDisk(log.clone())),
        templates: Box::new(FakeTemplates(log.clone())),
        log: Some(log),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_records_and_injects() {
        let mut p = Platform::fake();
        let log = p.log.clone().expect("fake log");
        log.borrow_mut().space = Some((100, 40));
        log.borrow_mut().changed = vec![PathBuf::from("/x")];
        assert_eq!(p.disk.space(Path::new("/")), Some((100, 40)));
        assert_eq!(
            p.trash.trash(&[PathBuf::from("a"), PathBuf::from("b")]),
            Ok(2)
        );
        p.clipboard
            .write_files(&[PathBuf::from("c")], true)
            .unwrap();
        assert_eq!(
            p.clipboard.read_files(),
            Some((vec![PathBuf::from("c")], true))
        );
        assert_eq!(p.watcher.poll(), vec![PathBuf::from("/x")]);
        assert!(p.watcher.poll().is_empty());
        p.opener.open(Path::new("f.txt")).unwrap();
        log.borrow_mut().open_fails = true;
        assert!(p.opener.open(Path::new("g.txt")).is_err());
        let mut s = p
            .pty
            .spawn(&p.shell.default_shell().unwrap(), Path::new("."), 80, 24)
            .unwrap();
        s.write(b"hi").unwrap();
        let mut buf = [0u8; 8];
        assert_eq!(s.read(&mut buf).unwrap(), 2);
        assert_eq!(&buf[..2], b"hi");
        let calls = log.borrow().calls.clone();
        assert_eq!(calls[0], "disk:/");
        assert!(calls.iter().any(|c| c == "trash:2"));
        assert!(calls.iter().any(|c| c == "clip.write:1:cut"));
        assert!(calls.iter().any(|c| c.starts_with("open:f.txt")));
        assert!(calls.iter().any(|c| c.starts_with("pty:fake:")));
    }
}
