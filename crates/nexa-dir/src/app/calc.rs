//! 계산 컬럼 호스트(NEW-060 · T-213 · 사용자 11-14): 열 머리글 우클릭 메뉴 → 정의 창(`이름 = 식`) → 활성 탭에 추가 · 편집 · 삭제.
//! 정의 · 평가 = [`crate::calccol`] · 탭 보관 · 열/정렬/세션 = [`crate::panel::Panel`].

use crate::calccol::{CalcCol, DefError, CALC_MAX};
use crate::*;

impl App {
    /// 정의 창을 연다 — `at` = 편집할 자리(없으면 새 열) · `initial` = 입력란에 채울 글(오류 뒤 다시 묻기).
    pub(crate) fn calc_ask(&mut self, at: Option<usize>, initial: Option<String>) {
        let cols = self.panels[self.active].calc_cols();
        if at.is_none() && cols.len() >= CALC_MAX {
            self.toasts.push(
                toast::ToastKind::Warn,
                tr("calc.dlg.title"),
                trf("calc.full", &[&CALC_MAX.to_string()]),
            );
            self.redraw();
            return;
        }
        let initial = initial
            .or_else(|| at.and_then(|i| cols.get(i)).map(CalcCol::def_text))
            .unwrap_or_default();
        // 두 칸(이름 · 식) + 미리보기 줄 — 결과 글은 종전 한 줄 입력과 같은 `이름 = 식`.
        let (name, expr) = initial.split_once('=').unwrap_or((initial.as_str(), ""));
        let mut spec = crate::dlg_win::DlgSpec::confirm(
            tr("calc.dlg.title"),
            tr("calc.dlg.text"),
            tr("dlg.ok"),
        );
        spec.input = Some(crate::dlg_win::DlgInput {
            label: tr("calc.dlg.name"),
            masked: false,
            initial: name.trim().to_owned(),
            placeholder: tr("calc.dlg.namePlaceholder"),
        });
        spec.more = Some(crate::dlg_win::DlgMore {
            input: crate::dlg_win::DlgInput {
                label: tr("calc.dlg.input"),
                masked: false,
                initial: expr.trim().to_owned(),
                placeholder: tr("calc.dlg.placeholder"),
            },
            join: " = ".into(),
        });
        let _ = self.ask(spec, crate::app::dialogs::DlgReply::CalcColumn(at));
    }

    /// 정의 창의 미리보기 줄(유휴 틱 · `dlg.type` 뒤): 입력이 바뀌었으면 지금 글을 열로 풀어 **첫 선택 행(없으면 첫 행)**의 값을
    /// 보여 준다 — 틀리면 사유(위험색). 계산 컬럼 창이 아니면 무동작.
    pub(crate) fn calc_hint_tick(&mut self) {
        if !self.dlg.take_edited()
            || !matches!(
                self.dlg_reply,
                Some(crate::app::dialogs::DlgReply::CalcColumn(_))
            )
        {
            return;
        }
        let text = self.dlg.input_text().unwrap_or_default();
        let (name, expr) = text.split_once('=').unwrap_or((text.as_str(), ""));
        let (hint, error) = if expr.trim().is_empty() {
            (String::new(), false) // 아직 식을 안 썼다 — 조용히
        } else {
            // 이름을 아직 안 썼어도 식은 미리 본다(이름 오류는 확인 때 알린다).
            let name = if name.trim().is_empty() { "_" } else { name };
            match CalcCol::new(name, expr) {
                Err(DefError::Expr(m)) => (trf("calc.err.expr", &[&m]), true),
                Err(DefError::Name) => (tr("calc.err.name"), true),
                Err(DefError::Shape) => (tr("calc.err.shape"), true),
                Ok(col) => match self.panels[self.active].calc_preview(&col) {
                    Some((row, Ok(v))) => (trf("calc.hint.value", &[&row, &v]), false),
                    Some((row, Err(m))) => (trf("calc.hint.rowError", &[&row, &m]), true),
                    None => (tr("calc.hint.ok"), false),
                },
            }
        };
        self.dlg.set_hint(hint, error);
    }

    /// 정의 창 확인 — 입력을 열로 풀어 활성 탭에 추가/교체한다. 틀리면 사유를 알리고 같은 글로 다시 묻는다.
    pub(crate) fn calc_entered(&mut self, at: Option<usize>, text: &str) -> bool {
        let a = self.active;
        let mut col = match CalcCol::parse_def(text) {
            Ok(c) => c,
            Err(e) => {
                let why = match e {
                    DefError::Shape => tr("calc.err.shape"),
                    DefError::Name => tr("calc.err.name"),
                    DefError::Expr(m) => trf("calc.err.expr", &[&m]),
                };
                self.toasts
                    .push(toast::ToastKind::Warn, tr("calc.dlg.title"), why);
                self.calc_ask(at, Some(text.to_owned()));
                self.redraw();
                return false;
            }
        };
        let mut cols = self.panels[a].calc_cols().to_vec();
        match at.filter(|i| *i < cols.len()) {
            Some(i) => {
                col.width = cols[i].width;
                cols[i] = col;
            }
            None if cols.len() < CALC_MAX => cols.push(col),
            None => return false,
        }
        let mut inv = Invalidations::default();
        self.panels[a].set_calc(cols, &mut inv);
        self.redraw();
        true
    }

    /// 활성 탭의 계산 컬럼 하나 삭제.
    pub(crate) fn calc_delete(&mut self, slot: usize) {
        let mut inv = Invalidations::default();
        self.panels[self.active].calc_remove(slot, &mut inv);
        self.redraw();
    }

    /// 열 머리글 메뉴에 붙일 계산 컬럼 항목: 추가… · (있는 열마다) 편집… · 삭제.
    pub(crate) fn calc_menu_items(&self) -> Vec<CtxItem> {
        let mut items = vec![CtxItem::item("aux.calc.add", tr("calc.menu.add"))];
        for (i, c) in self.panels[self.active].calc_cols().iter().enumerate() {
            items.push(CtxItem::item(
                format!("aux.calc.edit:{i}"),
                trf("calc.menu.edit", &[&c.name]),
            ));
            items.push(CtxItem::item(
                format!("aux.calc.del:{i}"),
                trf("calc.menu.del", &[&c.name]),
            ));
        }
        items
    }

    /// 보조 메뉴 항목 실행(`aux.calc.*`) — 처리했으면 참.
    pub(crate) fn calc_menu_action(&mut self, id: &str) -> bool {
        if id == "aux.calc.add" {
            self.calc_ask(None, None);
            return true;
        }
        let slot = |rest: &str| rest.parse::<usize>().ok();
        if let Some(i) = id.strip_prefix("aux.calc.edit:").and_then(slot) {
            self.calc_ask(Some(i), None);
            return true;
        }
        if let Some(i) = id.strip_prefix("aux.calc.del:").and_then(slot) {
            self.calc_delete(i);
            return true;
        }
        false
    }

    /// 두 패널의 계산 컬럼 동기([`crate::panel::Panel::calc_sync`]) — 바뀐 목록 사각형은 `inv`에.
    pub(crate) fn calc_sync(&mut self, inv: &mut Invalidations) {
        for p in &mut self.panels {
            p.calc_sync(inv);
        }
    }
}
