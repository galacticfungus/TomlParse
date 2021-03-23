mod error;
mod pair;
mod parser;

pub struct Parser {
    position: usize,
    state: ParserState,
    line_number: usize, // This is only used for error reporting
    name_start: Option<usize>,
    name_end: Option<usize>,
    value_start: Option<usize>,
    value_end: Option<usize>,
}

pub use error::{Error, ErrorKind};

// Currently only supports strings and integers
#[derive(PartialEq, Debug)]
pub enum TomlValue<'a> {
    String(&'a str),
    Integer(u64),
}

#[derive(PartialEq, Debug)]
pub struct TomlPair<'a> {
    name: &'a str,
    value: TomlValue<'a>,
}

pub enum ParserState {
    Normal,         // Parser expects to see a name, whitespace or the end of the file
    ReadingName,    // Parser has started reading a name
    BeforeEquals,   // Parser has read a name and now expects an =
    AfterEquals,    // Parser has seen an = and is now expecting a value of some kind
    ReadingString,  // Parser is reading a basic "hello" string
    ReadingInteger, // Parser is reading an integer or potentially a float or date
    ReadingBoolean,
    AfterValue,
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
        println!("Length is {}", toml_string.len());
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
            Some(TomlPair::new("junk", TomlValue::String("caveman"))),
            "First test failed"
        );
        let second_pair = parser.read_test_pair(toml_string).unwrap();
        assert_eq!(
            second_pair,
            Some(TomlPair::new("aggro", TomlValue::String("fred"))),
            "Second test failed"
        );
        assert_eq!(parser.read_test_pair(toml_string).unwrap(), None);
    }

    #[test]
    fn test_reading_integer() {
        let toml_string = "junk = 1234";
        let mut parser = super::Parser::new();
        let pair = parser.read_test_pair(toml_string).unwrap();
        assert_eq!(pair, Some(TomlPair::new("junk", TomlValue::Integer(1234))));
    }

    #[test]
    fn test_reading_integer2() {
        let toml_string = "junk = 1234\n";
        let mut parser = super::Parser::new();
        let pair = parser.read_test_pair(toml_string).unwrap();
        assert_eq!(pair, Some(TomlPair::new("junk", TomlValue::Integer(1234))));
    }

    #[test]
    fn test_reading_integer3() {
        let toml_string = "junk = 1232344\n  ";
        let mut parser = super::Parser::new();
        let pair = parser.read_test_pair(toml_string).unwrap();
        assert_eq!(
            pair,
            Some(TomlPair::new("junk", TomlValue::Integer(1232344)))
        );
    }

    #[test]
    fn test_reading_integer4() {
        let toml_string = "integer = 123\n  number=321\nword=\"abc\"";
        let mut parser = super::Parser::new();
        let pair = parser.read_test_pair(toml_string).unwrap();
        assert_eq!(
            pair,
            Some(TomlPair::new("integer", TomlValue::Integer(123))),
            "Failed first test"
        );
        let pair = parser.read_test_pair(toml_string).unwrap();
        assert_eq!(
            pair,
            Some(TomlPair::new("number", TomlValue::Integer(321))),
            "Failed second test"
        );
        let pair = parser.read_test_pair(toml_string).unwrap();
        assert_eq!(
            pair,
            Some(TomlPair::new("word", TomlValue::String("abc"))),
            "Failed third test"
        );
    }
}
