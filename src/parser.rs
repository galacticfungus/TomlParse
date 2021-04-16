use std::{collections::HashMap, mem::replace};

use super::{Error, ErrorKind, Parser, ParserState, TomlPair, TomlValue};

impl Parser {
    pub fn new() -> Parser {
        Parser {
            position: 0,
            state: ParserState::Normal,
            line_number: 1,
            name_start: None,
            name_end: None,
            value_end: None,
            value_start: None,
        }
    }

    fn set_name_start(&mut self, name_start: usize) {
        debug_assert!(
            self.name_start.is_none(),
            "ASSERT FAILED: Incorrect usage of set name start - name start can only be set once"
        );
        self.name_start = Some(name_start);
    }

    fn set_name_end(&mut self, name_end: usize) {
        debug_assert!(
            self.name_end.is_none(),
            "ASSERT FAILED: Incorrect usage of set name end - name end can only be set once"
        );
        self.name_end = Some(name_end);
    }

    fn set_value_start(&mut self, value_start: usize) {
        debug_assert!(
            self.value_start.is_none(),
            "ASSERT FAILED: Incorrect usage of set value start - value start can only be set once"
        );
        self.value_start = Some(value_start);
    }

    fn set_value_end(&mut self, value_end: usize) {
        debug_assert!(
            self.value_end.is_none(),
            "ASSERT FAILED: Incorrect usage of set value end - value end can only be set once"
        );
        self.value_end = Some(value_end);
    }

    fn name_start(&mut self) -> usize {
        debug_assert!(
            self.name_start.is_some(),
            "ASSERT FAILED: Retrieving name start index before it has been set"
        );
        replace(&mut self.name_start, None).unwrap()
    }

    fn name_end(&mut self) -> usize {
        debug_assert!(
            self.name_end.is_some(),
            "ASSERT FAILED: Retrieving name end index before it has been set"
        );
        replace(&mut self.name_end, None).unwrap()
    }

    fn value_end(&mut self) -> usize {
        debug_assert!(
            self.value_end.is_some(),
            "ASSERT FAILED: Retrieving value end index before it has been set"
        );
        replace(&mut self.value_end, None).unwrap()
    }

    fn value_start(&mut self) -> usize {
        debug_assert!(
            self.value_start.is_some(),
            "ASSERT FAILED: Retrieving value start index before it has been set"
        );
        replace(&mut self.value_start, None).unwrap()
    }

