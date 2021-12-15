use std::str::FromStr;

/// The error type for failed query execution.
#[derive(Debug)]
pub struct QueryFailed {
    raw: String,
    code: String,
    line: usize,
    position: usize,
    message: String,
    file: String,
}

impl QueryFailed {
    pub(crate) fn new(raw: String) -> Self {
        let code_index = raw.find('[').unwrap();
        let code_stop = code_index + raw[code_index..].find(']').unwrap();
        let code = raw[code_index + 1..code_stop].to_owned();

        let line_separator = raw[..code_index].rfind('/').unwrap();
        let line_start = raw[..line_separator].rfind(',').unwrap();
        let line_stop = line_separator + raw[line_separator..].find(':').unwrap();
        let line = &raw[line_start + 2..line_separator];
        let line = usize::from_str(line).unwrap();

        let position = &raw[line_separator + 1..line_stop];
        let position = usize::from_str(position).unwrap();

        let message = raw[code_stop + 2..].to_owned();
        let file = raw[11..line_start].to_owned();

        Self {
            raw,
            code,
            line,
            position,
            message,
            file,
        }
    }

    /// The unparsed error string.
    pub fn raw(&self) -> &str {
        &self.raw
    }

    /// The XQuery [error code](https://docs.basex.org/wiki/XQuery_Errors).
    pub fn code(&self) -> &str {
        &self.code
    }

    /// The line in the file where the error occurred.
    pub fn line(&self) -> usize {
        self.line
    }

    /// The character position in the line where the error occurred.
    pub fn position(&self) -> usize {
        self.position
    }

    /// The error message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// The XQuery source file. Is presented as `.` (dot character) when not from file.
    pub fn file(&self) -> &str {
        &self.file
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parsing_errors() {
        let dataset = [
            (
                "Stopped at ., 1/2264: [XPST0003] Expecting ']', found '&'. Error in parse(text = x) : attempt to use \
                 zero-length variable name",
                "XPST0003",
                "Expecting ']', found '&'. Error in parse(text = x) : attempt to use zero-length variable name",
                ".",
                1,
                2264,
            ),
            (
                "Stopped at C:/Program Files (x86)/BaseX/etc/file2, 9/6: [XPST0003] Expecting '}', found '{'.",
                "XPST0003",
                "Expecting '}', found '{'.",
                "C:/Program Files (x86)/BaseX/etc/file2",
                9,
                6,
            ),
            (
                "Stopped at ., 1/87: [bxerr:BASX0000] java.lang.StringIndexOutOfBoundsException: String index out of \
                range: -1 Error in parse(text = x) : attempt to use zero-length variable name",
                "bxerr:BASX0000",
                "java.lang.StringIndexOutOfBoundsException: String index out of range: -1 Error in parse(text = x) : \
                attempt to use zero-length variable name",
                ".",
                1,
                87,
            ),
        ];

        for (expected_raw, expected_code, expected_message, expected_file, expected_line, expected_position) in dataset
        {
            let error = QueryFailed::new(expected_raw.to_owned());

            assert_eq!(expected_raw, error.raw());
            assert_eq!(expected_code, error.code());
            assert_eq!(expected_message, error.message());
            assert_eq!(expected_file, error.file());
            assert_eq!(expected_line, error.line());
            assert_eq!(expected_position, error.position());
        }
    }
}
