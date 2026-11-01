//! 재개(T-202 ② · docs/26 §8 "재개"): **저널** = 항목이 **시작**할 때(`started` · note = `new`/`overwrite`)와 **끝날** 때(`done`) Report `items[]`
//! 원소와 같은 모양의 JSON 한 줄을 덧붙인 파일 · **재개** = 이전 Report JSON **또는** 저널을 읽어 끝 상태가 `done`인 쌍은 건너뛰고(`skipped` ·
//! `note` = `resumed`), 끝 상태가 `started`인 쌍(끊긴 항목)은 **이전 전송이 만든 부분 대상**으로 보아 덮어쓰며 스테이징 잔여를 치운다(GAP-030).
//!
//! - 단위 = 최상위 항목(파일 중간 재개 없음). 프로세스가 죽으면 새 대상의 부분 파일 · 덮어쓰기 중이던 스테이징 사본이 **남는다**(11-01 kill -9 실측 —
//!   오류·취소 경로만 정리한다) → `started` 줄이 그 대상이 우리 것임을 보증한다(사용자 결정 11-01: 이전 전송이 만든 대상만 덮어쓰기 기본).
//! - 미러는 재비교가 곧 재개라 대상이 아니다(`validate`가 거른다).
//! - 같은 파일을 `journal`과 `resume`에 함께 주면 "끊긴 데서 이어서"가 된다(먼저 읽고 나서 덧붙여 연다).

use crate::json::{self, Json};
use crate::report::ItemReport;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;

/// 완료 저널(덧붙임 전용 · 줄 하나 = 항목 하나 · `write_all` 한 번 + flush = 줄 단위 원자성).
#[derive(Debug)]
pub struct Journal {
    file: File,
}

impl Journal {
    /// 열기 — 없으면 만들고 있으면 뒤에 덧붙인다.
    pub fn open(path: &Path) -> std::io::Result<Journal> {
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        Ok(Journal { file })
    }

    /// 항목 한 줄 기록.
    pub fn record(&mut self, item: &ItemReport) -> std::io::Result<()> {
        let mut line = json::dump(&item.to_json_value());
        line.push('\n');
        self.file.write_all(line.as_bytes())?;
        self.file.flush()
    }
}

/// 이전 Report JSON(`{"items":[…]}`) **또는** 저널(JSON 줄)에서 **끝 상태가 `done`인** 항목을 뽑는다.
/// 같은 (source, dest) 쌍이 여러 번 나오면 **마지막 기록이 이긴다**(started → done · failed → done 순서 · 같은 파일을 journal과 resume에 같이 쓸 때).
/// 저널의 **마지막** 줄이 잘려 있으면(쓰다 끊김) 그 줄만 버린다 · 그 밖의 깨진 줄은 오류.
pub fn done_items(text: &str) -> Result<Vec<ItemReport>, String> {
    Ok(folded_items(text)?
        .into_iter()
        .filter(|i| i.status == "done")
        .collect())
}

/// 끝 상태가 `started`인 쌍 = 끊긴 항목(GAP-030): 재개가 그 `dest`를 덮어쓰고 스테이징 잔여를 치운다 · note = 당시 결정(`new`/`overwrite`).
pub fn interrupted_items(text: &str) -> Result<Vec<ItemReport>, String> {
    Ok(folded_items(text)?
        .into_iter()
        .filter(|i| i.status == "started")
        .collect())
}

