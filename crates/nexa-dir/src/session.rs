//! 세션 파일 — dir2 `data\session.cfg`(`# nexa-dir session v1`) **형식 그대로**(docs/port/13 §5-2 · port/15 PREFS-050): dir2 사용자의 세션을
//! 그대로 읽는다. dir3 저장 이름 = 설정 폴더의 `session.conf`(없으면 `session.cfg`도 읽는다 — PREFS-045 대응).
//!
//! 키: `active_panel` · `panel{i}.tabs`(`|` 연결 — Windows 경로 불가 문자) · `.active` · `.exp{j}`(탭별 펼침 · 탭당 ≤200) · `.locked`/`.pinned`
//! (`0|1` · 하나라도 참일 때만) · `.modes`(`tree|flat|tiles` · 전부 tree면 생략) · `.views`(bit0 숨김·bit1 Dot·bit2 폴더 우선) · `.cols`(레이아웃 문자열) ·
//! `.colw`(표시 열 폭 `,` 연결). dir3가 아직 안 쓰는 키(exp·locked·pinned·views·cols)는 **파싱해 보존**하고 다시 쓴다(dir2로 돌아가도 잃지 않게).
//! 영속하지 않는 것(dir2 그대로): 정렬 · 선택 · 캐럿 · 스크롤 · 히스토리.

use std::path::{Path, PathBuf};

