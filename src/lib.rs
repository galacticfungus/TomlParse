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

// Currently only supports strings and integers
#[derive(PartialEq, Debug)]
pub enum TomlValue<'a> {
    String(&'a str), // TODO: Strings are subject to more restrictions than a UTF8 string
    Integer(u64),
    Float(f64),
}

// TODO: A TOML name can be a name or a String
#[derive(PartialEq, Debug)]
pub struct TomlPair<'a> {
    name: &'a str, // TODO: Two types of names, normal and string
    value: TomlValue<'a>,
}

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    source: Option<Box<dyn std::error::Error>>,
}

#[derive(Debug)]
pub enum ErrorKind {
    /// There are only two valid line endings \n and \r\n
    InvalidEndOfLine(usize),
    /// No Value was found on the given line - ie 'fred =' or 'fred'
    MissingValue(usize),
    /// Unrecognizable value type ie 'fred = ..! ' this is not a known value type
    UnknownValueType(usize),
    /// Invalid Value ie including letters in a number, not ending a string with a "
    InvalidValue(usize),
    /// Name contains invalid characters, ie fred\n = 4 or fred \n = 4
    InvalidName(usize),
}

pub enum ParserState {
    Normal,         // Parser expects to see a name, whitespace or the end of the file
    ReadingName,    // Parser has started reading a name
    BeforeEquals,   // Parser has read a name and now expects an =
    AfterEquals,    // Parser has seen an = and is now expecting a value of some kind
    ReadingString,  // Parser is reading a basic "hello" string
    ReadingInteger, // Parser is reading an integer or potentially a float or date
    ReadingBoolean, // Parser is reading a boolean
    AfterValue, // Parser has finished reading a value and is waiting until it sees the next line
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
