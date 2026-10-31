//! 작업 명세 `JobSpec` — 직렬화 가능(JSON · T-97 "직렬화 가능한 JobSpec") · 앱 · `nexa-copy --job` · 후속 cdylib이 같은 모양을 쓴다.
//!
//! ```json
//! {"op":"copy","sources":["/a/x.txt","/a/dir"],"dest":"/b","conflict":"skip","threads":0,"native":true,"unbuffered_mb":64,
//!  "include":["*.txt"],"exclude":[],"verify":false,"mirror":false,"dry_run":false,"yes":false,
//!  "journal":null,"resume":null}   (unbuffered_mb 기본 0 · journal/resume = 재개 T-202 ② · 경로 문자열 또는 null)
//! ```
//! 빠진 키 = 기본값 · 모르는 키 = 무시(앞으로 호환) · `op` = `copy`|`move` · `conflict` = `skip`|`overwrite`.

use crate::json::{self, Json};
use ndir_ops::fastcopy::NameFilter;
use ndir_ops::{Conflict, Op};
use std::path::PathBuf;

/// 충돌 정책(대상에 이미 있을 때).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConflictPolicy {
    #[default]
    Skip,
    Overwrite,
}

impl ConflictPolicy {
    #[must_use]
    pub fn resolve(self) -> Conflict {
        match self {
            ConflictPolicy::Skip => Conflict::Skip,
            ConflictPolicy::Overwrite => Conflict::Overwrite,
        }
    }
}

/// 작업 명세.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobSpec {
    pub op: Op,
    pub sources: Vec<PathBuf>,
    pub dest: PathBuf,
    pub conflict: ConflictPolicy,
    /// 0 = 자동(장치 판정).
    pub threads: usize,
    pub native: bool,
    pub unbuffered_mb: u64,
    pub filter: NameFilter,
    pub verify: bool,
    pub mirror: bool,
    pub dry_run: bool,
    /// 미러의 영구 삭제 확인.
    pub yes: bool,
    /// 완료 저널(T-202 ② · `resume.rs`): 최상위 항목이 끝날 때마다 Report 항목 모양 JSON 한 줄을 덧붙인다 · dry-run이면 쓰지 않음 · None = 안 씀.
    pub journal: Option<PathBuf>,
    /// 재개 입력(이전 Report JSON 또는 저널): 끝 상태가 `done`인 (source, dest) 쌍은 건너뛴다(`status` skipped · `note` resumed) · 미러에는 못 씀.
    pub resume: Option<PathBuf>,
}

impl Default for JobSpec {
    fn default() -> Self {
        JobSpec {
            op: Op::Copy,
            sources: Vec::new(),
            dest: PathBuf::new(),
            conflict: ConflictPolicy::Skip,
            threads: 0,
            native: true,
            unbuffered_mb: 0, // 엔진 기본과 같다(캐시 우회 = 실측 손해 · docs/24 §6-2 · 간단 인자와 한 길)
            filter: NameFilter::default(),
            verify: false,
            mirror: false,
            dry_run: false,
            yes: false,
            journal: None,
            resume: None,
        }
    }
}

fn strs(v: Option<&Json>, key: &str) -> Result<Vec<String>, String> {
    match v {
        None | Some(Json::Null) => Ok(Vec::new()),
        Some(Json::Arr(items)) => items
            .iter()
            .map(|i| {
                i.as_str()
                    .map(str::to_string)
                    .ok_or_else(|| format!("{key}: expected strings"))
            })
            .collect(),
        Some(_) => Err(format!("{key}: expected an array of strings")),
    }
}

fn path(v: Option<&Json>, key: &str) -> Result<Option<PathBuf>, String> {
    match v {
        None | Some(Json::Null) => Ok(None),
        Some(Json::Str(s)) if s.is_empty() => Ok(None),
        Some(Json::Str(s)) => Ok(Some(PathBuf::from(s))),
        Some(_) => Err(format!("{key}: expected a path string")),
    }
}

fn flag(v: Option<&Json>, key: &str, default: bool) -> Result<bool, String> {
    match v {
        None | Some(Json::Null) => Ok(default),
        Some(j) => j
            .as_bool()
            .ok_or_else(|| format!("{key}: expected true/false")),
    }
}

