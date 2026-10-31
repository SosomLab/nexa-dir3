//! 진행(`Progress` · 한 줄 JSON) · 결과(`Report` · JSON) — 앱 진행 창 · `nexa-copy --progress` · cdylib(후속)이 같은 모양.

use crate::json::{self, Json};
use std::path::{Path, PathBuf};

/// 진행 단계.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Plan,
    Transfer,
    Verify,
    Mirror,
    Delete,
}

impl Phase {
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Phase::Plan => "plan",
            Phase::Transfer => "transfer",
            Phase::Verify => "verify",
            Phase::Mirror => "mirror",
            Phase::Delete => "delete",
        }
    }
}

/// 진행 스냅샷.
#[derive(Debug, Clone, PartialEq)]
pub struct Progress {
    pub phase: Phase,
    pub done_bytes: u64,
    pub total_bytes: u64,
    pub item_index: usize,
    pub item_count: usize,
    /// 지금 쓰는 대상(있으면).
    pub current: Option<PathBuf>,
}

impl Progress {
    /// `{"phase":"transfer","done":1,"total":2,"item":0,"items":3,"current":"..."}` 한 줄.
    #[must_use]
    pub fn to_json_line(&self) -> String {
        let mut kv = vec![
            ("phase".to_string(), Json::Str(self.phase.name().into())),
            ("done".into(), Json::Num(self.done_bytes as f64)),
            ("total".into(), Json::Num(self.total_bytes as f64)),
            ("item".into(), Json::Num(self.item_index as f64)),
            ("items".into(), Json::Num(self.item_count as f64)),
        ];
        if let Some(c) = &self.current {
            kv.push((
                "current".into(),
                Json::Str(c.to_string_lossy().into_owned()),
            ));
        }
        json::dump(&Json::Obj(kv))
    }
}

/// 항목 하나의 결과/계획.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemReport {
    /// copy · move · duplicate · overwrite · skip · noop · error · delete(미러).
    pub action: String,
    /// planned · done · skipped · failed · canceled.
    pub status: String,
    pub bytes: u64,
    pub source: PathBuf,
    pub dest: PathBuf,
    pub note: String,
}

/// 검증 요약.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VerifySummary {
    pub files: u64,
    pub bytes: u64,
    pub mismatches: Vec<(PathBuf, String)>,
}

/// 작업 결과.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Report {
    pub ok: bool,
    pub canceled: bool,
    pub dry_run: bool,
    pub items: Vec<ItemReport>,
    pub transferred_bytes: u64,
    pub failed: usize,
    pub skipped: usize,
    pub deleted: usize,
    pub verify: Option<VerifySummary>,
    pub elapsed_ms: u64,
    /// 작업 전체 수준의 오류(명세 검증 · 대상 없음 · 미러 거부 …).
    pub errors: Vec<String>,
}

impl Report {
    #[must_use]
    pub fn failed_with(msg: impl Into<String>) -> Report {
        Report {
            ok: false,
            errors: vec![msg.into()],
            ..Report::default()
        }
    }

    /// 종료 코드 규칙(nexa-copy와 같다): 취소 130 · 실패/불일치/오류 1 · 그 밖 0.
    #[must_use]
    pub fn exit_code(&self) -> u8 {
        if self.canceled {
            130
        } else if self.ok {
            0
        } else {
            1
        }
    }

    #[must_use]
    pub fn to_json(&self) -> String {
        let p = |p: &Path| Json::Str(p.to_string_lossy().into_owned());
        let items = Json::Arr(
            self.items
                .iter()
                .map(|i| {
                    Json::Obj(vec![
                        ("action".into(), Json::Str(i.action.clone())),
                        ("status".into(), Json::Str(i.status.clone())),
                        ("bytes".into(), Json::Num(i.bytes as f64)),
                        ("source".into(), p(&i.source)),
                        ("dest".into(), p(&i.dest)),
                        ("note".into(), Json::Str(i.note.clone())),
                    ])
                })
                .collect(),
        );
        let verify = match &self.verify {
            None => Json::Null,
            Some(v) => Json::Obj(vec![
                ("files".into(), Json::Num(v.files as f64)),
                ("bytes".into(), Json::Num(v.bytes as f64)),
                (
                    "mismatches".into(),
                    Json::Arr(
                        v.mismatches
                            .iter()
                            .map(|(path, why)| {
                                Json::Obj(vec![
                                    ("path".into(), p(path)),
                                    ("why".into(), Json::Str(why.clone())),
                                ])
                            })
                            .collect(),
                    ),
                ),
            ]),
        };
        json::dump(&Json::Obj(vec![
            ("ok".into(), Json::Bool(self.ok)),
            ("canceled".into(), Json::Bool(self.canceled)),
            ("dry_run".into(), Json::Bool(self.dry_run)),
            (
                "transferred_bytes".into(),
                Json::Num(self.transferred_bytes as f64),
            ),
            ("failed".into(), Json::Num(self.failed as f64)),
            ("skipped".into(), Json::Num(self.skipped as f64)),
            ("deleted".into(), Json::Num(self.deleted as f64)),
            ("elapsed_ms".into(), Json::Num(self.elapsed_ms as f64)),
            ("verify".into(), verify),
            (
                "errors".into(),
                Json::Arr(self.errors.iter().map(|e| Json::Str(e.clone())).collect()),
            ),
            ("items".into(), items),
        ]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_line_and_report_json_parse_back() {
        let p = Progress {
            phase: Phase::Transfer,
            done_bytes: 5,
            total_bytes: 10,
            item_index: 1,
            item_count: 2,
            current: Some(PathBuf::from("/b/x")),
        };
        let j = json::parse(&p.to_json_line()).unwrap();
        assert_eq!(j.get("phase").and_then(Json::as_str), Some("transfer"));
        assert_eq!(j.get("done").and_then(Json::as_u64), Some(5));
        let r = Report {
            ok: false,
            failed: 1,
            items: vec![ItemReport {
                action: "copy".into(),
                status: "failed".into(),
                bytes: 3,
                source: PathBuf::from("/a"),
                dest: PathBuf::from("/b"),
                note: "boom".into(),
            }],
            verify: Some(VerifySummary {
                files: 1,
                bytes: 6,
                mismatches: vec![(PathBuf::from("/b"), "size differs".into())],
            }),
            ..Report::default()
        };
        assert_eq!(r.exit_code(), 1);
        let j = json::parse(&r.to_json()).unwrap();
        assert_eq!(j.get("failed").and_then(Json::as_u64), Some(1));
        assert_eq!(
            j.get("items").and_then(Json::as_arr).map(<[Json]>::len),
            Some(1)
        );
        assert!(j.get("verify").and_then(|v| v.get("mismatches")).is_some());
        assert_eq!(
            Report {
                canceled: true,
                ..Report::default()
            }
            .exit_code(),
            130
        );
        assert_eq!(
            Report {
                ok: true,
                ..Report::default()
            }
            .exit_code(),
            0
        );
    }
}