/// 쌍별 마지막 기록(입력 순서 유지).
fn folded_items(text: &str) -> Result<Vec<ItemReport>, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    let mut all: Vec<ItemReport> = Vec::new();
    // 전체가 객체 하나 = Report(여러 줄로 보기 좋게 찍혀 있어도) 또는 한 줄짜리 저널.
    if let Ok(j @ Json::Obj(_)) = json::parse(trimmed) {
        match j.get("items").and_then(Json::as_arr) {
            Some(items) => all.extend(items.iter().filter_map(ItemReport::from_json_value)),
            None => all.extend(ItemReport::from_json_value(&j)),
        }
    } else {
        let lines: Vec<&str> = trimmed
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .collect();
        let last = lines.len().saturating_sub(1);
        for (n, line) in lines.iter().enumerate() {
            match json::parse(line) {
                Ok(j) => all.extend(ItemReport::from_json_value(&j)),
                Err(_) if n == last => break, // 잘린 마지막 줄 = 그 항목은 완료 아님
                Err(e) => return Err(format!("line {}: {e}", n + 1)),
            }
        }
    }
    // 쌍별 마지막 기록만 남긴다(입력 순서 유지).
    let mut folded: Vec<ItemReport> = Vec::with_capacity(all.len());
    for it in all {
        if let Some(slot) = folded
            .iter_mut()
            .find(|f| f.source == it.source && f.dest == it.dest)
        {
            *slot = it;
        } else {
            folded.push(it);
        }
    }
    Ok(folded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::Report;
    use std::path::PathBuf;

    fn item(src: &str, status: &str) -> ItemReport {
        ItemReport {
            action: "copy".into(),
            status: status.into(),
            bytes: 5,
            source: PathBuf::from(src),
            dest: PathBuf::from("/d").join(src.trim_start_matches('/')),
            note: String::new(),
        }
    }

    /// Report 전체 · 저널 줄 · 잘린 마지막 줄 · 깨진 중간 줄 · 빈 글.
    #[test]
    fn done_items_from_report_and_journal() {
        let rep = Report {
            ok: false,
            items: vec![item("/a", "done"), item("/b", "failed"), item("/c", "done")],
            ..Report::default()
        };
        let got = done_items(&rep.to_json()).unwrap();
        assert_eq!(
            got.iter().map(|i| i.source.clone()).collect::<Vec<_>>(),
            vec![PathBuf::from("/a"), PathBuf::from("/c")]
        );
        let journal = format!(
            "{}\n{}\n{}\n{{\"action\":\"copy\",\"sta",
            json::dump(&item("/a", "done").to_json_value()),
            json::dump(&item("/b", "skipped").to_json_value()),
            json::dump(&item("/c", "done").to_json_value()),
        );
        let got = done_items(&journal).unwrap();
        assert_eq!(got.len(), 2, "{got:?}");
        assert_eq!(
            (got[1].bytes, got[1].dest.as_path()),
            (5, Path::new("/d/c"))
        );
        // 한 줄짜리 저널도 읽힌다.
        assert_eq!(
            done_items(&json::dump(&item("/a", "done").to_json_value()))
                .unwrap()
                .len(),
            1
        );
        assert!(done_items("   \n").unwrap().is_empty());
        // 같은 쌍은 마지막 기록이 이긴다: done → failed = 완료 아님 · failed → done = 완료.
        let flip = format!(
            "{}\n{}\n{}\n{}",
            json::dump(&item("/a", "done").to_json_value()),
            json::dump(&item("/a", "failed").to_json_value()),
            json::dump(&item("/b", "failed").to_json_value()),
            json::dump(&item("/b", "done").to_json_value()),
        );
        let got = done_items(&flip).unwrap();
        assert_eq!(
            got.iter().map(|i| i.source.clone()).collect::<Vec<_>>(),
            vec![PathBuf::from("/b")],
            "{got:?}"
        );
        // 시작 줄(GAP-030): started → done = 완료 · started만 = 끊김(interrupted) · done 뒤 started(다시 시작) = 끊김.
        let seq = format!(
            "{}\n{}\n{}\n{}\n{}",
            json::dump(&item("/a", "started").to_json_value()),
            json::dump(&item("/a", "done").to_json_value()),
            json::dump(&item("/b", "started").to_json_value()),
            json::dump(&item("/c", "done").to_json_value()),
            json::dump(&item("/c", "started").to_json_value()),
        );
        let done = done_items(&seq).unwrap();
        assert_eq!(
            done.iter().map(|i| i.source.clone()).collect::<Vec<_>>(),
            vec![PathBuf::from("/a")]
        );
        let cut = interrupted_items(&seq).unwrap();
        assert_eq!(
            cut.iter().map(|i| i.source.clone()).collect::<Vec<_>>(),
            vec![PathBuf::from("/b"), PathBuf::from("/c")]
        );
        let bad = format!(
            "{}\nnot json\n{}",
            json::dump(&item("/a", "done").to_json_value()),
            json::dump(&item("/c", "done").to_json_value())
        );
        assert!(done_items(&bad).unwrap_err().starts_with("line 2"));
    }

    /// 저널 쓰기 = 덧붙임(두 번 열어도 이어진다) · 되읽기 왕복.
    #[test]
    fn journal_appends_and_reads_back() {
        let d = std::env::temp_dir().join(format!("ndir-xfer-journal-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let p = d.join("job.log");
        {
            let mut j = Journal::open(&p).unwrap();
            j.record(&item("/a", "done")).unwrap();
        }
        {
            let mut j = Journal::open(&p).unwrap();
            j.record(&item("/b", "done")).unwrap();
        }
        let text = std::fs::read_to_string(&p).unwrap();
        assert_eq!(text.lines().count(), 2, "{text}");
        let got = done_items(&text).unwrap();
        assert_eq!(got, vec![item("/a", "done"), item("/b", "done")]);
        let _ = std::fs::remove_dir_all(&d);
    }
}
