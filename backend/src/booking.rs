//! 可接案判斷純函數：例外 > 週範本 > 無規則即公休。
//! 與 DB 無關，可單元測試；routes 只負責把 rows 轉成 views 餵進來。
//!
//! 人數不進系統：滿了店家手動勾「當日額滿」，不營業勾「不營業」。

use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};

/// 一個時段＝(起點，終點)，賣家自填、可任意多個
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Window {
    pub start: String, // HH:MM
    pub end: String,   // HH:MM
}

impl Window {
    pub fn label(&self) -> String {
        if self.end.is_empty() {
            self.start.clone()
        } else {
            format!("{}-{}", self.start, self.end)
        }
    }
}

/// 輸入時只求有填：起終點純文字自由填（如 "09:00"、"中午"、"晚上" 都可）
pub fn valid_window(start: &str, end: &str) -> bool {
    !start.is_empty() && !end.is_empty() && start.len() <= 20 && end.len() <= 20
}

/// 讀取時寬容：新格式 [{start,end}] 照收；舊格式 ["09:00-12:00"] 轉成對
///（舊庫升級過渡用，PUT 一律只寫新格式）
pub fn parse_windows(v: &serde_json::Value) -> Vec<Window> {
    let arr = match v.as_array() {
        Some(a) => a,
        None => return vec![],
    };
    arr.iter()
        .filter_map(|x| {
            if let (Some(s), Some(e)) = (
                x.get("start").and_then(|s| s.as_str()),
                x.get("end").and_then(|s| s.as_str()),
            ) {
                Some(Window {
                    start: s.to_string(),
                    end: e.to_string(),
                })
            } else if let Some(s) = x.as_str() {
                let mut it = s.splitn(2, '-');
                match (it.next(), it.next()) {
                    (Some(a), Some(b)) => Some(Window {
                        start: a.to_string(),
                        end: b.to_string(),
                    }),
                    _ => None,
                }
            } else {
                None
            }
        })
        .collect()
}

pub struct RuleView {
    pub weekday: i32,
    pub open: bool,
    pub windows: Vec<Window>,
}

pub struct ExcView {
    pub date: String,
    pub open: bool,
    pub full: bool,
}

pub struct DayPlan {
    pub open: bool,
    pub full: bool,
    pub windows: Vec<Window>,
}

/// 嚴格 YYYY-MM-DD，非法回 None
pub fn parse_date(s: &str) -> Option<NaiveDate> {
    if s.len() != 10 {
        return None;
    }
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}

/// 0=週日 .. 6=週六
pub fn weekday_of(d: NaiveDate) -> i32 {
    d.weekday().num_days_from_sunday() as i32
}

/// 判斷某日是否可接。date 無效回 None。
/// 可約 ⇔ open 且非 full（人數不看，滿了店家自己勾）。
pub fn resolve(rules: &[RuleView], exceptions: &[ExcView], date: &str) -> Option<DayPlan> {
    let d = parse_date(date)?;
    if let Some(e) = exceptions.iter().find(|e| e.date == date) {
        return Some(DayPlan {
            open: e.open && !e.full,
            full: e.full,
            windows: vec![],
        });
    }
    let wd = weekday_of(d);
    match rules.iter().find(|r| r.weekday == wd) {
        Some(r) => Some(DayPlan {
            open: r.open,
            full: false,
            windows: r.windows.clone(),
        }),
        None => Some(DayPlan {
            open: false,
            full: false,
            windows: vec![],
        }),
    }
}

/// YYYY-MM；非法回 None
pub fn parse_month(s: &str) -> Option<(i32, u32)> {
    if s.len() != 7 {
        return None;
    }
    let y: i32 = s[..4].parse().ok()?;
    let m: u32 = s[5..7].parse().ok()?;
    if s.as_bytes()[4] != b'-' || !(1..=12).contains(&m) || !(2000..=2100).contains(&y) {
        return None;
    }
    Some((y, m))
}