    pub fn parse<'a>(
        &mut self,
        data_to_parse: &'a str,
    ) -> Result<HashMap<&'a str, TomlValue<'a>>, Error> {
        let mut hs = HashMap::new();
        while let Some(pair) = self.read_pair(data_to_parse)? {
            // Add it to the hashmap
            hs.insert(pair.name, pair.value);
        }
        Ok(hs)
    }

    /// Wrapper around read_pair that is used in testing
    #[cfg(test)]
    pub(crate) fn read_test_pair<'a>(
        &mut self,
        data_to_parse: &'a str,
    ) -> Result<Option<TomlPair<'a>>, Error> {
        let pair = self.read_pair(data_to_parse)?;
        Ok(pair)
    }

    // TODO: Convert to stream to allow file io while parsing
    /// Returns the next TOML statement, returns none if there are no more lines
    fn read_pair<'a>(&mut self, data_to_parse: &'a str) -> Result<Option<TomlPair<'a>>, Error> {
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
            "Remaining string to parse is {}",
            &data_to_parse[self.position..]
        );

        println!(
            "Pointing at character {:?}",
            &data_to_parse[self.position..]
        );

        loop {
            match self.state {
                ParserState::Normal => {
                    // Normal state means we are ready to accept a new name value pair or the end of the file
                    self.process_normal_state(&mut sequence)?;
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
                    // return Ok(Some(value));
                }
                ParserState::ReadingFloat(after_exponent) => {
                    self.process_read_float_state(&mut sequence, data_to_parse, after_exponent)?;
                }
                ParserState::ReadingString => {
                    self.process_read_string_state(&mut sequence)?;
                    // return Ok(Some(string_value));
                }
                ParserState::AfterValue => {
                    // This state means we have read in a name value pair but we have not seen a new line that would indicate the start of a potential new name value pair
                    // self.state.next()
                    self.process_after_value_state(&mut sequence)?;
                }
                ParserState::ReadingTrue(index) => {
                    self.process_reading_true_state(index, data_to_parse)?
                }
                ParserState::ReadingFalse(index) => {
                    self.process_reading_false_state(index, data_to_parse)?
                }
                // Add states building BuildingString, BuildingInteger, BuildingBoolean etc these are the final states, once this state is reached the pair has been read
                ParserState::FinishedFloat(eof, new_line) => {
                    let float_pair = self.build_float_pair(data_to_parse)?;
                    match (eof, new_line) {
                        (true, false) => self.state = ParserState::EndOfFile,
                        
                        (false, true) => self.state = ParserState::Normal,
                        (false, false) => self.state = ParserState::AfterValue,
                        (true, true) => unreachable!("Both the new line and eof were marked when entering the FinishedInteger State, this should be impossible"),
                    }
                    return Ok(Some(float_pair));
                }
                ParserState::FinishedBoolean(bool_value) => {
                    let pair = self.build_bool_pair(bool_value, data_to_parse)?;
                    self.state = ParserState::AfterValue;
                    return Ok(Some(pair));
                }
                ParserState::FinishedInteger(eof, eol) => {
                    let int_pair = self.build_integer_pair(data_to_parse)?;
                    match (eof, eol) {
                        // Eof after reading the integer
                        (true, false) => self.state = ParserState::EndOfFile,
                        // We can proceed directly to normal state since we already saw a new line
                        (false, true) => self.state = ParserState::Normal,
                        (false, false) => self.state = ParserState::AfterValue,
                        (true, true) => unreachable!("Both the new line and eof were marked when entering the FinishedInteger State, this should be impossible"),
                        
                    }
                    return Ok(Some(int_pair));
                }
                ParserState::FinishedNegativeInteger(eof, eol) => {
                    let int_pair = self.build_negative_integer_pair(data_to_parse)?;
                    match (eof, eol) {
                        // Eof after reading the integer
                        (true, false) => self.state = ParserState::EndOfFile,
                        // We can proceed directly to normal state since we already saw a new line
                        (false, true) => self.state = ParserState::Normal,
                        (false, false) => self.state = ParserState::AfterValue,
                        (true, true) => unreachable!("Both the new line and eof were marked when entering the FinishedInteger State, this should be impossible"),
                        
                    }
                    return Ok(Some(int_pair));
                }
                ParserState::ReadingOctalInteger => {
                    self.process_reading_octal(&mut sequence, data_to_parse)?;
                }
                ParserState::ReadingBinaryInteger => {
                    self.process_reading_binary(&mut sequence, data_to_parse)?;
                },
                ParserState::ReadingNegativeInteger => {
                    self.process_negative_integer(&mut sequence, data_to_parse)?;
                }
                ParserState::FinishedBinaryInteger(eof, eol) => {
                    let int_pair = self.build_binary_integer_pair(data_to_parse)?;
                    match (eof, eol) {
                        // Eof after reading the integer
                        (true, false) => self.state = ParserState::EndOfFile,
                        // We can proceed directly to normal state since we already saw a new line
                        (false, true) => self.state = ParserState::Normal,
                        (false, false) => self.state = ParserState::AfterValue,
                        (true, true) => unreachable!("Both the new line and eof were marked when entering the FinishedInteger State, this should be impossible"),
                        
                    }
                    return Ok(Some(int_pair));
                },
                ParserState::FinishedHexInteger(eof, eol) => {
                    let int_pair = self.build_hex_integer_pair(data_to_parse)?;
                    match (eof, eol) {
                        // Eof after reading the integer
                        (true, false) => self.state = ParserState::EndOfFile,
                        // We can proceed directly to normal state since we already saw a new line
                        (false, true) => self.state = ParserState::Normal,
                        (false, false) => self.state = ParserState::AfterValue,
                        (true, true) => unreachable!("Both the new line and eof were marked when entering the FinishedInteger State, this should be impossible"),
                    }
                    return Ok(Some(int_pair));
                },
                ParserState::FinishedOctalInteger(eof, eol) => {
                    let int_pair = self.build_octal_integer_pair(data_to_parse)?;
                    match (eof, eol) {
                        // Eof after reading the integer
                        (true, false) => self.state = ParserState::EndOfFile,
                        // We can proceed directly to normal state since we already saw a new line
                        (false, true) => self.state = ParserState::Normal,
                        (false, false) => self.state = ParserState::AfterValue,
                        (true, true) => unreachable!("Both the new line and eof were marked when entering the FinishedInteger State, this should be impossible"),
                        
                    }
                    return Ok(Some(int_pair));
                },
                ParserState::ReadingHexInteger => {
                    self.process_reading_hex(&mut sequence, data_to_parse)?;
                }
                ParserState::FinishedString => {
                    let string_pair = self.build_string_pair(data_to_parse)?;
                    self.state = ParserState::AfterValue;
                    return Ok(Some(string_pair));
                }
                ParserState::EndOfFile => {
                    return Ok(None);
                }
            }
        }
    }

    fn process_reading_true_state<'a>(
        &mut self,
        start_index: usize,
        data_to_parse: &'a str,
    ) -> Result<(), Error> {
        // data_to_parse[value_start..value_start + 4] == true
        // else invalid value
        if data_to_parse.len() < start_index + 4 {
            // Buffer is not long enough to contain the true so its an invalid value
            return Err(Error::new(ErrorKind::InvalidValue(self.line_number), None));
        }
        match &data_to_parse[start_index..start_index + 4] {
            "true" => self.set_value_end(start_index + 4),
            _ => return Err(Error::new(ErrorKind::InvalidValue(self.line_number), None)),
        }
        self.position += start_index + 4;
        self.state = ParserState::FinishedBoolean(true);
        Ok(())
    }

    fn process_reading_false_state<'a>(
        &mut self,
        start_index: usize,
        data_to_parse: &'a str,
    ) -> Result<(), Error> {
        // data_to_parse[value_start..value_start + 5] == false
        // else invalid value
        if data_to_parse.len() < start_index + 5 {
            // Buffer is not long enough to contain the true so its an invalid value
            return Err(Error::new(ErrorKind::InvalidValue(self.line_number), None));
        }
        match &data_to_parse[start_index..start_index + 5] {
            "false" => self.set_value_end(start_index + 5),
            _ => return Err(Error::new(ErrorKind::InvalidValue(self.line_number), None)),
        }
        self.position += start_index + 5;
        self.state = ParserState::FinishedBoolean(false);
        Ok(())
    }

    fn process_after_value_state(
        &mut self,
        sequence: &mut std::iter::Enumerate<std::str::Chars>,
    ) -> Result<(), Error> {
        match sequence.next() {
            Some((_, char)) => match char {
                ' ' | '\t' => {
                    println!("Whitespace after a value");
                    return Ok(());
                }
                '#' => {
                    // Comment after value is valid
                    // self.transition_to(ParserState::Comment)
                    unimplemented!("Comments are not done");
                }
                '\n' => {
                    // Whitespace - Move to NewLine state
                    self.line_number += 1;
                    self.state = ParserState::Normal;
                    return Ok(());
                }
                '\r' => {
                    // Next character must be \n and then return pair
                    if let Some((_, '\n')) = sequence.next() {
                        // No Op continue looking for the start of a name
                        self.line_number += 1;
                        self.state = ParserState::Normal;
                        return Ok(());
                    } else {
                        return Err(Error::new(
                            ErrorKind::InvalidEndOfLine(self.line_number),
                            None,
                        ));
                    }
                }
                _ => {
                    // Error invalid value - started a new value or name on the same line as a completed name/value
                    return Err(Error::new(ErrorKind::InvalidValue(self.line_number), None));
                }
            },
            None => {
                // File ended with a new line
                println!("File ended with a new line");
                self.state = ParserState::EndOfFile;
                return Ok(());
            }
        }
    }

    // Processes the state after reading a pair, returns Some(()) if a name was found otherwise returns none
    // Returns Some(()) if we didn't reach the end of the file
    fn process_normal_state(
        &mut self,
        sequence: &mut std::iter::Enumerate<std::str::Chars>,
    ) -> Result<(), Error> {
        match sequence.next() {
            Some((index, char)) => match char {
                ' ' | '\t' => {
                    println!("Whitespace on a new line");
                    return Ok(());
                }
                '#' => {
                    // Comment, we scan until the end of the line
                    // self.transition_to(ParserState::CommentLine);
                    unimplemented!("Support for comments not done");
                }
                '\n' => {
                    // Whitespace - no op
                    self.line_number += 1;
                    return Ok(());
                }
                '\r' => {
                    // Next character must be \n and then return pair
                    if let Some((_, '\n')) = sequence.next() {
                        // No Op continue looking for the start of a name
                        self.line_number += 1;
                        return Ok(());
                    } else {
                        return Err(Error::new(
                            ErrorKind::InvalidEndOfLine(self.line_number),
                            None,
                        ));
                    }
                }
                '"' => {
                    unimplemented!("String names are not supported yet");
                    // self.transition_to(ParserState::ReadingStringName);
                    // self.state = ParserState::ReadingStringName;
                    //self.set_name_start(index + self.position + 1);
                    //return Ok(Some(()));
                }
                _ => {
                    self.state = ParserState::ReadingName;
                    self.set_name_start(index + self.position);
                    println!("Starting reading name with {} - {:?}", char, char);
                    return Ok(());
                }
            },
            None => {
                // File ended with a new line
                println!("File ended with a new line");
                // Change state to end of file
                self.state = ParserState::EndOfFile;
                return Ok(());
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
                        // Not valid - a name can't be multiline
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
                Some((_, char)) => match char {
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
                    return Err(Error::new(ErrorKind::MissingValue(self.line_number), None));
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
                    '\n' => {
                        return Err(Error::new(ErrorKind::MissingValue(self.line_number), None));
                    }
                    '\r' => {
                        if let Some((_, '\n')) = sequence.next() {
                            return Err(Error::new(
                                ErrorKind::MissingValue(self.line_number),
                                None,
                            ));
                        } else {
                            return Err(Error::new(
                                ErrorKind::InvalidEndOfLine(self.line_number),
                                None,
                            ));
                        }
                        // return Err(Error::new(ErrorKind::MissingValue(self.line_number), None));
                    }
                    '#' => {
                        // Invalid missing a value
                        return Err(Error::new(ErrorKind::MissingValue(self.line_number), None));
                    }
                    '"' => {
                        self.state = ParserState::ReadingString;
                        self.set_value_start(index + self.position + 1);
                        return Ok(());
                    }
                    // Booleans are always lower case...
                    't' => {
                        self.state = ParserState::ReadingTrue(index + self.position);
                        self.set_value_start(index + self.position);
                        return Ok(());
                    }
                    'f' => {
                        self.state = ParserState::ReadingFalse(index + self.position);
                        self.set_value_start(index + self.position);
                        return Ok(());
                    }
                    '0' => {
                        // This could be a 0
                        // or a 0.0423 float
                        match sequence.next() {
                            Some((after_zero_index, char)) => {
                                match char {
                                    ' ' | '\t' => {
                                        // Basic integer 0
                                        self.state = ParserState::FinishedInteger(false, false);
                                        self.set_value_start(index + self.position);
                                        self.set_value_end(after_zero_index);
                                        return Ok(());
                                    },
                                    '.' => {
                                        self.state = ParserState::ReadingFloat(false);
                                        self.set_value_start(index + self.position);
                                        return Ok(());
                                    },
                                    'e' | 'E' => {
                                        // TODO: 0e2 is valid?
                                        self.state = ParserState::ReadingFloat(true);
                                        self.set_value_start(index + self.position);
                                        return Ok(());
                                    },
                                    '\n' => {
                                        // Basic integer zero followed by a new line
                                        self.state = ParserState::FinishedInteger(false, true);
                                        self.set_value_start(index + self.position);
                                        self.set_value_end(after_zero_index);
                                        return Ok(());
                                    }
                                    '\r' => {
                                        if let Some((_, '\n')) = sequence.next() {
                                            // Basic integer zero followed by a new line
                                            self.state = ParserState::FinishedInteger(false, true);
                                            self.set_value_start(index + self.position);
                                            self.set_value_end(after_zero_index);
                                            return Ok(());
                                        } else {
                                            return Err(Error::new(
                                                ErrorKind::InvalidEndOfLine(self.line_number),
                                                None,
                                            ));
                                        }
                                    }
                                    'x' => {
                                        // Hex int
                                        self.state = ParserState::ReadingHexInteger;
                                        self.set_value_start(after_zero_index + self.position + 1);
                                        return Ok(());
                                    }
                                    'b' => {
                                        // binary int
                                        self.state = ParserState::ReadingBinaryInteger;
                                        self.set_value_start(after_zero_index + self.position + 1);
                                        return Ok(());
                                    }
                                    'o' => {
                                        // octal int
                                        self.state = ParserState::ReadingOctalInteger;
                                        self.set_value_start(after_zero_index + self.position + 1);
                                        return Ok(());
                                    }
                                    char if char.is_ascii_digit() == true => {
                                        // Explicitly not allowed
                                        return Err(Error::new(
                                            ErrorKind::InvalidValue(self.line_number),
                                            None,
                                        ));
                                    }
                                    _ => {
                                        // TODO: There may be valid combinations left
                                        println!("Generic catch all hit after seeing a 0, may not be correct");
                                        return Err(Error::new(
                                            ErrorKind::InvalidValue(self.line_number),
                                            None,
                                        ));
                                    }
                                }
                            }
                            None => {
                                // File ended on a zero so we read a zero integer
                                self.state = ParserState::FinishedInteger(true, false);
                                self.set_value_start(index + self.position);
                                // TODO: Careful here as this could be unsafe???
                                self.set_value_end(index + self.position + 1);
                                return Ok(());
                            }
                        }
                    }
                    '-' => {
                        self.state = ParserState::ReadingNegativeInteger;
                        match sequence.next() {
                            Some((after_negative_sign, char)) => {
                                match char {
                                    '0' => {
                                        // Special case -0
                                        // The only allowed values after a -0 are whitespace, end of line or end of file
                                        match sequence.next() {
                                            Some((after_zero_index, char)) => {
                                                match char {
                                                    ' ' | '\t' => {
                                                        // Integer finished
                                                        self.state = ParserState::FinishedInteger(false, false);
                                                        self.set_value_start(after_negative_sign);
                                                        self.set_value_end(after_zero_index);
                                                        self.position += after_zero_index + 1;
                                                        return Ok(());
                                                    },
                                                    '\n' => {
                                                        // Integer finished
                                                        self.state = ParserState::FinishedInteger(false, true);
                                                        self.set_value_start(after_negative_sign);
                                                        self.set_value_end(after_zero_index);
                                                        self.position += after_zero_index + 1;
                                                        self.line_number += 1;
                                                        return Ok(());
                                                    },
                                                    '\r' => {
                                                        if let Some((_, '\n')) = sequence.next() {
                                                            // Basic integer zero followed by a new line
                                                            self.state = ParserState::FinishedInteger(false, true);
                                                            self.set_value_start(after_negative_sign);
                                                            self.set_value_end(after_zero_index);
                                                            self.position += after_zero_index + 2;
                                                            return Ok(());
                                                        } else {
                                                            return Err(Error::new(
                                                                ErrorKind::InvalidEndOfLine(self.line_number),
                                                                None,
                                                            ));
                                                        }
                                                    },
                                                    '#' => {
                                                        unimplemented!("Comments not done");
                                                    },
                                                    _ => {
                                                        return Err(Error::new(
                                                                ErrorKind::InvalidEndOfLine(self.line_number),
                                                                None,
                                                            ));
                                                    },
                                                }
                                            },
                                            None => {
                                                // Valid - File ended on a -0
                                                self.set_value_start(after_negative_sign);
                                                self.set_value_end(after_negative_sign + 1);
                                                self.state = ParserState::FinishedInteger(true, false);
                                                return Ok(());
                                            },
                                        }
                                    },
                                    char if char.is_ascii_digit() == true => {
                                        self.state = ParserState::ReadingNegativeInteger;
                                        self.set_value_start(self.position + after_negative_sign);
                                        return Ok(());
                                    },
                                    _ => {
                                        // Invalid
                                        return Err(Error::new(ErrorKind::InvalidValue(self.position), None));
                                    }
                                }
                            },
                            None => {
                                // Invalid
                                return Err(Error::new(ErrorKind::InvalidValue(self.position), None));
                            }
                        }                        
                    }
                    '+' => {
                        // -0 and +0 are valid and identical to an unprefixed zero
                        // + can only be a base 10 integer
                    }
                    '_' => {
                        // Invalid integer seperator
                        unimplemented!("Invalid integer seperator, ie value started with one, clever errors not finished")
                    }
                    char if char.is_ascii_digit() == true => {
                        // We could be reading a float or a decimal integer
                        self.state = ParserState::ReadingInteger;
                        self.set_value_start(index + self.position);
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

    fn process_negative_integer<'a>(&mut self, sequence: &mut std::iter::Enumerate<std::str::Chars>,
        data_to_parse: &'a str,
    ) -> Result<(), Error> {
        match sequence.next() {
            Some((index, char)) => match char {
                char if char.is_ascii_digit() == true => {
                    // Negative Integer digit
                    return Ok(());
                },
                '.' => {
                    // Negative float
                    unimplemented!("Negative floats not supported");
                },
                '_' => {
                    // This is valid as long as its folowed by a digit
                    return Ok(());
                }
                '\r' => {
                    if let Some((_, '\n')) = sequence.next() {
                        self.state = ParserState::FinishedNegativeInteger(false, true);
                        self.line_number += 1;
                        self.set_value_end(index + self.position);
                        self.position += index;
                        return Ok(());
                    } else {
                        return Err(Error::new(ErrorKind::InvalidEndOfLine(self.line_number), None));
                    }
                },
                '\n' => {
                        self.state = ParserState::FinishedNegativeInteger(false, true);
                        self.line_number += 1;
                        self.set_value_end(index + self.position);
                        self.position += index;
                        return Ok(());
                },
                ' ' | '\t' => {
                    self.state = ParserState::FinishedNegativeInteger(false, false);
                        self.set_value_end(index + self.position);
                        self.position += index;
                        return Ok(());
                }
                _ => return Err(Error::new(ErrorKind::InvalidValue(self.line_number), None)),
            },
            None => {
                self.state = ParserState::FinishedNegativeInteger(true, false);
                self.set_value_end(data_to_parse.len());
                // TODO: self.position is invalid at this point as it wasn't incremented by the length of the last character
                return Ok(());
            }
        }
    }

    fn process_read_integer_state<'a>(
        &mut self,
        sequence: &mut std::iter::Enumerate<std::str::Chars>,
        data_to_parse: &'a str,
    ) -> Result<(), Error> {
        // TODO: Deal with zero
        match sequence.next() {
            Some((index, char)) => match char {
                char if char.is_ascii_digit() == true => {
                    // No Op
                    return Ok(());
                }
                ' ' | '\t' => {
                    // Whitespace means the integer ended.
                    // This means we are still looking for a new line but the integer is done
                    self.state = ParserState::FinishedInteger(false, false);
                    self.set_value_end(index + self.position);
                    self.position += index;
                    // Moving to AfterValue state
                    // self.state = ParserState::AfterValue;
                    // return Ok(self.build_integer_pair(data_to_parse)?);
                    return Ok(());
                },
                '.' => {
                    // unimplemented!("No support for floats");
                    self.state = ParserState::ReadingFloat(false);
                    // let float_pair = self.process_read_float_state(sequence, data_to_parse)?;
                    return Ok(());
                    //break;
                },
                'e' | 'E' => {
                    self.state = ParserState::ReadingFloat(true);
                    // let float_pair = self.process_read_float_state(sequence, data_to_parse)?;
                    return Ok(());
                },
                '_' => {
                    // not a no op, read the next char to ensure it is a digit
                    match sequence.next() {
                        Some((_, char)) => {
                            match char {
                                char if char.is_ascii_digit() == true => {
                                    return Ok(());
                                }
                                _ => {
                                    // Invalid
                                    return Err(Error::new(ErrorKind::InvalidValue(self.line_number), None));
                                }
                            }
                        },
                        None => {
                            // Value ended with a _ which is invalid
                            return Err(Error::new(ErrorKind::InvalidValue(self.line_number), None));
                        }
                    }
                }
                '\n' => {
                    // End of integer
                    self.state = ParserState::FinishedInteger(false, true);
                    // Our state transition when finishing a value is based on the character that was read during the transition to finish
                    // (state, previous_character)
                    self.line_number += 1;
                    self.set_value_end(index + self.position);
                    self.position += index;
                    // return Ok(self.build_integer_pair(data_to_parse)?);
                    return Ok(());
                }
                '\r' => {
                    // Next character must be \n and then return pair
                    if let Some((_, '\n')) = sequence.next() {
                        self.state = ParserState::FinishedInteger(false, true);
                        self.line_number += 1;
                        self.set_value_end(index + self.position);
                        self.position += index;
                        // We set the previous token so that when we move from finished state we move to normal rather than after value
                        // return Ok(self.build_integer_pair(data_to_parse)?);
                        return Ok(());
                        // Integer is done and we are moving to NewLine state
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
                self.set_value_end(data_to_parse.len());
                self.state = ParserState::FinishedInteger(true, false);
                // TODO: This could lead to problems like reading the next character of the sequence no longer returns a null
                // return Ok(self.build_integer_pair(data_to_parse)?);
                return Ok(());
            }
        }
    }

    fn process_read_float_state<'a>(
        &mut self,
        sequence: &mut std::iter::Enumerate<std::str::Chars>,
        data_to_parse: &'a str,
        after_exponent: bool,
    ) -> Result<(), Error> {
        match sequence.next() {
            Some((index, char)) => match char {
                char if char.is_ascii_digit() == true => {
                    // No Op
                    return Ok(());
                }
                ' ' | '\t' => {
                    // Whitespace means the integer ended.
                    // This means we are still looking for a new line but the integer is done
                    // self.state = ParserState::AfterValue;
                    self.set_value_end(index + self.position);
                    self.position += index;
                    // Moving to AfterValue state
                    self.state = ParserState::FinishedFloat(false, false);
                    return Ok(());
                }
                '.' => {
                    // This is always an error since a period will always come before an exponential
                    // 1.34e5 is valid
                    // 1e02.45 is not
                    return Err(Error::new(ErrorKind::InvalidValue(self.line_number), None));
                },
                'e' | 'E' => {
                    // Check if we have already seen a e
                    if after_exponent {
                        return Err(Error::new(ErrorKind::InvalidValue(self.line_number), None));
                    }
                    // Change state so that we know we are reading the exponent now
                    self.state = ParserState::ReadingFloat(true);
                    // Next character must be a digit for this to be valid, but that digit can be a zero
                    // 23.456e0 is valid
                    // 21.5436e06 is valid
                    match sequence.next() {
                        Some((index, char)) => {
                            match char {
                                char if char.is_ascii_digit() == true => {
                                    return Ok(());
                                }
                                _ => {
                                    // Error - Exponential part of float was empty
                                    return Err(Error::new(ErrorKind::InvalidValue(self.line_number), None));
                                }
                            }
                            
                        }
                        None => {
                            // Error - File ended with line like
                            //fred = 23.457e
                            return Err(Error::new(ErrorKind::InvalidValue(self.line_number), None));
                        }
                    }
                }
                '\n' => {
                    // End of integer
                    self.state = ParserState::FinishedFloat(false, true);
                    // Our state transition when finishing a value is based on the character that was read during the transition to finish
                    // (state, previous_character)
                    self.line_number += 1;
                    self.set_value_end(index + self.position);
                    self.position += index;
                    return Ok(());
                }
                '\r' => {
                    // Next character must be \n and then return pair
                    if let Some((index, '\n')) = sequence.next() {
                        self.state = ParserState::FinishedFloat(false, true);
                        self.line_number += 1;
                        self.set_value_end(index + self.position - 1);
                        self.position += index;
                        // We set the previous token so that when we move from finished state we move to normal rather than after value
                        return Ok(());
                        // Integer is done and we are moving to NewLine state
                    } else {
                        // Error - invalid integer because of invalid end of line
                        return Err(Error::new(
                            ErrorKind::InvalidEndOfLine(self.line_number),
                            None,
                        ));
                    }
                },
                '_' => {
                    // not a no op, read the next char to ensure it is a digit
                    match sequence.next() {
                        Some((_, char)) => {
                            match char {
                                char if char.is_ascii_digit() == true => {
                                    return Ok(());
                                }
                                _ => {
                                    // Invalid
                                    return Err(Error::new(ErrorKind::InvalidValue(self.line_number), None));
                                }
                            }
                        },
                        None => {
                            // Value ended with a _ which is invalid
                            return Err(Error::new(ErrorKind::InvalidValue(self.line_number), None));
                        }
                    }
                }
                _ => {
                    // Error - invalid character in integer
                    return Err(Error::new(ErrorKind::InvalidValue(self.line_number), None));
                }
            },
            None => {
                // File ended while reading an integer, this is valid the end of the file denotes the end of the float
                self.set_value_end(data_to_parse.len());
                self.state = ParserState::FinishedFloat(true, false);
                // TODO: This could lead to problems like reading the next character of the sequence no longer returns a null
                // return Ok(self.build_integer_pair(data_to_parse)?);
                return Ok(());
            }
        }
    }

    fn process_read_string_state<'a>(
        &mut self,
        sequence: &mut std::iter::Enumerate<std::str::Chars>,
    ) -> Result<(), Error> {
        match sequence.next() {
            Some((index, char)) => match char {
                '"' => {
                    // End of the string
                    // We are now scanning for the end of line
                    println!("Found end of string at {}", index);
                    // self.state = ParserState::AfterValue;
                    self.set_value_end(index + self.position);
                    // *value_ends = index + self.position;
                    self.position += index + 1;
                    self.state = ParserState::FinishedString;
                    // return Ok(self.build_string_pair(data_to_parse)?);
                    return Ok(());
                }
                '\n' | '\r' => {
                    // End of line without ending the string - this is invalid to read but not to produce
                    // TODO: This is supported with literal strings
                    return Err(Error::new(ErrorKind::InvalidValue(self.line_number), None));
                }
                // TODO: Escaped characters are supported?
                // An escaped character necessitates copying the string to a new value since we need to convert the escapes into actual characters
                // TODO: Invalid characters inside a string are there any?? - any unsupported escaped characters
                _ => {
                    // A character - there are some characters that will be illegal
                    println!("Got {} as part of a string", char);
                    return Ok(());
                }
            },
            None => {
                // We have reached the end of the file
                // This is invalid a string is only valid if it is ended with a "
                return Err(Error::new(ErrorKind::InvalidValue(self.line_number), None));
            }
        }
    }

    fn build_bool_pair<'a>(
        &mut self,
        bool_value: bool,
        data_to_parse: &'a str,
    ) -> Result<TomlPair<'a>, Error> {
        let name = &data_to_parse[self.name_start()..self.name_end()];
        self.value_start();
        self.value_end();
        let toml_pair = TomlPair::new(name, TomlValue::Bool(bool_value));
        Ok(toml_pair)
    }

    fn build_integer_pair<'a>(&mut self, data_to_parse: &'a str) -> Result<TomlPair<'a>, Error> {
        let value = &data_to_parse[self.value_start()..self.value_end()];
        let name = &data_to_parse[self.name_start()..self.name_end()];
        // Positive case only
        let mut integer = 0;
        for char in value.chars() {
            // Here we are converting the sring to a base 10 u64

            let value = match char {
            
                '0'..='9' => char as i64 - '0' as i64,
                // An underscore is a no op
                '_' => continue,
                _ => unreachable!("Invalid character was found while building an integer pair, all validation should have been done while parsing the integer"),
            
            };
            integer *= 10;
            integer += value;
        }
        return Ok(TomlPair::new(name, TomlValue::Integer(integer)));
    }

    fn build_negative_integer_pair<'a>(&mut self, data_to_parse: &'a str) -> Result<TomlPair<'a>, Error> {
        let value = &data_to_parse[self.value_start()..self.value_end()];
        let name = &data_to_parse[self.name_start()..self.name_end()];
        // Negative case only
        let mut integer = 0;
        for char in value.chars() {
            // Here we are converting the sring to a base 10 u64

            let value = match char {
            
                '0'..='9' => char as i64 - '0' as i64,
                // An underscore is a no op
                '_' => continue,
                _ => unreachable!("Invalid character was found while building an integer pair, all validation should have been done while parsing the integer, character was {}", char),
            
            };
            // TODO: Check for integer overflow
            integer *= 10;
            integer -= value;
        }
        return Ok(TomlPair::new(name, TomlValue::Integer(integer)));
    }

    fn build_binary_integer_pair<'a>(&mut self, data_to_parse: &'a str) -> Result<TomlPair<'a>, Error> {
        let value = &data_to_parse[self.value_start()..self.value_end()];
        let name = &data_to_parse[self.name_start()..self.name_end()];
        // Positive case only
        let mut integer = 0;
        for char in value.chars() {
            // Here we are converting the sring to a base 10 u64

            let bin_value = match char {            
                '0'..='1' => char as i64 - '0' as i64,
                // An underscore is a no op
                '_' => continue,
                _ => unreachable!("Invalid character was found while building a binary integer pair, all validation should have been done while parsing the integer"),            
            };
            
            integer <<= 1;
            integer += bin_value;
        }
        return Ok(TomlPair::new(name, TomlValue::Integer(integer)));
    }

    fn build_hex_integer_pair<'a>(&mut self, data_to_parse: &'a str) -> Result<TomlPair<'a>, Error> {
        let value = &data_to_parse[self.value_start()..self.value_end()];
        let name = &data_to_parse[self.name_start()..self.name_end()];
        // Positive case only
        let mut integer = 0;
        for char in value.chars() {
            // Here we are converting the sring to a base 10 u64

            let hex_value = match char {            
                '0'..='9' => char as i64 - '0' as i64,
                'a'..='f' => char as i64 - 'a' as i64 + 10,
                'A'..='F' => char as i64 - 'A' as i64 + 10,
                // An underscore is a no op
                '_' => continue,
                _ => unreachable!("Invalid character was found while building a binary integer pair, all validation should have been done while parsing the integer"),            
            };
            
            integer *= 16;
            integer += hex_value;
        }
        return Ok(TomlPair::new(name, TomlValue::Integer(integer)));
    }

    fn build_octal_integer_pair<'a>(&mut self, data_to_parse: &'a str) -> Result<TomlPair<'a>, Error> {
        let value = &data_to_parse[self.value_start()..self.value_end()];
        let name = &data_to_parse[self.name_start()..self.name_end()];
        // Positive case only
        let mut integer = 0;
        for char in value.chars() {
            // Here we are converting the sring to a base 10 u64

            let octal_value = match char {            
                '0'..='7' => char as i64 - '0' as i64,
                // An underscore is a no op
                '_' => continue,
                _ => unreachable!("Invalid character was found while building a binary integer pair, all validation should have been done while parsing the integer"),            
            };
            
            integer *= 8;
            integer += octal_value;
        }
        return Ok(TomlPair::new(name, TomlValue::Integer(integer)));
    }

    fn build_float_pair<'a>(&mut self, data_to_parse: &'a str) -> Result<TomlPair<'a>, Error> {
        let value = &data_to_parse[self.value_start()..self.value_end()];
        let name = &data_to_parse[self.name_start()..self.name_end()];
        // Parsing a string to float is error prone and complex
        // TODO: Custom parsing allows us to avoid the string copy
        let mut copied_string = value.to_string();
        copied_string.retain(|c| c != '_');
        let float = match copied_string.parse::<f64>() {
            Ok(integer) => integer,
            Err(error) => {
                // This is somewhat unreachable since we will see the error before this point when reading the toml file
                return Err(Error::new(
                    ErrorKind::InvalidValue(self.line_number),
                    Some(Box::new(error)),
                ));
            }
        };
        return Ok(TomlPair::new(name, TomlValue::Float(float)));
    }

    fn build_string_pair<'a>(&mut self, data_to_parse: &'a str) -> Result<TomlPair<'a>, Error> {
        let value = &data_to_parse[self.value_start()..self.value_end()];
        let name = &data_to_parse[self.name_start()..self.name_end()];
        return Ok(TomlPair::new(name, TomlValue::String(value)));
    }

    fn process_reading_octal<'a>(
        &mut self,
        sequence: &mut std::iter::Enumerate<std::str::Chars>,
        data_to_parse: &'a str,
    ) -> Result<(), Error> {
        match sequence.next() {
            Some((index, char)) => match char {
                char if char.is_ascii_digit() == true => match char {
                    '8' | '9' => {
                        return Err(Error::new(ErrorKind::InvalidValue(self.line_number), None));
                    }
                    _ => return Ok(()),
                },
                ' ' | '\t' => {
                    // Whitespace means the integer ended.
                    // This means we are still looking for a new line but the integer is done
                    self.state = ParserState::FinishedOctalInteger(false, false);
                    self.set_value_end(index + self.position);
                    self.position += index;
                    // Moving to AfterValue state
                    // self.state = ParserState::AfterValue;
                    // return Ok(self.build_integer_pair(data_to_parse)?);
                    return Ok(());
                }
                '.' => {
                    // TODO: Floats are invalid here
                    unimplemented!("No support for floats");
                }
                '_' => {
                    // not a no op, read the next char to ensure it is a digit
                    return Ok(());
                    // TODO: Each underscore must be surrounded by at least one digit on each side
                    // TODO: This requirement seems flawed, it appears to be a way to help parsers but really just makes parsing more complex
                    // This is a no op, the string to integer functions are _ aware
                }
                '\n' => {
                    // End of integer
                    self.state = ParserState::FinishedOctalInteger(false, true);
                    // Our state transition when finishing a value is based on the character that was read during the transition to finish
                    // (state, previous_character)
                    self.line_number += 1;
                    self.set_value_end(index + self.position);
                    self.position += index;
                    // return Ok(self.build_integer_pair(data_to_parse)?);
                    return Ok(());
                }
                '\r' => {
                    // Next character must be \n and then return pair
                    if let Some((index, '\n')) = sequence.next() {
                        self.state = ParserState::FinishedOctalInteger(false, true);
                        self.line_number += 1;
                        // We reduce the length by one to account for the two byte eol sequence
                        self.set_value_end(index + self.position - 1);
                        self.position += index;
                        // We set the previous token so that when we move from finished state we move to normal rather than after value
                        // return Ok(self.build_integer_pair(data_to_parse)?);
                        return Ok(());
                        // Integer is done and we are moving to NewLine state
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
                self.set_value_end(data_to_parse.len());
                self.state = ParserState::FinishedOctalInteger(true, false);
                // TODO: This could lead to problems like reading the next character of the sequence no longer returns a null
                // return Ok(self.build_integer_pair(data_to_parse)?);
                return Ok(());
            }
        }
    }

    fn process_reading_binary<'a>(
        &mut self,
        sequence: &mut std::iter::Enumerate<std::str::Chars>,
        data_to_parse: &'a str,
    ) -> Result<(), Error> {
        match sequence.next() {
            Some((index, char)) => match char {
                char if char.is_ascii_digit() == true => match char {
                    '0' | '1' => {
                        return Ok(());
                    }
                    _ => return Err(Error::new(ErrorKind::InvalidValue(self.line_number), None)),
                },
                ' ' | '\t' => {
                    // Whitespace means the integer ended.
                    // This means we are still looking for a new line but the integer is done
                    self.state = ParserState::FinishedBinaryInteger(false, false);
                    self.set_value_end(index + self.position);
                    self.position += index;
                    // Moving to AfterValue state
                    // self.state = ParserState::AfterValue;
                    // return Ok(self.build_integer_pair(data_to_parse)?);
                    return Ok(());
                }
                '.' => {
                    unimplemented!("No support for floats");
                }
                '_' => {
                    // not a no op, read the next char to ensure it is a digit
                    return Ok(());
                    // TODO: Each underscore must be surrounded by at least one digit on each side
                    // TODO: This requirement seems flawed, it appears to be a way to help parsers but really just makes parsing more complex
                    // This is a no op, the string to integer functions are _ aware
                }
                '\n' => {
                    // End of integer
                    self.state = ParserState::FinishedBinaryInteger(false, true);
                    // Our state transition when finishing a value is based on the character that was read during the transition to finish
                    // (state, previous_character)
                    self.line_number += 1;
                    self.set_value_end(index + self.position);
                    self.position += index;
                    // return Ok(self.build_integer_pair(data_to_parse)?);
                    return Ok(());
                }
                '\r' => {
                    // Next character must be \n and then return pair
                    if let Some((index, '\n')) = sequence.next() {
                        self.state = ParserState::FinishedBinaryInteger(false, true);
                        self.line_number += 1;
                        // We reduce the length by one to account for the two byte eol sequence
                        self.set_value_end(index + self.position - 1);
                        self.position += index;
                        // We set the previous token so that when we move from finished state we move to normal rather than after value
                        // return Ok(self.build_integer_pair(data_to_parse)?);
                        return Ok(());
                        // Integer is done and we are moving to NewLine state
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
                self.set_value_end(data_to_parse.len());
                self.state = ParserState::FinishedBinaryInteger(true, false);
                // TODO: This could lead to problems like reading the next character of the sequence no longer returns a null
                // return Ok(self.build_integer_pair(data_to_parse)?);
                return Ok(());
            }
        }
    }

    fn process_reading_hex<'a>(
        &mut self,
        sequence: &mut std::iter::Enumerate<std::str::Chars>,
        data_to_parse: &'a str,
    ) -> Result<(), Error> {
        match sequence.next() {
            Some((index, char)) => match char {
                char if char.is_ascii_digit() == true => return Ok(()),
                'a' | 'b' | 'c' | 'd' | 'e' | 'f' => return Ok(()),
                'A' | 'B' | 'C' | 'D' | 'E' | 'F' => return Ok(()),
                ' ' | '\t' => {
                    // Whitespace means the integer ended.
                    // This means we are still looking for a new line but the integer is done
                    self.state = ParserState::FinishedHexInteger(false, false);
                    self.set_value_end(index + self.position);
                    self.position += index;
                    return Ok(());
                },
                '.' => {
                    unimplemented!("No support for floats");
                },
                '_' => {
                    // not a no op, read the next char to ensure it is a digit
                    return Ok(());
                    // TODO: Each underscore must be surrounded by at least one digit on each side
                    // TODO: This requirement seems flawed, it appears to be a way to help parsers but really just makes parsing more complex
                    // This is a no op, the string to integer functions are _ aware
                },
                '\n' => {
                    // End of integer
                    self.state = ParserState::FinishedHexInteger(false, true);
                    // Our state transition when finishing a value is based on the character that was read during the transition to finish
                    // (state, previous_character)
                    self.line_number += 1;
                    self.set_value_end(index + self.position);
                    self.position += index;
                    // return Ok(self.build_integer_pair(data_to_parse)?);
                    return Ok(());
                },
                '\r' => {
                    // Next character must be \n and then return pair
                    if let Some((index, '\n')) = sequence.next() {
                        self.state = ParserState::FinishedHexInteger(false, true);
                        self.line_number += 1;
                        // We reduce the length by one to account for the two byte eol sequence
                        self.set_value_end(index + self.position - 1);
                        self.position += index;
                        // We set the previous token so that when we move from finished state we move to normal rather than after value
                        // return Ok(self.build_integer_pair(data_to_parse)?);
                        return Ok(());
                        // Integer is done and we are moving to NewLine state
                    } else {
                        // Error - invalid integer because of invalid end of line
                        return Err(Error::new(
                            ErrorKind::InvalidEndOfLine(self.line_number),
                            None,
                        ));
                    }
                },
                _ => {
                    // Error - invalid character in integer
                    return Err(Error::new(ErrorKind::InvalidValue(self.line_number), None));
                }
            },
            None => {
                // File ended while reading an integer, this is valid the end of the file denotes the end of the integer
                self.set_value_end(data_to_parse.len());
                self.state = ParserState::FinishedHexInteger(true, false);
                return Ok(());
            }
        }
    }
}
