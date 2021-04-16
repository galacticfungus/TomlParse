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
    Integer(i64),
    Float(f64),
    Bool(bool),
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

#[derive(Debug, Clone, Copy, PartialEq)]
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
    /// Parser expects to see a name, whitespace or the end of the file
    Normal,
    /// Parser has started reading a name
    ReadingName,
    /// Parser has read a name and now expects an =
    BeforeEquals,
    /// Parser has seen an = and is now expecting a value of some kind
    AfterEquals,
    /// Parser is reading a basic "hello" string
    ReadingString,
    /// Parser is reading an integer or potentially a float or date, if it is an integer then it is base 10
    ReadingInteger,
    // TODO: Change this so that it is passed as state rather than a totally seperate state
    ReadingNegativeInteger,
    FinishedNegativeInteger(
        /// End of file Reached
        bool,
        /// Finished on a new line
        bool,
    ),
    ReadingOctalInteger,
    ReadingHexInteger,
    ReadingBinaryInteger,
    /// Parser is reading a float
    ReadingFloat(
        /// Have we seen the exponential
        bool,
    ),
    /// Parser is reading a boolean
    ReadingTrue(usize),
    /// Parser is reading a false boolean
    ReadingFalse(usize),
    /// Parser has finished reading a value and is waiting until it sees a next line
    AfterValue,
    /// Parser has finished reading a String value
    FinishedString,
    /// Parser has finished reading a float value
    FinishedFloat(bool, bool),
    /// Parser has finished reading a integer value
    FinishedInteger(
        /// End of file Reached
        bool,
        /// Finished on a new line
        bool,
    ),
    /// Parser finished reading a binary integer value
    FinishedBinaryInteger(
        /// End of file reached
        bool,
        /// Finished on a new line
        bool,
    ),
    /// Parser finished reading a binary integer value
    FinishedHexInteger(
        /// End of file reached
        bool,
        /// Finished on a new line
        bool,
    ),
    /// Parser finished reading a binary integer value
    FinishedOctalInteger(
        /// End of file reached
        bool,
        /// Finished on a new line
        bool,
    ),
    FinishedBoolean(bool), // Parser has finished reading a boolean value
    EndOfFile,             // Parser has reached the end of a file
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
        let toml_string = "integer = 123\r\n  number=321\n";
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
    }

    #[test]
    fn test_reading_integer5() {
        let toml_string = "integer = 123 \n";
        let mut parser = super::Parser::new();
        let pair = parser.read_test_pair(toml_string).unwrap();
        assert_eq!(
            pair,
            Some(TomlPair::new("integer", TomlValue::Integer(123))),
            "Failed first test"
        );
    }

    #[test]
    fn test_reading_integer6() {
        let toml_string = "junk = 1_234";
        let mut parser = super::Parser::new();
        let pair = parser.read_test_pair(toml_string).unwrap();
        assert_eq!(pair, Some(TomlPair::new("junk", TomlValue::Integer(1234))));
    }

    #[test]
    fn test_reading_integer7() {
        let toml_string = "junk = 0";
        let mut parser = super::Parser::new();
        let pair = parser.read_test_pair(toml_string).unwrap();
        assert_eq!(pair, Some(TomlPair::new("junk", TomlValue::Integer(0))));
    }

    #[test]
    fn test_reading_integer8() {
        let toml_string = "junk = 0\r\n";
        let mut parser = super::Parser::new();
        let pair = parser.read_test_pair(toml_string).unwrap();
        assert_eq!(pair, Some(TomlPair::new("junk", TomlValue::Integer(0))));
    }

    #[test]
    fn test_reading_signed_integer() {
        let toml_string = "junk = +0";
        let mut parser = super::Parser::new();
        let pair = parser.read_test_pair(toml_string).unwrap();
        assert_eq!(pair, Some(TomlPair::new("junk", TomlValue::Integer(0))));
    }

    #[test]
    fn test_reading_signed_integer2() {
        let toml_string = "junk = +123";
        let mut parser = super::Parser::new();
        let pair = parser.read_test_pair(toml_string).unwrap();
        assert_eq!(pair, Some(TomlPair::new("junk", TomlValue::Integer(123))));
    }

    #[test]
    fn test_reading_negative_integer() {
        let toml_string = "data1 = -245_546";
        let mut parser = super::Parser::new();
        let pair = parser.read_test_pair(toml_string).unwrap();
        assert_eq!(
            pair,
            Some(TomlPair::new("data1", TomlValue::Integer(-245546)))
        );
    }

    #[test]
    fn test_reading_negative_integer2() {
        let toml_string = "data1 = -46\r\n";
        let mut parser = super::Parser::new();
        let pair = parser.read_test_pair(toml_string).unwrap();
        assert_eq!(pair, Some(TomlPair::new("data1", TomlValue::Integer(-46))));
    }

    #[test]
    fn test_reading_negative_integer3() {
        let toml_string = "data1 = -0\r\n";
        let mut parser = super::Parser::new();
        let pair = parser.read_test_pair(toml_string).unwrap();
        assert_eq!(pair, Some(TomlPair::new("data1", TomlValue::Integer(0))));
    }

    #[test]
    fn test_reading_negative_integer4() {
        let toml_string = "data1 = -0";
        let mut parser = super::Parser::new();
        let pair = parser.read_test_pair(toml_string).unwrap();
        assert_eq!(pair, Some(TomlPair::new("data1", TomlValue::Integer(0))));
    }

    #[test]
    fn test_reading_negative_integer5() {
        let toml_string = "data1 = -0 ";
        let mut parser = super::Parser::new();
        let pair = parser.read_test_pair(toml_string).unwrap();
        assert_eq!(pair, Some(TomlPair::new("data1", TomlValue::Integer(0))));
    }

    #[test]
    fn test_reading_binary_integer() {
        let toml_string = "junk = 0b111";
        let mut parser = super::Parser::new();
        let pair = parser.read_test_pair(toml_string).unwrap();
        assert_eq!(pair, Some(TomlPair::new("junk", TomlValue::Integer(0b111))));
    }

    #[test]
    fn test_reading_binary_integer2() {
        let toml_string = "junk = 0b11101\r\n";
        let mut parser = super::Parser::new();
        let pair = parser.read_test_pair(toml_string).unwrap();
        assert_eq!(
            pair,
            Some(TomlPair::new("junk", TomlValue::Integer(0b11101)))
        );
    }

    #[test]
    fn test_reading_hex_integer() {
        let toml_string = "junk = 0x45b\r\n";
        let mut parser = super::Parser::new();
        let pair = parser.read_test_pair(toml_string).unwrap();
        assert_eq!(pair, Some(TomlPair::new("junk", TomlValue::Integer(0x45b))));
    }

    #[test]
    fn test_reading_octal_integer() {
        let toml_string = "junk = 0o457\r\n";
        let mut parser = super::Parser::new();
        let pair = parser.read_test_pair(toml_string).unwrap();
        assert_eq!(pair, Some(TomlPair::new("junk", TomlValue::Integer(0o457))));
    }

    #[test]
    fn test_reading_float() {
        let toml_string = "float = 123.43\n";
        let mut parser = super::Parser::new();
        // Reading a float will panic for now
        let pair = parser.read_test_pair(toml_string).unwrap();
        assert_eq!(
            pair,
            Some(TomlPair::new("float", TomlValue::Float(123.43))),
            "Failed first test"
        );
    }

    #[test]
    fn test_reading_float2() {
        let toml_string = "float = 123.43\r\n";
        let mut parser = super::Parser::new();
        // Reading a float will panic for now
        let pair = parser.read_test_pair(toml_string).unwrap();
        assert_eq!(
            pair,
            Some(TomlPair::new("float", TomlValue::Float(123.43))),
            "Failed first test"
        );
    }

    #[test]
    fn test_reading_float3() {
        let toml_string = "float = 0.4_3\r\n";
        let mut parser = super::Parser::new();
        // Reading a float will panic for now
        let pair = parser.read_test_pair(toml_string).unwrap();
        assert_eq!(
            pair,
            Some(TomlPair::new("float", TomlValue::Float(0.43))),
            "Failed first test"
        );
    }

    #[test]
    fn test_reading_float4() {
        let toml_string = "float = 0.1e02";
        let mut parser = super::Parser::new();
        // Reading a float will panic for now
        let pair = parser.read_test_pair(toml_string).unwrap().unwrap();

        assert_eq!(pair.name, "float", "Failed first test");
        let value = match pair.value {
            TomlValue::Float(d) => d,
            _ => unreachable!(
                "Float was parsed as not a float, value was {:?}",
                pair.value
            ),
        };
        println!("Value is: {}", value);
        assert!((value - 0.1e02).abs() < 0.01, "Values were not equilivant");
    }

    #[test]
    fn test_reading_float5() {
        let toml_string = "float = 0.1e02e3";
        let mut parser = super::Parser::new();
        // Reading a float will panic for now
        let error = parser.read_test_pair(toml_string).unwrap_err();

        assert_eq!(
            error.kind(),
            ErrorKind::InvalidValue(1),
            "Failed first test"
        );
    }

    #[test]
    fn test_reading_boolean() {
        let toml_string = "boolean = false\nboolean2 = true";
        let mut parser = super::Parser::new();
        // Reading a float will panic for now
        let pair = parser.read_test_pair(toml_string).unwrap();
        assert_eq!(
            pair,
            Some(TomlPair::new("boolean", TomlValue::Bool(false))),
            "Failed first boolean test"
        );

        // Reading a float will panic for now
        let pair = parser.read_test_pair(toml_string).unwrap();
        assert_eq!(
            pair,
            Some(TomlPair::new("boolean2", TomlValue::Bool(true))),
            "Failed second boolean test"
        );
    }

    #[test]
    fn test_reading_boolean2() {
        let toml_string = "boolean = f";
        let mut parser = super::Parser::new();
        // Reading a float will panic for now
        let error = parser.read_test_pair(toml_string).unwrap_err();
        assert_eq!(
            error.kind(),
            ErrorKind::InvalidValue(1),
            "Expected invalid value on the first line"
        );

        let toml_string2 = "boolean = tru";
        let mut parser2 = super::Parser::new();
        // Reading a float will panic for now
        let error2 = parser2.read_test_pair(toml_string2).unwrap_err();
        assert_eq!(
            error2.kind(),
            ErrorKind::InvalidValue(1),
            "Expected invalid value on the first line"
        );

        let toml_string3 = "boolean = fals";
        let mut parser3 = super::Parser::new();
        // Reading a float will panic for now
        let error3 = parser3.read_test_pair(toml_string3).unwrap_err();
        assert_eq!(
            error3.kind(),
            ErrorKind::InvalidValue(1),
            "Expected invalid value on the first line"
        );
    }
}
