// oxycash-rs - state.rs
// AppState: shared application state wrapped in Arc<Mutex<AppState>>.
use crate::model::{Line, Month};
use crate::storage::Storage;
use crate::storage::SyncStatus;

pub struct AppState {
    pub storage:        Storage,
    pub current_month:  String,
    pub sections_open:  [bool; 4],
    pub lines_expanded: [Vec<bool>; 4],
    pub register_asc:   bool,
}

impl AppState {
    pub fn new() -> Self {
        let mut storage = Storage::new();
        let status = storage.load();
        if status == SyncStatus::Dav {
            storage.dav_ok = true;
        }
        let current_month = crate::model::detect_budget_month().to_string();
        Self {
            storage,
            current_month,
            sections_open:  [false; 4],
            lines_expanded: [vec![], vec![], vec![], vec![]],
            register_asc:   false,
        }
    }

    pub fn month(&self) -> Option<&Month> {
        self.storage.data.months.get(&self.current_month)
    }

    pub fn month_mut(&mut self) -> Option<&mut Month> {
        self.storage.data.months.get_mut(&self.current_month)
    }

    pub fn sec_lines(&self, si: usize) -> &Vec<Line> {
        match self.month() {
            Some(m) => match si {
                0 => &m.revenus,
                1 => &m.retraits,
                2 => &m.fixes,
                _ => &m.variables,
            },
            None => panic!("no month"),
        }
    }

    pub fn sec_lines_mut(&mut self, si: usize) -> &mut Vec<Line> {
        let mk = self.current_month.clone();
        let m = self.storage.data.months.get_mut(&mk).unwrap();
        match si {
            0 => &mut m.revenus,
            1 => &mut m.retraits,
            2 => &mut m.fixes,
            _ => &mut m.variables,
        }
    }

    pub fn ensure_expanded(&mut self) {
        for si in 0..4 {
            let n = self.sec_lines(si).len();
            let exp = &mut self.lines_expanded[si];
            exp.resize(n, false);
        }
    }
}
