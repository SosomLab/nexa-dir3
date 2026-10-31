//! nexa-copy — 고속 복사 **명령행 실행파일**(NEW-007 제공 형태 "bin `nexa-copy`" · 사용자 10-24 "별도 실행파일로 제공해서 cli로").
//!
//! 엔진은 앱과 **같은 길**(`ndir_ops::transfer` + `fastcopy` 전략 계층 · docs/24) — 같은 폴더 규칙 · 충돌 정책 · 스테이징 교체 · 교차 볼륨
//! 이동 · 항목별 실패 격리 · OS 복사 경로(Windows `CopyFileExW` · Linux `FICLONE`/`copy_file_range` · macOS `copyfile`) · 작은 파일 병렬.
//! 창 · 설정 파일 · 라이선스 없음(전 기능 무료 · DR-4) · 외부 crate 0. 인자 해석과 실행은 [`cli`](순수 함수 + 시험).

mod cli;
mod ctrlc;
mod mirror;

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut err = std::io::stderr().lock();
    cli::main_with(&args, &mut err)
}