pub(crate) const FILE_NAME: &str = "session.conf";
/// dir2 파일 이름(같은 폴더에 있으면 읽기만 — 저장은 `FILE_NAME`).
pub(crate) const LEGACY_FILE_NAME: &str = "session.cfg";
const HEADER: &str = "# nexa-dir session v1";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct PanelSession {
    pub tabs: Vec<PathBuf>,
    pub active: usize,
    /// 탭별 펼침 경로(인덱스 정렬 · 빈 탭 자리 허용).
    pub expanded: Vec<Vec<PathBuf>>,
    pub locked: Vec<bool>,
    pub pinned: Vec<bool>,
    /// 탭별 보기 모드(`tree`/`flat`/`tiles`).
    pub modes: Vec<String>,
    pub views: Vec<u8>,
    pub col_layout: String,
    pub col_widths: Vec<i32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Session {
    pub active_panel: usize,
    pub panels: [PanelSession; 2],
    /// 폴더별 보기 옵션 기억(`dirview=<플래그>|<경로>` 줄 · 보기 범위 "폴더" — bit0 숨김 · bit1 Dot · bit2 폴더 우선 ·
    /// 뒤가 최근 · 상한 [`DIR_VIEWS_MAX`]). 설정 기본값과 같은 폴더는 기록하지 않는다.
    pub dir_views: Vec<(PathBuf, u8)>,
}

/// 폴더별 보기 옵션 기억 상한(넘으면 오래된 것부터 버린다).
pub(crate) const DIR_VIEWS_MAX: usize = 300;

/// `key=value` 줄(주석 `#` · 빈 줄 무시 · 첫 `=` 기준).
fn kv_lines(text: &str) -> impl Iterator<Item = (&str, &str)> {
    text.lines().filter_map(|l| {
        let l = l.trim_end_matches('\r');
        if l.trim().is_empty() || l.trim_start().starts_with('#') {
            return None;
        }
        let (k, v) = l.split_once('=')?;
        Some((k.trim(), v.trim()))
    })
}

impl Session {
    /// 탭 경로 구분자 `|` — Windows 경로에 등장 불가 문자(dir2 `serialize` 그대로).
    pub(crate) fn serialize(&self) -> String {
        let mut out = format!("{HEADER}\n");
        out.push_str(&format!("active_panel={}\n", self.active_panel));
        for (i, p) in self.panels.iter().enumerate() {
            let tabs: Vec<String> = p
                .tabs
                .iter()
                .map(|t| t.to_string_lossy().into_owned())
                .collect();
            out.push_str(&format!("panel{i}.tabs={}\n", tabs.join("|")));
            out.push_str(&format!("panel{i}.active={}\n", p.active));
            for (j, exp) in p.expanded.iter().enumerate() {
                if !exp.is_empty() {
                    let list: Vec<String> = exp
                        .iter()
                        .take(200)
                        .map(|t| t.to_string_lossy().into_owned())
                        .collect();
                    out.push_str(&format!("panel{i}.exp{j}={}\n", list.join("|")));
                }
            }
            if p.locked.iter().any(|l| *l) {
                let flags: Vec<&str> = p
                    .locked
                    .iter()
                    .map(|l| if *l { "1" } else { "0" })
                    .collect();
                out.push_str(&format!("panel{i}.locked={}\n", flags.join("|")));
            }
            if p.pinned.iter().any(|l| *l) {
                let flags: Vec<&str> = p
                    .pinned
                    .iter()
                    .map(|l| if *l { "1" } else { "0" })
                    .collect();
                out.push_str(&format!("panel{i}.pinned={}\n", flags.join("|")));
            }
            if p.modes.iter().any(|m| m != "tree") {
                out.push_str(&format!("panel{i}.modes={}\n", p.modes.join("|")));
            }
            if !p.views.is_empty() {
                let vs: Vec<String> = p.views.iter().map(|f| f.to_string()).collect();
                out.push_str(&format!("panel{i}.views={}\n", vs.join("|")));
            }
            if !p.col_layout.is_empty() {
                out.push_str(&format!("panel{i}.cols={}\n", p.col_layout));
            }
            if !p.col_widths.is_empty() {
                let ws: Vec<String> = p.col_widths.iter().map(|w| w.to_string()).collect();
                out.push_str(&format!("panel{i}.colw={}\n", ws.join(",")));
            }
        }
        for (path, flags) in &self.dir_views {
            out.push_str(&format!("dirview={flags}|{}\n", path.to_string_lossy()));
        }
        out
    }

    /// 손상 입력 = 기본값(패닉 없음 · PREFS-731).
    pub(crate) fn parse(text: &str) -> Session {
        let mut s = Session::default();
        for (k, v) in kv_lines(text) {
            let idx = usize::from(k.starts_with("panel1"));
            let Some(sub) = k
                .strip_prefix("panel0.")
                .or_else(|| k.strip_prefix("panel1."))
            else {
                if k == "active_panel" {
                    s.active_panel = v.parse().unwrap_or(0).min(1);
                } else if k == "dirview" {
                    if let Some((f, path)) = v.split_once('|') {
                        if let (Ok(f), false) = (f.trim().parse::<u8>(), path.is_empty()) {
                            if s.dir_views.len() < DIR_VIEWS_MAX {
                                s.dir_views.push((PathBuf::from(path), f & 0xF));
                            }
                        }
                    }
                }
                continue;
            };
            let p = &mut s.panels[idx];
            match sub {
                "tabs" => {
                    p.tabs = v
                        .split('|')
                        .filter(|t| !t.is_empty())
                        .map(PathBuf::from)
                        .collect();
                }
                "active" => p.active = v.parse().unwrap_or(0),
                "locked" => p.locked = v.split('|').map(|f| f == "1").collect(),
                "pinned" => p.pinned = v.split('|').map(|f| f == "1").collect(),
                "views" => {
                    p.views = v
                        .split('|')
                        .map(|f| f.trim().parse::<u8>().unwrap_or(0) & 0xF)
                        .collect();
                }
                "cols" => p.col_layout = v.to_string(),
                "colw" => {
                    p.col_widths = v
                        .split(',')
                        .filter_map(|w| w.trim().parse::<i32>().ok())
                        .collect();
                }
                "modes" => {
                    p.modes = v
                        .split('|')
                        .map(|m| {
                            if matches!(m, "flat" | "tiles") {
                                m.to_string()
                            } else {
                                "tree".to_string()
                            }
                        })
                        .collect();
                }
                sub if sub.starts_with("exp") => {
                    let Ok(j) = sub["exp".len()..].parse::<usize>() else {
                        continue;
                    };
                    if j > 64 {
                        continue; // 손상 방어
                    }
                    if p.expanded.len() <= j {
                        p.expanded.resize(j + 1, Vec::new());
                    }
                    p.expanded[j] = v
                        .split('|')
                        .filter(|t| !t.is_empty())
                        .map(PathBuf::from)
                        .collect();
                }
                _ => {}
            }
        }
        s
    }

    /// 설정 폴더에서 읽는다 — `session.conf` → 없으면 dir2 `session.cfg` → 없으면 `None`.
    pub(crate) fn load(dir: &Path) -> Option<Session> {
        let text = std::fs::read_to_string(dir.join(FILE_NAME))
            .or_else(|_| std::fs::read_to_string(dir.join(LEGACY_FILE_NAME)))
            .ok()?;
        Some(Session::parse(&text))
    }

    /// 원자적 저장(임시 파일 + rename — 저장 중 크래시에도 기존 파일 보존 · dir2 SESS 규약).
    pub(crate) fn save(&self, dir: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(dir)?;
        let path = dir.join(FILE_NAME);
        let tmp = dir.join(format!("{FILE_NAME}.tmp"));
        std::fs::write(&tmp, self.serialize())?;
        if let Err(e) = std::fs::rename(&tmp, &path) {
            let _ = std::fs::remove_file(&tmp);
            return Err(e);
        }
        Ok(())
    }

    /// 탭이 하나도 없는 세션(= 복원할 것 없음).
    pub(crate) fn is_empty(&self) -> bool {
        self.panels.iter().all(|p| p.tabs.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// dir2 `session_roundtrip_with_pipe_separator` 그대로 + 보존 키.
    #[test]
    fn session_roundtrip_with_pipe_separator() {
        let s = Session {
            active_panel: 1,
            panels: [
                PanelSession {
                    tabs: vec![PathBuf::from("C:\\a"), PathBuf::from("D:\\b c\\d")],
                    active: 1,
                    expanded: vec![
                        vec![],
                        vec![
                            PathBuf::from("D:\\b c\\d\\sub"),
                            PathBuf::from("D:\\b c\\d\\한글"),
                        ],
                    ],
                    locked: vec![false, true],
                    pinned: vec![true, false],
                    modes: vec!["tiles".into(), "tree".into()],
                    views: vec![5, 2],
                    col_widths: vec![320, 64, 96],
                    col_layout: "cols:1[ext:1,name:1,size:0,modified:1,kind:1]".into(),
                },
                PanelSession {
                    tabs: vec![PathBuf::from("C:\\")],
                    active: 0,
                    ..PanelSession::default()
                },
            ],
            // 폴더별 보기 옵션 기억(경로에 `|`가 있어도 첫 `|`만 구분자) · 순서 보존.
            dir_views: vec![
                (PathBuf::from("/home/u/src"), 3),
                (PathBuf::from("D:\\b c\\d"), 6),
                (PathBuf::from("/tmp/a|b"), 0),
            ],
        };
        let text = s.serialize();
        assert!(text.contains("dirview=3|/home/u/src\n") && text.contains("dirview=0|/tmp/a|b\n"));
        assert!(text.starts_with("# nexa-dir session v1\nactive_panel=1\n"));
        let parsed = Session::parse(&text);
        assert_eq!(parsed, s);
        assert!(!parsed.is_empty());
        // 전부 tree면 modes 생략 · 빈 패널은 tabs= 빈 줄.
        let plain = Session::default();
        let t = plain.serialize();
        assert!(!t.contains("modes") && t.contains("panel1.tabs=\n"));
        assert!(Session::parse(&t).is_empty());
    }

    #[test]
    fn parse_is_tolerant_and_loads_legacy_name() {
        let s = Session::parse("garbage\npanel0.active=x\npanel5.tabs=a\nactive_panel=9\npanel1.exp99=a|b\npanel0.modes=flat|weird\n");
        assert_eq!(s.active_panel, 1, "범위 밖 = 1로 클램프");
        assert_eq!(s.panels[0].active, 0);
        assert_eq!(s.panels[0].modes, vec!["flat", "tree"]);
        assert!(
            s.panels[1].expanded.is_empty(),
            "exp 인덱스 > 64 = 손상 방어"
        );
        let dir = std::env::temp_dir().join(format!("ndir-session-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert!(Session::load(&dir).is_none());
        std::fs::write(
            dir.join(LEGACY_FILE_NAME),
            "# nexa-dir session v1\npanel0.tabs=C:\\x\n",
        )
        .unwrap();
        assert_eq!(
            Session::load(&dir).unwrap().panels[0].tabs,
            vec![PathBuf::from("C:\\x")]
        );
        let mut s2 = Session::default();
        s2.panels[1].tabs = vec![PathBuf::from("/tmp/z")];
        s2.save(&dir).unwrap();
        assert!(dir.join(FILE_NAME).is_file() && !dir.join(format!("{FILE_NAME}.tmp")).exists());
        assert_eq!(Session::load(&dir).unwrap(), s2, "conf가 cfg보다 우선");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
