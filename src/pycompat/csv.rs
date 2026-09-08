//! `csv.writer` excel dialect: fields joined by `,`, quoted with `"`
//! (doubled inside) only when containing the delimiter, a quote, `\r` or
//! `\n`; rows terminated by `\r\n`.

/// Render one CSV row (excel dialect, `QUOTE_MINIMAL`).
pub fn write_row(fields: &[&str]) -> String {
    let mut out = String::new();
    for (i, field) in fields.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        if field.contains(',')
            || field.contains('"')
            || field.contains('\r')
            || field.contains('\n')
        {
            out.push('"');
            out.push_str(&field.replace('"', "\"\""));
            out.push('"');
        } else {
            out.push_str(field);
        }
    }
    out.push_str("\r\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_only_when_needed() {
        assert_eq!(
            write_row(&["a", "b,c", "d\"e", "f\ng"]),
            "a,\"b,c\",\"d\"\"e\",\"f\ng\"\r\n"
        );
        assert_eq!(write_row(&["plain", "text"]), "plain,text\r\n");
    }
}
