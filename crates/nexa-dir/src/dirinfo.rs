//! 폴더 정보(탭 상태바 칸 · docs/22 NEW-004 · NEW-005 1차) — **Git 브랜치**(프로세스 없이 `.git/HEAD`를 읽는다)와
//! 폴더 항목 수. 해석은 순수 함수(고정물 시험) · 읽기는 작은 파일 몇 개뿐이라 UI 스레드에서 한다(네트워크 경로는 호출부가 건너뜀).

use std::path::{Path, PathBuf};

/// `.git/HEAD` 본문 → 표시할 이름(순수): `ref: refs/heads/<브랜치>` = 브랜치 · 그 밖의 ref = 끝 이름 · 해시(분리된 HEAD) = 앞 7자.
pub(crate) fn parse_head(text: &str) -> Option<String> {
    let line = text.lines().next()?.trim();
    if let Some(r) = line.strip_prefix("ref:") {
        let r = r.trim();
        let name = r.strip_prefix("refs/heads/").unwrap_or(r);
        return (!name.is_empty()).then(|| name.to_string());
    }
    (line.len() >= 7 && line.chars().all(|c| c.is_ascii_hexdigit())).then(|| line[..7].to_string())
}

/// `.git` **파일**(worktree · 서브모듈) 본문 → 실제 git 폴더 경로(순수): `gitdir: <경로>`.
pub(crate) fn parse_gitdir_file(text: &str) -> Option<&str> {
    let p = text.lines().next()?.trim().strip_prefix("gitdir:")?.trim();
    (!p.is_empty()).then_some(p)
}

/// 저장소 루트의 **git 디렉터리**(`.git` 폴더 · worktree식 `.git` 파일이면 그 안의 `gitdir:`) — 상태 갱신 감시용(10-06).
pub(crate) fn git_dir(repo: &Path) -> Option<PathBuf> {
    let dot = repo.join(".git");
    let meta = std::fs::metadata(&dot).ok()?;
    if meta.is_dir() {
        return Some(dot);
    }
    let text = std::fs::read_to_string(&dot).ok()?;
    let p = Path::new(parse_gitdir_file(&text)?);
    Some(if p.is_absolute() {
        p.to_path_buf()
    } else {
        repo.join(p)
    })
}

/// `dir`에서 위로 올라가며 저장소를 찾아 `(저장소 루트, 브랜치)`를 돌려준다(없으면 `None`).
pub(crate) fn git_branch(dir: &Path) -> Option<(PathBuf, String)> {
    let mut cur = Some(dir);
    let mut hops = 0;
    while let Some(d) = cur {
        let dot = d.join(".git");
        if let Ok(meta) = std::fs::metadata(&dot) {
            let git_dir = if meta.is_dir() {
                dot
            } else {
                let text = std::fs::read_to_string(&dot).ok()?;
                let p = Path::new(parse_gitdir_file(&text)?);
                if p.is_absolute() {
                    p.to_path_buf()
                } else {
                    d.join(p)
                }
            };
            let head = std::fs::read_to_string(git_dir.join("HEAD")).ok()?;
            return Some((d.to_path_buf(), parse_head(&head)?));
        }
        hops += 1;
        if hops > 64 {
            return None;
        }
        cur = d.parent();
    }
    None
}

/// 저장소 상태 요약(`git status --porcelain=v2 --branch` — NEW-005 2차).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct GitDetail {
    /// 업스트림(`origin/main` · 없으면 `None`).
    pub upstream: Option<String>,
    /// 업스트림보다 앞선 · 뒤진 커밋 수.
    pub ahead: u32,
    pub behind: u32,
    /// 스테이지된(X ∈ AMDRCT) · 작업 트리 수정(Y ∈ MT) · 미추적 · 충돌 항목 수.
    pub staged: u32,
    pub changed: u32,
    pub untracked: u32,
    pub conflicts: u32,
    /// 삭제(X 또는 Y = D · Starship 규칙 = 스테이지 여부와 무관) · 이름 변경(스테이지된 R) · stash 수.
    pub deleted: u32,
    pub renamed: u32,
    pub stash: u32,
}

impl GitDetail {
    /// 작업 트리가 깨끗한가(stash는 작업 트리가 아니다).
    pub(crate) fn is_clean(&self) -> bool {
        self.staged + self.changed + self.untracked + self.conflicts + self.deleted == 0
    }