impl JobSpec {
    /// JSON → 명세(검증: `dest` 필수 · 미러가 아니면 `sources` 1개 이상 · 미러면 정확히 1개).
    #[allow(clippy::field_reassign_with_default)] // 키마다 검증하며 채운다(리터럴이면 검증 순서가 흩어진다)
    pub fn from_json(text: &str) -> Result<JobSpec, String> {
        let j = json::parse(text)?;
        if !matches!(j, Json::Obj(_)) {
            return Err("job: expected a JSON object".into());
        }
        let mut s = JobSpec::default();
        s.op = match j.get("op").and_then(Json::as_str).unwrap_or("copy") {
            "copy" => Op::Copy,
            "move" => Op::Move,
            other => return Err(format!("op: expected copy|move, got '{other}'")),
        };
        s.sources = strs(j.get("sources"), "sources")?
            .into_iter()
            .map(PathBuf::from)
            .collect();
        s.dest = PathBuf::from(
            j.get("dest")
                .and_then(Json::as_str)
                .filter(|d| !d.is_empty())
                .ok_or("dest: required")?,
        );
        s.conflict = match j.get("conflict").and_then(Json::as_str).unwrap_or("skip") {
            "skip" => ConflictPolicy::Skip,
            "overwrite" => ConflictPolicy::Overwrite,
            other => return Err(format!("conflict: expected skip|overwrite, got '{other}'")),
        };
        if let Some(t) = j.get("threads") {
            s.threads = t.as_u64().ok_or("threads: expected a number")?.min(64) as usize;
        }
        if let Some(u) = j.get("unbuffered_mb") {
            s.unbuffered_mb = u.as_u64().ok_or("unbuffered_mb: expected a number")?;
        }
        s.native = flag(j.get("native"), "native", true)?;
        s.filter = NameFilter {
            include: strs(j.get("include"), "include")?,
            exclude: strs(j.get("exclude"), "exclude")?,
        };
        s.verify = flag(j.get("verify"), "verify", false)?;
        s.mirror = flag(j.get("mirror"), "mirror", false)?;
        s.dry_run = flag(j.get("dry_run"), "dry_run", false)?;
        s.yes = flag(j.get("yes"), "yes", false)?;
        s.journal = path(j.get("journal"), "journal")?;
        s.resume = path(j.get("resume"), "resume")?;
        if s.mirror && s.sources.len() != 1 {
            return Err("mirror: needs exactly one source folder".into());
        }
        if s.sources.is_empty() {
            return Err("sources: need at least one".into());
        }
        Ok(s)
    }

    /// 상대 경로를 `base`(잡 파일이 있는 폴더 · 사용자 결정 10-31 §11 ①) 기준으로 푼다 — 절대 경로는 그대로.
    #[must_use]
    pub fn resolved_against(mut self, base: &std::path::Path) -> JobSpec {
        let fix = |p: &PathBuf| {
            if p.is_absolute() {
                p.clone()
            } else {
                base.join(p)
            }
        };
        self.sources = self.sources.iter().map(fix).collect();
        self.dest = fix(&self.dest);
        self.journal = self.journal.as_ref().map(fix);
        self.resume = self.resume.as_ref().map(fix);
        self
    }

    /// 명세 → JSON(모든 키 · 입력 순서 고정).
    #[must_use]
    pub fn to_json(&self) -> String {
        let strs = |v: &[String]| Json::Arr(v.iter().map(|s| Json::Str(s.clone())).collect());
        let paths = |v: &[PathBuf]| {
            Json::Arr(
                v.iter()
                    .map(|p| Json::Str(p.to_string_lossy().into_owned()))
                    .collect(),
            )
        };
        json::dump(&Json::Obj(vec![
            (
                "op".into(),
                Json::Str(if self.op == Op::Move { "move" } else { "copy" }.into()),
            ),
            ("sources".into(), paths(&self.sources)),
            (
                "dest".into(),
                Json::Str(self.dest.to_string_lossy().into_owned()),
            ),
            (
                "conflict".into(),
                Json::Str(
                    if self.conflict == ConflictPolicy::Overwrite {
                        "overwrite"
                    } else {
                        "skip"
                    }
                    .into(),
                ),
            ),
            ("threads".into(), Json::Num(self.threads as f64)),
            ("native".into(), Json::Bool(self.native)),
            ("unbuffered_mb".into(), Json::Num(self.unbuffered_mb as f64)),
            ("include".into(), strs(&self.filter.include)),
            ("exclude".into(), strs(&self.filter.exclude)),
            ("verify".into(), Json::Bool(self.verify)),
            ("mirror".into(), Json::Bool(self.mirror)),
            ("dry_run".into(), Json::Bool(self.dry_run)),
            ("yes".into(), Json::Bool(self.yes)),
            ("journal".into(), opt_path(self.journal.as_deref())),
            ("resume".into(), opt_path(self.resume.as_deref())),
        ]))
    }
}

