use std::collections::HashMap;

/// Parser tham số đơn giản: chấp nhận cả `-key value` lẫn `--key value`.
pub struct Args {
    map: HashMap<String, String>,
}

impl Args {
    pub fn parse<I: Iterator<Item = String>>(iter: I) -> Self {
        let mut map = HashMap::new();
        let mut key: Option<String> = None;

        for token in iter {
            if let Some(stripped) = strip_flag(&token) {
                // flag mới; flag trước đó không có value thì lưu rỗng
                if let Some(k) = key.take() {
                    map.insert(k, String::new());
                }
                key = Some(stripped.to_string());
            } else if let Some(k) = key.take() {
                map.insert(k, token);
            }
        }
        if let Some(k) = key {
            map.insert(k, String::new());
        }

        Args { map }
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.map.get(key).cloned()
    }

    /// Lấy tham số bắt buộc, báo lỗi rõ ràng nếu thiếu.
    pub fn require(&self, key: &str) -> Result<String, String> {
        match self.map.get(key) {
            Some(v) if !v.is_empty() => Ok(v.clone()),
            _ => Err(format!("Thiếu tham số bắt buộc: -{}", key)),
        }
    }
}

fn strip_flag(token: &str) -> Option<&str> {
    let s = token.strip_prefix("--").or_else(|| token.strip_prefix('-'))?;
    // tránh nhầm số âm (vd: -5) thành flag
    if s.is_empty() || s.chars().next().unwrap().is_ascii_digit() {
        return None;
    }
    Some(s)
}