    /// 탭 상태바 칸에 덧붙이는 짧은 요약(없으면 빈 글) — **Starship 계열**(Starship · Spaceship · Powerlevel10k가 같이 쓰는 사실상
    /// 표준 · 사용자 10-06): `⇡1⇣2 +1 !3 ?2 ✘1 »1 =1 $1` = 앞섬/뒤짐 · 스테이지 · 수정 · 미추적 · 삭제 · 이름 변경 · 충돌 · stash.
    /// 0인 칸은 뺀다(종전 `↑1 ↓2 ●3` = 합계 한 칸 → 종류별로).
    pub(crate) fn short(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        let mut ab = String::new();
        if self.ahead > 0 {
            ab.push_str(&format!("\u{21E1}{}", self.ahead));
        }
        if self.behind > 0 {
            ab.push_str(&format!("\u{21E3}{}", self.behind));
        }
        if !ab.is_empty() {
            parts.push(ab);
        }
        for (sym, n) in [
            ("+", self.staged),
            ("!", self.changed),
            ("?", self.untracked),
            ("\u{2718}", self.deleted),
            ("\u{00BB}", self.renamed),
            ("=", self.conflicts),
            ("$", self.stash),
        ] {
            if n > 0 {
                parts.push(format!("{sym}{n}"));
            }
        }
        parts.join(" ")
    }
}

