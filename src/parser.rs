use std::{collections::HashMap, mem::replace};

use error::ErrorKind;

use super::{
    error::{self, Error},
    Parser, ParserState, TomlPair, TomlValue, ValueType,
};

impl Parser {
    pub fn new() -> Parser {
        Parser {
            position: 0,
            state: ParserState::NewLine,
            line_number: 1,
            name_start: None,
            name_end: None,
            value_end: None,
            value_start: None,
            value_type: ValueType::Unknown,
        }
    }

    pub fn set_name_start(&mut self, name_start: usize) {
        debug_assert!(
            self.name_start.is_none(),
            "ASSERT FAILED: Incorrect usage of set name start - name start can only be set once"
        );
        self.name_start = Some(name_start);
    }

    pub fn set_name_end(&mut self, name_end: usize) {
        debug_assert!(
            self.name_end.is_none(),
            "ASSERT FAILED: Incorrect usage of set name end - name end can only be set once"
        );
        self.name_end = Some(name_end);
    }

    pub fn set_value_start(&mut self, value_start: usize) {
        debug_assert!(
            self.value_start.is_none(),
            "ASSERT FAILED: Incorrect usage of set value start - value start can only be set once"
        );
        self.value_start = Some(value_start);
    }

    pub fn set_value_end(&mut self, value_end: usize) {
        debug_assert!(
            self.value_end.is_none(),
            "ASSERT FAILED: Incorrect usage of set value end - value end can only be set once"
        );
        self.value_end = Some(value_end);
    }

    pub fn name_start(&mut self) -> usize {
        debug_assert!(
            self.name_start.is_some(),
            "ASSERT FAILED: Retrieving name start index before it has been set"
        );
        replace(&mut self.name_start, None).unwrap()
    }

    pub fn name_end(&mut self) -> usize {
        debug_assert!(
            self.name_end.is_some(),
            "ASSERT FAILED: Retrieving name end index before it has been set"
        );
        replace(&mut self.name_end, None).unwrap()
    }

    pub fn value_end(&mut self) -> usize {
        debug_assert!(
            self.value_end.is_some(),
            "ASSERT FAILED: Retrieving value end index before it has been set"
        );
        replace(&mut self.value_end, None).unwrap()
    }

    pub fn value_start(&mut self) -> usize {
        debug_assert!(
            self.value_start.is_some(),
            "ASSERT FAILED: Retrieving value start index before it has been set"
        );
        replace(&mut self.value_start, None).unwrap()
    }

