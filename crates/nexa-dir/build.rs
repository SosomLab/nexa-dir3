//! build.rs — GUI `nexa-dir.exe`의 아이콘·버전 리소스(Windows). 본체 = `packaging/windows/winres.rs`(nexa-sql T-62 복사 · docs/port/40 SKEL-405).
//! 다른 OS·rc 도구 없음 = 조용히 건너뛴다(아이콘 없는 exe · 기능 동일).
include!("../../packaging/windows/winres.rs");

fn main() {
    embed_windows_resources("nexa-dir.rc", "nexa-dir");
}