/// `git status --porcelain=v2 --branch` 출력 → 요약(순수). 머리 줄 = `# branch.upstream <이름>` · `# branch.ab +A -B` ·
/// 항목 줄 = `1`/`2`(XY 두 글자 — X = 스테이지 · Y = 작업 트리 · `.` = 변화 없음) · `u`(충돌) · `?`(미추적) · `!`(무시)는 세지 않는다.
pub(crate) fn parse_porcelain_v2(text: &str) -> GitDetail {
    let mut d = GitDetail::default();
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("# branch.upstream ") {
            d.upstream = Some(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix("# stash ") {
            // `--show-stash`(git ≥ 2.19) — stash가 0이면 줄 자체가 없다.
            d.stash = rest.trim().parse().unwrap_or(0);
        } else if let Some(rest) = line.strip_prefix("# branch.ab ") {
            for tok in rest.split_whitespace() {
                if let Some(n) = tok.strip_prefix('+') {
                    d.ahead = n.parse().unwrap_or(0);
                } else if let Some(n) = tok.strip_prefix('-') {
                    d.behind = n.parse().unwrap_or(0);
                }
            }
        } else if line.starts_with("1 ") || line.starts_with("2 ") {
            let mut xy = line[2..].chars();
            let (x, y) = (xy.next().unwrap_or('.'), xy.next().unwrap_or('.'));
            // Starship 규칙: 스테이지 = X가 바뀜(A·M·D·R·C·T) · 수정 = Y ∈ {M, T} · 삭제 = X 또는 Y = D · 이름 변경 = X = R.
            if x != '.' {
                d.staged += 1;
            }
            if matches!(y, 'M' | 'T') {
                d.changed += 1;
            }
            if x == 'D' || y == 'D' {
                d.deleted += 1;
            }
            if x == 'R' {
                d.renamed += 1;
            }
        } else if line.starts_with("u ") {
            d.conflicts += 1;
        } else if line.starts_with("? ") {
            d.untracked += 1;
        }
    }
    d
}

/// 폴더 바로 아래 항목 수 `(폴더, 파일)` — 숨김 · 점 파일 포함 **전부**(상세 표시용 · 읽기 실패 = `None`).
pub(crate) fn count_entries(dir: &Path) -> Option<(usize, usize)> {
    let mut n = (0, 0);
    for e in std::fs::read_dir(dir).ok()?.flatten() {
        if e.file_type().is_ok_and(|t| t.is_dir()) {
            n.0 += 1;
        } else {
            n.1 += 1;
        }
    }
    Some(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn head_and_gitdir_parsers() {
        assert_eq!(
            parse_head("ref: refs/heads/main\n").as_deref(),
            Some("main")
        );
        assert_eq!(
            parse_head("ref: refs/heads/feat/x-1\n").as_deref(),
            Some("feat/x-1")
        );
        assert_eq!(
            parse_head("ref: refs/remotes/origin/main").as_deref(),
            Some("refs/remotes/origin/main")
        );
        assert_eq!(
            parse_head("0123456789abcdef0123456789abcdef01234567\n").as_deref(),
            Some("0123456")
        );
        assert_eq!(parse_head(""), None);
        assert_eq!(parse_head("garbage here"), None);
        assert_eq!(parse_head("ref: "), None);
        assert_eq!(
            parse_gitdir_file("gitdir: ../.git/worktrees/w1\n"),
            Some("../.git/worktrees/w1")
        );
        assert_eq!(parse_gitdir_file("nope"), None);
    }

    /// porcelain v2 고정물: 업스트림 · 앞섬/뒤짐 · 스테이지/변경/미추적/충돌 수 · 짧은 요약.
    #[test]
    fn porcelain_v2_summary() {
        let text = "# branch.oid 1234abcd\n# branch.head main\n# branch.upstream origin/main\n# branch.ab +2 -1\n\
1 M. N... 100644 100644 100644 aaa bbb staged.rs\n1 .M N... 100644 100644 100644 aaa bbb changed.rs\n\
1 MM N... 100644 100644 100644 aaa bbb both.rs\n2 R. N... 100644 100644 100644 aaa bbb R100 new.rs\told.rs\n\
u UU N... 100644 100644 100644 100644 a b c conflict.rs\n? untracked.txt\n? other.txt\n! ignored.log\n";
        let d = parse_porcelain_v2(text);
        assert_eq!(d.upstream.as_deref(), Some("origin/main"));
        assert_eq!((d.ahead, d.behind), (2, 1));
        assert_eq!(
            (
                d.staged,
                d.changed,
                d.untracked,
                d.conflicts,
                d.deleted,
                d.renamed
            ),
            (3, 2, 2, 1, 0, 1)
        );
        assert!(!d.is_clean());
        assert_eq!(
            d.short(),
            "\u{21E1}2\u{21E3}1 +3 !2 ?2 \u{00BB}1 =1",
            "Starship 계열"
        );
        let clean = parse_porcelain_v2("# branch.oid x\n# branch.head main\n");
        assert!(clean.is_clean() && clean.upstream.is_none());
        assert_eq!(clean.short(), "");
        // 삭제(스테이지 여부 무관) · stash · 뒤짐만.
        let del = parse_porcelain_v2(
            "# branch.ab +0 -3\n# stash 2\n1 D. N... 100644 000000 000000 aaa bbb gone.rs\n1 .D N... 100644 100644 000000 aaa bbb wt.rs\n",
        );
        assert_eq!((del.staged, del.changed, del.deleted), (1, 0, 2));
        assert!(!del.is_clean());
        assert_eq!(del.short(), "\u{21E3}3 +1 \u{2718}2 $2");
    }

    /// 실제 폴더: 하위 폴더에서 위로 올라가 저장소를 찾는다 · worktree식 `.git` 파일 · 저장소 밖 = None · 항목 수.
    #[test]
    fn finds_repo_upwards_and_counts_entries() {
        let base = std::env::temp_dir().join(format!("ndir-dirinfo-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let repo = base.join("repo");
        let deep = repo.join("a").join("b");
        std::fs::create_dir_all(&deep).unwrap();
        std::fs::create_dir_all(repo.join(".git")).unwrap();
        std::fs::write(repo.join(".git").join("HEAD"), "ref: refs/heads/dev\n").unwrap();
        assert_eq!(git_branch(&deep), Some((repo.clone(), "dev".to_string())));
        assert_eq!(git_dir(&repo), Some(repo.join(".git")));
        assert_eq!(git_dir(&deep), None, "루트가 아닌 폴더 = 없음");
        // worktree: `.git` 파일이 실제 git 폴더를 가리킨다(상대 경로).
        let wt = base.join("wt");
        std::fs::create_dir_all(&wt).unwrap();
        let real = base.join("gitdirs").join("w1");
        std::fs::create_dir_all(&real).unwrap();
        std::fs::write(real.join("HEAD"), "ref: refs/heads/topic\n").unwrap();
        std::fs::write(wt.join(".git"), "gitdir: ../gitdirs/w1\n").unwrap();
        assert_eq!(git_branch(&wt).map(|r| r.1).as_deref(), Some("topic"));
        std::fs::write(repo.join("f.txt"), "x").unwrap();
        assert_eq!(count_entries(&repo), Some((2, 1)), ".git · a · f.txt");
        assert_eq!(count_entries(&base.join("missing")), None);
        let _ = std::fs::remove_dir_all(&base);
    }
}
