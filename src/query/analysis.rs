use std::fmt::{Debug, Display, Formatter};
use std::str::FromStr;
use std::time::Duration;

/// Provides analysis data about a [`Query`].
///
/// # Example
/// ```
/// # use basex::analysis::Info;
/// # fn example(info: impl Info) {
/// println!("Parsing: {:?}", info.parsing_time());
/// println!("Compiling: {:?}", info.compiling_time());
/// println!("Evaluating: {:?}", info.evaluating_time());
/// println!("Printing: {:?}", info.printing_time());
/// println!("Total Time: {:?}", info.total_time());
/// println!("Hit(s): {:?}", info.hits());
/// println!("Updated: {:?}", info.updated());
/// println!("Printed: {:?}", info.printed());
/// println!("Read Locking: {:?}", info.read_locking());
/// println!("Write Locking: {:?}", info.write_locking());
/// println!("Optimized Query: {:?}", info.optimized_query());
/// println!("Query: {:?}", info.query());
/// println!("Compiling: {:?}", info.compiling());
/// # }
/// ```
///
/// [`Query`]: crate::Query
pub trait Info: Debug + Display + Clone + PartialEq {
    /// Time it took to parse the query.
    fn parsing_time(&self) -> Duration;

    /// Time it took to compile the query.
    fn compiling_time(&self) -> Duration;

    /// Time it took to evaluate.
    fn evaluating_time(&self) -> Duration;

    /// Time it took to print the info.
    fn printing_time(&self) -> Duration;

    /// Total time it took to analyse the query.
    fn total_time(&self) -> Duration;

    /// Nodes hit.
    fn hits(&self) -> usize;

    /// Nodes updated.
    fn updated(&self) -> usize;

    /// Bytes printed for the query analysis.
    fn printed(&self) -> usize;

    /// Specifies the database that's going to be locked for reading by running this query, if there is any.
    fn read_locking(&self) -> Option<String>;

    /// Specifies the database that's going to be locked for writing by running this query, if there is any.
    fn write_locking(&self) -> Option<String>;

    /// The optimized XQuery after compilation.
    fn optimized_query(&self) -> String;

    /// The input XQuery.
    fn query(&self) -> String;

    /// Compilation steps to parse XQuery and produce an optimized version.
    fn compiling(&self) -> Vec<String>;
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RawInfo {
    raw: String,
}

impl Display for RawInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.raw)
    }
}

impl RawInfo {
    pub fn new(raw: String) -> Self {
        Self { raw }
    }

    fn duration_from_str(duration: &str) -> Duration {
        let v: Vec<&str> = duration.splitn(2, ' ').collect();
        let (time, unit) = (v[0], v[1]);
        let unit: String = unit.chars().take_while(|c| c.is_alphabetic()).collect();
        let time = f64::from_str(time).unwrap();

        match unit.as_str() {
            "s" => Duration::from_secs_f64(time),
            "ms" => Duration::from_nanos((time * 1000000.0) as u64),
            other => panic!("Unexpected unit: {}", other),
        }
    }

    fn string_from(&self, header: &str) -> String {
        let start = self.raw.find(header).unwrap() + header.len();
        let stop = self.raw[start..].find('\n').unwrap();
        self.raw[start..start + stop].to_owned()
    }

    fn option_string_from(&self, header: &str) -> Option<String> {
        let str = self.string_from(header);
        match str.as_str() {
            "(none)" => None,
            _ => Some(str),
        }
    }

    fn duration_from(&self, header: &str) -> Duration {
        RawInfo::duration_from_str(&self.string_from(header))
    }

    fn usize_from(&self, header: &str) -> usize {
        let s: String = self
            .string_from(header)
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();

        usize::from_str(&s).unwrap()
    }
}

impl Info for RawInfo {
    fn parsing_time(&self) -> Duration {
        self.duration_from("Parsing: ")
    }

    fn compiling_time(&self) -> Duration {
        self.duration_from("Compiling: ")
    }

    fn evaluating_time(&self) -> Duration {
        self.duration_from("Evaluating: ")
    }

    fn printing_time(&self) -> Duration {
        self.duration_from("Printing: ")
    }

    fn total_time(&self) -> Duration {
        self.duration_from("Total Time: ")
    }

