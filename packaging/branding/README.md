# branding — Nexa Dir 아이콘 원천(dir2 `crates/nexa-app/assets` 그대로)

- `nexa-dir-1024.png`(원본 · dir2) → `png/nexa-dir-{16,24,32,48,64,128,256,512,1024}.png`(Linux hicolor · macOS iconutil 입력 · PIL LANCZOS).
- `nexa-dir.ico` · `nexa-dir-light.ico`(dir2) — Windows exe 리소스(`packaging/windows/nexa-dir.rc`) · MSI ARP 아이콘.
- `.icns`는 macOS 러너의 `sips`+`iconutil`이 build-app.sh에서 즉석 생성(없으면 `nexa-dir.icns` 폴백 자리).
- 재생성: `python - <<'EOF'`로 PIL 리사이즈(porting 스크립트 `port_packaging.py` 참조) — 원본만 SSOT.
