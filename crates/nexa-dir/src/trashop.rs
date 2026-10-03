//! 휴지통 삭제의 되돌리기(T-51 B-2c · dir2 docs/port/22 OPS "일반 삭제(휴지통)" undo · SHELL-049): `Trash` 포트의 `trash`/`restore`를
//! `ReversibleOp`로 감싼다 — undo = 원래 경로로 복원(Windows 셸 `undelete` · Linux `.trashinfo`) · redo = 다시 휴지통으로.
//! 일부만 복원되면 `OpError::Failed(n)`(히스토리가 집계 안내).

use crate::platform::Trash;
use ndir_ops::history::{OpError, ReversibleOp};
use std::path::PathBuf;
use std::rc::Rc;

pub(crate) struct TrashOp {
    paths: Vec<PathBuf>,
    description: String,
    trash: Rc<dyn Trash>,
}

impl std::fmt::Debug for TrashOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrashOp")
            .field("paths", &self.paths)
            .field("description", &self.description)
            .finish_non_exhaustive()
    }
}

impl TrashOp {
    pub(crate) fn new(paths: Vec<PathBuf>, description: String, trash: Rc<dyn Trash>) -> Self {
        TrashOp {
            paths,
            description,
            trash,
        }
    }
}

impl ReversibleOp for TrashOp {
    fn description(&self) -> &str {
        &self.description
    }

    fn undo(&mut self) -> Result<(), OpError> {
        let missing: Vec<PathBuf> = self.paths.iter().filter(|p| !p.exists()).cloned().collect();
        if missing.is_empty() {
            return Ok(());
        }
        match self.trash.restore(&missing) {
            Ok(n) if n >= missing.len() => Ok(()),
            Ok(n) => Err(OpError::Failed(missing.len() - n)),
            Err(_) => Err(OpError::Failed(missing.len())),
        }
    }

    fn redo(&mut self) -> Result<(), OpError> {
        let present: Vec<PathBuf> = self.paths.iter().filter(|p| p.exists()).cloned().collect();
        if present.is_empty() {
            return Ok(());
        }
        match self.trash.trash(&present) {
            Ok(n) if n >= present.len() => Ok(()),
            Ok(n) => Err(OpError::Failed(present.len() - n)),
            Err(_) => Err(OpError::Failed(present.len())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::PlatformError;
    use std::cell::RefCell;

    /// 메모리 휴지통(시험): 삭제 = 이름 보관 · 복원 = 다시 만든다(내용 "x").
    struct MemTrash(RefCell<Vec<PathBuf>>, bool);
    impl Trash for MemTrash {
        fn trash(&self, paths: &[PathBuf]) -> Result<usize, PlatformError> {
            for p in paths {
                let _ = std::fs::remove_file(p);
                self.0.borrow_mut().push(p.clone());
            }
            Ok(paths.len())
        }
        fn restore(&self, paths: &[PathBuf]) -> Result<usize, PlatformError> {
            if !self.1 {
                return Err(PlatformError::Unsupported("trash.restore"));
            }
            let mut n = 0;
            for p in paths {
                if self.0.borrow().contains(p) {
                    std::fs::write(p, b"x").map_err(|e| PlatformError::Failed(e.to_string()))?;
                    self.0.borrow_mut().retain(|q| q != p);
                    n += 1;
                }
            }
            Ok(n)
        }
    }

    #[test]
    fn undo_restores_missing_and_redo_trashes_present() {
        let dir = std::env::temp_dir().join(format!("ndir-trashop-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let a = dir.join("a.txt");
        std::fs::write(&a, b"x").unwrap();
        let t: Rc<dyn Trash> = Rc::new(MemTrash(RefCell::new(Vec::new()), true));
        assert_eq!(t.trash(std::slice::from_ref(&a)).unwrap(), 1);
        let mut op = TrashOp::new(vec![a.clone()], "recycle 1 item(s)".into(), Rc::clone(&t));
        assert_eq!(op.description(), "recycle 1 item(s)");
        assert!(!a.exists());
        assert_eq!(op.undo(), Ok(()));
        assert!(a.exists(), "복원");
        assert_eq!(op.undo(), Ok(()), "이미 있으면 할 일 없음");
        assert_eq!(op.redo(), Ok(()));
        assert!(!a.exists(), "다시 휴지통");
        // 복원 미지원 포트 = 실패 집계.
        let u: Rc<dyn Trash> = Rc::new(MemTrash(RefCell::new(vec![a.clone()]), false));
        let mut op2 = TrashOp::new(vec![a], "x".into(), u);
        assert_eq!(op2.undo(), Err(OpError::Failed(1)));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