    fn hits(&self) -> usize {
        self.usize_from("Hit(s): ")
    }

    fn updated(&self) -> usize {
        self.usize_from("Updated: ")
    }

    fn printed(&self) -> usize {
        self.usize_from("Printed: ")
    }

    fn read_locking(&self) -> Option<String> {
        self.option_string_from("Read Locking: ")
    }

    fn write_locking(&self) -> Option<String> {
        self.option_string_from("Write Locking: ")
    }

    fn optimized_query(&self) -> String {
        self.string_from("Optimized Query:\n")
    }

    fn query(&self) -> String {
        self.string_from("Query:\n")
    }

    fn compiling(&self) -> Vec<String> {
        let header = "Compiling:\n- ";
        let start = self.raw.find(header).unwrap() + header.len();
        let stop = self.raw[start..].find("\n\n").unwrap();
        self.raw[start..start + stop]
            .to_owned()
            .split("\n- ")
            .map(|v| v.to_owned())
            .collect()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    pub static QUERY_INFO: &str = r#"
Query:
count(/None/*)

Compiling:
- rewrite context value to document-node() item: . -> db:open-pre("d601a46", 0)
- rewrite util:root(nodes) to document-node() item: util:root(db:open-pre("d601a46", 0)) -> db:open-pre("d601a46", 0)
- rewrite fn:count(items) to xs:integer item: count(db:open-pre("d601a46", 0)/None/*) -> 3

Optimized Query:
3

Parsing: 381.41 ms
Compiling: 12.22 ms
Evaluating: 0.09 ms
Printing: 4.79 ms
Total Time: 398.5 ms

Hit(s): 1 Item
Updated: 0 Items
Printed: 1 b
Read Locking: d601a46
Write Locking: (none)

Query executed in 398.5 ms.
"#;

    #[macro_export]
    macro_rules! assert_query_info {
        ($info:expr) => {
            use std::time::Duration;
            let info = $info;
            assert_eq!("count(/None/*)", info.query());
            assert_eq!("3", info.optimized_query());
            assert_eq!(Duration::from_micros(381410), info.parsing_time());
            assert_eq!(Duration::from_micros(12220), info.compiling_time());
            assert_eq!(Duration::from_micros(0090), info.evaluating_time());
            assert_eq!(Duration::from_micros(4790), info.printing_time());
            assert_eq!(Duration::from_micros(398500), info.total_time());
            assert_eq!(
                vec![
                    "rewrite context value to document-node() item: \
                . -> db:open-pre(\"d601a46\", 0)",
                    "rewrite util:root(nodes) to document-node() item: \
                util:root(db:open-pre(\"d601a46\", 0)) -> db:open-pre(\"d601a46\", 0)",
                    "rewrite fn:count(items) to xs:integer item: \
                count(db:open-pre(\"d601a46\", 0)/None/*) -> 3",
                ],
                info.compiling()
            );
            assert_eq!(1, info.hits());
            assert_eq!(0, info.updated());
            assert_eq!(1, info.printed());
            assert_eq!(Some("d601a46"), info.read_locking().as_ref().map(|v| v.as_str()));
            assert_eq!(None, info.write_locking());
        };
    }

    pub use assert_query_info;

    #[test]
    fn test_parses_with_correct_values() {
        let raw = QUERY_INFO;
        let info = RawInfo::new(raw.to_owned());
        assert_query_info!(info);
    }

    #[test]
    fn test_formats_as_debug() {
        format!("{:?}", RawInfo::new(QUERY_INFO.to_owned()));
    }

    #[test]
    fn test_formats_as_display() {
        format!("{}", RawInfo::new(QUERY_INFO.to_owned()));
    }

    #[test]
    fn test_can_eq() {
        assert_eq!(RawInfo::new(QUERY_INFO.to_owned()), RawInfo::new(QUERY_INFO.to_owned()));
    }

    #[test]
    fn test_clones() {
        let _ = RawInfo::new(QUERY_INFO.to_owned()).clone();
    }

    #[test]
    #[should_panic]
    fn test_duration_from_str_panics_on_invalid_unit() {
        RawInfo::duration_from_str("69 mss.");
    }
}
