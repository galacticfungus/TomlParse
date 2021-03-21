use super::{TomlPair, TomlValue};

impl<'a> TomlPair<'a> {
    pub fn new(name: &'a str, value: TomlValue<'a>) -> TomlPair<'a> {
        TomlPair { name, value }
    }
}