fn opt_path(p: Option<&std::path::Path>) -> Json {
    p.map_or(Json::Null, |p| Json::Str(p.to_string_lossy().into_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_roundtrip_and_validation() {
        let s = JobSpec {
            op: Op::Move,
            sources: vec![PathBuf::from("/a/x"), PathBuf::from("/a/y")],
            dest: PathBuf::from("/b"),
            conflict: ConflictPolicy::Overwrite,
            threads: 4,
            native: false,
            unbuffered_mb: 8,
            filter: NameFilter {
                include: vec!["*.txt".into()],
                exclude: vec!["*.tmp".into()],
            },
            verify: true,
            mirror: false,
            dry_run: true,
            yes: false,
            journal: Some(PathBuf::from("/j/log")),
            resume: Some(PathBuf::from("/j/prev.json")),
        };
        assert_eq!(JobSpec::from_json(&s.to_json()).unwrap(), s);
        // journal/resume: 빠짐 · null · 빈 글 = None · 문자열 아님 = 오류.
        let d = JobSpec::from_json(r#"{"sources":["/a"],"dest":"/b","journal":null,"resume":""}"#)
            .unwrap();
        assert!(d.journal.is_none() && d.resume.is_none());
        assert!(
            JobSpec::from_json(r#"{"sources":["/a"],"dest":"/b","resume":1}"#)
                .unwrap_err()
                .starts_with("resume")
        );
        // 빠진 키 = 기본값 · 모르는 키 무시.
        let d = JobSpec::from_json(r#"{"sources":["/a"],"dest":"/b","future":1}"#).unwrap();
        assert_eq!(d.op, Op::Copy);
        assert_eq!(d.conflict, ConflictPolicy::Skip);
        assert_eq!(
            d.unbuffered_mb, 0,
            "엔진·간단 인자와 같은 기본(협업 지적 10-31)"
        );
        assert!(d.native && !d.verify && !d.mirror);
        // 검증.
        assert!(JobSpec::from_json(r#"{"sources":["/a"]}"#)
            .unwrap_err()
            .starts_with("dest"));
        assert!(JobSpec::from_json(r#"{"sources":[],"dest":"/b"}"#)
            .unwrap_err()
            .starts_with("sources"));
        assert!(
            JobSpec::from_json(r#"{"sources":["/a","/c"],"dest":"/b","mirror":true}"#)
                .unwrap_err()
                .starts_with("mirror")
        );
        assert!(
            JobSpec::from_json(r#"{"sources":["/a"],"dest":"/b","op":"zap"}"#)
                .unwrap_err()
                .starts_with("op")
        );
        assert!(JobSpec::from_json("[1]").is_err());
        // 상대 경로 = 기준 폴더 아래 · 절대 경로는 그대로.
        let abs = if cfg!(windows) {
            "C:\\abs\\x"
        } else {
            "/abs/x"
        };
        let r = JobSpec::from_json(&format!(
            r#"{{"sources":["src/a","{}"],"dest":"out","journal":"job.log"}}"#,
            abs.replace('\\', "\\\\")
        ))
        .unwrap()
        .resolved_against(std::path::Path::new("/jobs"));
        assert_eq!(r.sources[0], PathBuf::from("/jobs").join("src/a"));
        assert_eq!(r.sources[1], PathBuf::from(abs));
        assert_eq!(r.dest, PathBuf::from("/jobs").join("out"));
        assert_eq!(r.journal, Some(PathBuf::from("/jobs").join("job.log")));
    }
}
