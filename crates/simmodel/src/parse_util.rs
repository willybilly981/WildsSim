pub fn parse_i32(value: &str, default: i32) -> i32 {
    value.trim().parse::<i32>().unwrap_or(default)
}

pub fn parse_i32_default(value: &str) -> i32 {
    parse_i32(value, 0)
}

pub fn parse_bool(value: &str, default: bool) -> bool {
    let trimmed = value.trim();
    if trimmed.eq_ignore_ascii_case("true") {
        true
    } else if trimmed.eq_ignore_ascii_case("false") {
        false
    } else {
        default
    }
}