    // TODO: Pass in a string here instead of storing it in
    pub fn parse<'a>(
        &mut self,
        data_to_parse: &'a str,
    ) -> Result<HashMap<&'a str, TomlValue<'a>>, error::Error> {
        while let Some(pair) = self.read_pair(data_to_parse)? {
            // Add it to the hashmap
        }
        Ok(HashMap::new())
    }

    pub fn read_pair<'a>(
        &mut self,
        data_to_parse: &'a str,
    ) -> Result<Option<TomlPair<'a>>, error::Error> {
        // Take the current position and read the next name value pair
        // The name ends after we read an equal ?
        // We treat it as a state machine - ie initial state reading a name, then reading a value
        // A line must contain a = unless it is a multiline value but we process them in one go
        // This is not correct, a line may begin with whitespace
        if data_to_parse.len() == self.position {
            println!("End of buffer");
            return Ok(None);
        }

        let mut sequence = data_to_parse[self.position..].chars().enumerate();
        println!(
            "Remaining string to parse is \"{}\"",
            &data_to_parse[self.position..]
        );

        loop {
            match self.state {
                ParserState::NewLine => {
                    // Returns none otherwise continues
                    self.process_new_line_state(&mut sequence)?;
                }
                ParserState::ReadingName => {
                    self.process_reading_name_state(&mut sequence)?;
                }
                ParserState::BeforeEquals => {
                    self.process_before_equals_state(&mut sequence)?;
                }
                ParserState::AfterEquals => {
                    self.process_after_equals_state(&mut sequence)?;
                }
                ParserState::ReadingInteger => {
                    // This state can move to ReadingFloat anytime we see a .
                    self.process_read_integer_state(&mut sequence, data_to_parse)?;
                }
                ParserState::ReadingString => {
                    self.process_read_string_state(&mut sequence)?;
                }
                ParserState::PairDone => {
                    // Index becomes the new position
                    return Ok(Some(
                        self.process_pair_done_state(&mut sequence, data_to_parse)?,
                    ));
                }
            }
        }
    }
    // Processes the state after reading a pair, returns Some(()) if a name was found otherwise returns none
    fn process_new_line_state(
        &mut self,
        sequence: &mut std::iter::Enumerate<std::str::Chars>,
    ) -> Result<Option<()>, error::Error> {
        loop {
            // while let Some(index, char) = sequence.next() {} // Process None outside of loop
            match sequence.next() {
                Some((index, char)) => match char {
                    // TODO: We can loop internally whenever there is no change in state and only break when state changes
                    ' ' | '\t' => {
                        println!("Whitespace on a new line");
                    }
                    '\n' => {
                        // Whitespace - no op
                        self.line_number += 1;
                    }
                    '\r' => {
                        // Next character must be \n and then return pair
                        if let Some((index, '\n')) = sequence.next() {
                            // No Op continue looking for the start of a name
                            self.line_number += 1;
                        } else {
                            return Err(error::Error::new(
                                ErrorKind::InvalidEndOfLine(self.line_number),
                                None,
                            ));
                        }
                    }
                    _ => {
                        self.state = ParserState::ReadingName;

                        self.set_name_start(index + self.position);
                        //*name_starts = index + self.position;
                        println!("Starting reading name with {} - {:?}", char, char);
                        return Ok(Some(()));
                    }
                },
                None => {
                    // File ended with a new line
                    println!("File ended with a new line");
                    return Ok(None);
                }
            }
        }
    }

    fn process_reading_name_state(
        &mut self,
        sequence: &mut std::iter::Enumerate<std::str::Chars>,
    ) -> Result<(), Error> {
        loop {
            match sequence.next() {
                Some((index, char)) => match char {
                    '=' => {
                        self.state = ParserState::AfterEquals;
                        self.set_name_end(index + self.position);
                        // *name_ends = index + self.position;
                        return Ok(());
                    }
                    ' ' | '\t' => {
                        self.state = ParserState::BeforeEquals;
                        // *name_ends = index + self.position;
                        self.set_name_end(index + self.position);
                        return Ok(());
                    }
                    '\n' | '\r' => {
                        // Not valid a name can't be multiline
                        return Err(Error::new(ErrorKind::InvalidName(self.line_number), None));
                    }
                    _ => {
                        println!("While reading name we got character: {} at {}", char, index);
                        // No Op -
                    }
                },

                None => {
                    // File ended when reading the name - this is an error

                    return Err(Error::new(ErrorKind::InvalidName(self.line_number), None));
                }
            }
        }
    }

    fn process_before_equals_state(
        &mut self,
        sequence: &mut std::iter::Enumerate<std::str::Chars>,
    ) -> Result<(), Error> {
        loop {
            match sequence.next() {
                Some((index, char)) => match char {
                    '=' => {
                        self.state = ParserState::AfterEquals;
                        return Ok(());
                    }
                    ' ' | '\t' => {
                        // No Op we are waiting for a =
                    }
                    _ => {
                        // This is invalid a name and a value must be seperated by a = and optionally whitespace
                        // anything else is invalid
                        return Err(Error::new(ErrorKind::InvalidName(self.line_number), None));
                    }
                },
                None => {
                    // File ended after a name but before a =
                }
            }
        }
    }

    fn process_after_equals_state(
        &mut self,
        sequence: &mut std::iter::Enumerate<std::str::Chars>,
    ) -> Result<(), Error> {
        loop {
            match sequence.next() {
                Some((index, char)) => match char {
                    ' ' | '\t' => {
                        // No Op we are waiting for the start of a value
                    }
                    '"' => {
                        self.state = ParserState::ReadingString;
                        self.set_value_start(index + self.position + 1);
                        // *value_starts = index + self.position + 1;
                        return Ok(());
                    }
                    char if char.is_ascii_digit() == true => {
                        // Not true we could be reading a float
                        self.state = ParserState::ReadingInteger;
                        // *value_starts = index + self.position + 1;
                        self.set_value_start(index + self.position + 1);
                        return Ok(());
                    }
                    // TODO: Support for multiline strings etc
                    _ => {
                        // This should be an error since we have hit a value we dont recognize
                        return Err(Error::new(
                            ErrorKind::UnknownValueType(self.line_number),
                            None,
                        ));
                    }
                },
                None => {
                    // File ended after equals but before we saw a value
                    return Err(Error::new(ErrorKind::MissingValue(self.line_number), None));
                }
            }
        }
    }

    fn process_read_integer_state(
        &mut self,
        sequence: &mut std::iter::Enumerate<std::str::Chars>,
        data_to_parse: &str,
    ) -> Result<(), Error> {
        self.value_type = ValueType::Integer;
        loop {
            match sequence.next() {
                Some((index, char)) => match char {
                    char if char.is_ascii_digit() == true => {
                        // No Op
                    }
                    ' ' | '\t' => {
                        // Whitespace means the integer ended.
                        self.state = ParserState::PairDone;
                        self.set_value_end(index + self.position);
                        return Ok(());
                    }
                    '.' => {
                        unimplemented!("No support for floats");
                        //break;
                    }
                    '\n' => {
                        // End of integer
                        self.state = ParserState::PairDone;
                        self.line_number += 1;
                        self.set_value_end(index + self.position);
                        //return integer
                        return Ok(());
                    }
                    '\r' => {
                        // Next character must be \n and then return pair
                        if let Some((index, '\n')) = sequence.next() {
                            self.state = ParserState::PairDone;
                            self.line_number += 1;
                            self.set_value_end(index + self.position + 1);
                            return Ok(());
                        } else {
                            // Error - invalid integer because of invalid end of line
                            return Err(Error::new(
                                ErrorKind::InvalidEndOfLine(self.line_number),
                                None,
                            ));
                        }
                    }
                    _ => {
                        // Error - invalid character in integer
                        return Err(Error::new(ErrorKind::InvalidValue(self.line_number), None));
                    }
                },
                None => {
                    // File ended while reading an integer, this is valid the end of the file denotes the end of the integer
                    // TODO: We need the length of the buffer to set an appropriate end position
                    self.set_value_end(data_to_parse.len());
                    return Ok(());
                }
            }
        }
    }

    fn process_read_string_state(
        &mut self,
        sequence: &mut std::iter::Enumerate<std::str::Chars>,
    ) -> Result<(), Error> {
        self.value_type = ValueType::String;
        loop {
            match sequence.next() {
                Some((index, char)) => match char {
                    '"' => {
                        // End of the string
                        // We are now scanning for the end of line
                        println!("Found end of string at {}", index);
                        self.state = ParserState::PairDone;
                        self.set_value_end(index + self.position);
                        // *value_ends = index + self.position;
                        return Ok(());
                    }
                    '\n' => {
                        // End of line without ending the string - this is invalid to read but not to produce
                        return Err(Error::new(ErrorKind::InvalidValue(self.line_number), None));
                    }
                    // TODO: Escaped characters are supported?
                    // An escaped character necessitates copying the string to a new value since we need to convert the escapes into actual characters
                    // TODO: Invalid characters inside a string are there any?? - any unsupported escaped characters
                    _ => {
                        // A character
                        println!("Got {} as part of a string", char);
                    }
                },
                None => {
                    // This is invalid a string is only valid if it is ended with a "
                    return Err(Error::new(ErrorKind::InvalidValue(self.line_number), None));
                }
            }
        }
    }

    fn process_pair_done_state<'a>(
        &mut self,
        sequence: &mut std::iter::Enumerate<std::str::Chars>,
        data_to_parse: &'a str,
    ) -> Result<TomlPair<'a>, Error> {
        loop {
            match sequence.next() {
                Some((index, char)) => match char {
                    ' ' | '\t' => {

                        // No Op we are just looking for a new line or eof at this point
                    }
                    '\n' => {
                        println!("Found start of next pair at {}", index);
                        self.state = ParserState::NewLine;
                        let value = &data_to_parse[self.value_start()..self.value_end()];
                        let name = &data_to_parse[self.name_start()..self.name_end()];
                        self.line_number += 1;
                        self.position += index;
                        match self.value_type {
                            ValueType::Integer => {
                                let integer = match u64::from_str_radix(value, 10) {
                                    Ok(integer) => integer,
                                    Err(error) => {
                                        return Err(Error::new(
                                            ErrorKind::InvalidValue(self.line_number),
                                            Some(Box::new(error)),
                                        ))
                                    }
                                };
                                return Ok(TomlPair::new(name, TomlValue::Integer(integer)));
                            }
                            ValueType::String => {
                                return Ok(TomlPair::new(name, TomlValue::String(value)))
                            }
                            _ => {}
                        }
                    }
                    _ => {
                        println!("Random character after pair read - invalid ? {}", char);
                        return Err(Error::new(ErrorKind::InvalidValue(self.line_number), None));
                        // TODO: Return error
                    }
                },
                None => {
                    // Pair completed and we reached the end of the file
                    println!("File completed after reading a pair");
                    // Ensure that we point to the end of the buffer
                    self.state = ParserState::NewLine;
                    self.position = data_to_parse.len();
                    let value = &data_to_parse[self.value_start()..self.value_end()];
                    let name = &data_to_parse[self.name_start()..self.name_end()];
                    // let value = &self.buffer[value_starts..value_ends];
                    // let name = &self.buffer[name_starts..name_ends];
                    return Ok(TomlPair::new(name, TomlValue::String(value)));
                }
            }
        }
    }
}