/// 該月所有日期字串（閏年由 chrono 處理）
pub fn month_dates(year: i32, month: u32) -> Vec<String> {
    let mut out = vec![];
    let mut d = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    loop {
        if d.month() != month {
            break;
        }
        out.push(d.format("%Y-%m-%d").to_string());
        d = d.succ_opt().unwrap();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(s: &str, e: &str) -> Window {
        Window {
            start: s.into(),
            end: e.into(),
        }
    }

    fn rules() -> Vec<RuleView> {
        vec![
            RuleView {
                weekday: 2, // 週二開
                open: true,
                windows: vec![w("09:00", "12:00"), w("14:00", "18:00")],
            },
            RuleView {
                weekday: 0, // 週日關
                open: false,
                windows: vec![],
            },
        ]
    }

    #[test]
    fn weekday_matches_chrono() {
        // 2026-10-06 是週二
        let d = parse_date("2026-10-06").unwrap();
        assert_eq!(d.weekday(), chrono::Weekday::Tue);
        assert_eq!(weekday_of(d), 2);
    }

    #[test]
    fn bad_date_rejected() {
        assert!(parse_date("2026-13-01").is_none());
        assert!(parse_date("2026-10-6").is_none());
        assert!(parse_date("10/06/2026").is_none());
        assert!(parse_date("").is_none());
        assert!(resolve(&rules(), &[], "亂填").is_none());
    }

    #[test]
    fn window_valid_and_legacy_parse() {
        assert!(valid_window("09:00", "12:00"));
        assert!(valid_window("早上九點", "中午")); // 純文字自由填
        assert!(!valid_window("", "12:00"));
        assert!(!valid_window("09:00", ""));
        // 新格式
        let v = serde_json::json!([{"start": "09:00", "end": "12:00"}]);
        assert_eq!(parse_windows(&v), vec![w("09:00", "12:00")]);
        // 舊格式字串相容
        let v = serde_json::json!(["09:00-12:00"]);
        assert_eq!(parse_windows(&v), vec![w("09:00", "12:00")]);
        assert_eq!(w("09:00", "12:00").label(), "09:00-12:00");
    }

    #[test]
    fn rule_open_and_closed() {
        // 2026-10-06 週二開
        let p = resolve(&rules(), &[], "2026-10-06").unwrap();
        assert!(p.open && !p.full);
        assert_eq!(p.windows.len(), 2);
        // 2026-10-04 週日關
        let p = resolve(&rules(), &[], "2026-10-04").unwrap();
        assert!(!p.open);
        // 2026-10-07 週三無規則＝公休
        let p = resolve(&rules(), &[], "2026-10-07").unwrap();
        assert!(!p.open);
    }

    #[test]
    fn exception_wins_over_rule() {
        // 例外關（週二本來開）
        let exc = vec![ExcView {
            date: "2026-10-06".into(),
            open: false,
            full: false,
        }];
        let p = resolve(&rules(), &exc, "2026-10-06").unwrap();
        assert!(!p.open);
        // 例外手動額滿：open 也視為不可約
        let exc = vec![ExcView {
            date: "2026-10-06".into(),
            open: true,
            full: true,
        }];
        let p = resolve(&rules(), &exc, "2026-10-06").unwrap();
        assert!(!p.open && p.full);
        // 例外開（週三本來公休）
        let exc = vec![ExcView {
            date: "2026-10-07".into(),
            open: true,
            full: false,
        }];
        let p = resolve(&rules(), &exc, "2026-10-07").unwrap();
        assert!(p.open && !p.full);
    }

    #[test]
    fn month_helpers() {
        assert_eq!(parse_month("2026-10"), Some((2026, 10)));
        assert!(parse_month("2026-13").is_none());
        assert!(parse_month("26-10").is_none());
        let ds = month_dates(2026, 2);
        assert_eq!(ds.len(), 28);
        assert_eq!(ds[0], "2026-02-01");
        let ds = month_dates(2024, 2);
        assert_eq!(ds.len(), 29); // 閏年
    }
}
