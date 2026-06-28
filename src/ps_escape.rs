/// Escape a string for safe insertion into a PowerShell single-quoted string.
/// In PowerShell single-quoted strings, the only escape sequence is '' (two
/// single quotes) to represent a literal single quote.
pub fn escape_ps_string(s: &str) -> String {
    s.replace('\'', "''")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_special_chars() {
        assert_eq!(escape_ps_string("hello"), "hello");
    }

    #[test]
    fn escapes_single_quote() {
        assert_eq!(escape_ps_string("it's"), "it''s");
    }

    #[test]
    fn escapes_multiple_quotes() {
        assert_eq!(escape_ps_string("'a'b'"), "''a''b''");
    }
}
