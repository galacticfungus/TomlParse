mod parser;
mod pair;
mod error;

pub struct Parser {
    position: usize,
    state: ParserState,
    line_number: usize, // This is only used for error reporting
    name_start: Option<usize>,
    name_end: Option<usize>,
    value_start: Option<usize>,
    value_end: Option<usize>,
    value_type: ValueType,
}

// Currently only supports strings and integers
#[derive(PartialEq, Debug)]
pub enum TomlValue<'a> {
    String(&'a str),
    Integer(u64),
}

/// Represents the type of value currently being parsed
#[derive(Debug)]
pub enum ValueType {
    Unknown,
    String,
    Integer,
    Float,
}

#[derive(PartialEq, Debug)]
pub struct TomlPair<'a> {
    name: &'a str,
    value: TomlValue<'a>,
}

pub enum ParserState {
    NewLine,        // Parser expects to see a name, whitespace or the end of the file
    ReadingName,    // Parser has started reading a name
    BeforeEquals,   // Parser has read a name and now expects an =
    AfterEquals,    // Parser has seen an = and is now expecting a value of some kind
    ReadingString,  // Parser is reading a basic "hello" string
    ReadingInteger, // Parser is reading an integer or potentially a float
    PairDone, // This state is the final state of reading a pair, parser expects whitespace, new line or end of file
}

#[cfg(test)]
mod test_parser {
    use super::*;

    #[test]
    fn test_string_parse() {
        let toml_string = "junk = \"caveman\"";
        let mut parser = super::Parser::new();
        let pair = parser.read_test_pair(toml_string).unwrap();
        assert_eq!(
            pair,
            Some(TomlPair::new("junk", TomlValue::String("caveman")))
        );
    }

    #[test]
    fn test_string_parse2() {
        let toml_string = "junk = \"caveman\" ";
        let mut parser = super::Parser::new();
        let pair = parser.read_test_pair(toml_string).unwrap();
        assert_eq!(
            pair,
            Some(TomlPair::new("junk", TomlValue::String("caveman")))
        );
    }

    // Tests handling of a file ending in a empty line
    #[test]
    fn test_string_parse3() {
        let toml_string = "junk = \"caveman\"\n";
        let mut parser = super::Parser::new();
        let pair = parser.read_test_pair(toml_string).unwrap();
        assert_eq!(
            pair,
            Some(TomlPair::new("junk", TomlValue::String("caveman")))
        );
        let second_pair = parser.read_test_pair(toml_string).unwrap();
        assert_eq!(second_pair, None);
    }
    #[test]
    fn test_multiline_string_parse() {
        let toml_string = "junk = \"caveman\"\naggro = \"fred\"";
        let mut parser = super::Parser::new();
        let pair = parser.read_test_pair(toml_string).unwrap();
        assert_eq!(
            pair,
            Some(TomlPair::new("junk", TomlValue::String("caveman")))
        );
        let second_pair = parser.read_test_pair(toml_string).unwrap();
        assert_eq!(
            second_pair,
            Some(TomlPair::new("aggro", TomlValue::String("fred")))
        );
    }

    #[test]
    fn test_multiline_string_parse2() {
        let toml_string = "junk = \"caveman\"\n\naggro = \"fred\"";
        let mut parser = super::Parser::new();
        let pair = parser.read_test_pair(toml_string).unwrap();
        assert_eq!(
            pair,
            Some(TomlPair::new("junk", TomlValue::String("caveman")))
        );
        let second_pair = parser.read_test_pair(toml_string).unwrap();
        assert_eq!(
            second_pair,
            Some(TomlPair::new("aggro", TomlValue::String("fred")))
        );
        assert_eq!(parser.read_test_pair(toml_string).unwrap(), None);
    }

    #[test]
    fn test_reading_integer() {
        let toml_string = "junk = 1234";
        let mut parser = super::Parser::new();
        let pair = parser.read_test_pair(toml_string).unwrap();
        assert_eq!(
            pair,
            Some(TomlPair::new("junk", TomlValue::Integer(1234)))
        );
    }
}