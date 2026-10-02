//! Nexa Dir — 앱 진입점.
//!
//! M0(골격): 인자 해석 · `--smoke`(창 없음 기동 점검) · `--selfcheck`(자가 점검) 자리를 먼저 세운다.
//! 창·이벤트 루프는 M3(T-40)에서 nexa-sql 골격을 복사해 붙인다(docs/01 · docs/port/40).
//! 규칙: 인자 해석·판정은 순수 함수(docs/port/40 SKEL-440) — `cli.rs` · `selfcheck.rs`에 단위 시험.

mod cli;
mod selfcheck;

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match cli::parse(&args) {
        Err(msg) => {
            eprintln!("nexa-dir: {msg}\n{}", cli::USAGE);
            ExitCode::from(2)
        }
        Ok(cli::Mode::Version) => {
            println!("nexa-dir {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Ok(cli::Mode::Help) => {
            println!("{}", cli::USAGE);
            ExitCode::SUCCESS
        }
        Ok(cli::Mode::Smoke) => run_smoke(),
        Ok(cli::Mode::SelfCheck(opts)) => {
            let report = selfcheck::run(&opts);
            if opts.json {
                println!("{}", report.to_json());
            } else {
                print!("{}", report.to_table());
            }
            ExitCode::from(report.exit_code())
        }
        Ok(cli::Mode::Gui) => {
            // M3(T-40)에서 winit 호스트로 교체. 그 전까지는 창이 없음을 알리고 정상 종료한다.
            eprintln!(
                "nexa-dir {}: GUI는 M3에서 — 지금은 --smoke / --selfcheck / --version",
                env!("CARGO_PKG_VERSION")
            );
            ExitCode::SUCCESS
        }
    }
}

/// `--smoke`: 창 없이 "기동에 필요한 것"을 순서대로 확인하고 0으로 끝난다(CI 3-OS 공통 게이트 · docs/18 §3).
/// = 자가 점검의 `--ci` 부분집합 전체(미구현 그룹은 SKIP) — 항목이 늘면 스모크도 같이 넓어진다.
fn run_smoke() -> ExitCode {
    let report = selfcheck::run(&selfcheck::Options {
        ci: true,
        only: None,
        json: false,
        with_clipboard: false,
    });
    if report.failed() == 0 {
        println!("smoke ok (nexa-dir {})", env!("CARGO_PKG_VERSION"));
        ExitCode::SUCCESS
    } else {
        print!("{}", report.to_table());
        ExitCode::FAILURE
    }
}
