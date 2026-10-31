//! 재개(T-202 ② · docs/26 §8 "재개"): **저널** = 항목이 끝날 때마다 Report `items[]` 원소와 같은 모양의 JSON 한 줄을 덧붙인 파일 ·
//! **재개** = 이전 Report JSON **또는** 저널을 읽어 `status":"done"` 항목의 source를 건너뛴다(`run`이 `skipped` · `note` = `resumed`).
//!
//! - 단위 = 최상위 항목(파일 중간 재개 없음 — 스테이징 교체가 원자적이라 부분 파일이 남지 않는다 · docs/24).
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
/// 같은 (source, dest) 쌍이 여러 번 나오면 **마지막 기록이 이긴다**(failed → done 순서 · 같은 파일을 journal과 resume에 같이 쓸 때).
/// 저널의 **마지막** 줄이 잘려 있으면(쓰다 끊김) 그 줄만 버린다 · 그 밖의 깨진 줄은 오류.
pub fn done_items(text: &str) -> Result<Vec<ItemReport>, String> {
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
    Ok(folded.into_iter().filter(|i| i.status == "done").collect())
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
