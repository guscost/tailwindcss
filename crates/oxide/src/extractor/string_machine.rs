use crate::cursor;
use crate::extractor::machine::{Machine, MachineState};

#[derive(Debug, Default)]
pub(crate) struct StringMachine {
    /// Start position of the string
    start_pos: usize,

    /// The expected end character of the string
    ///
    /// E.g.: " or ' or `
    end_char: u8,

    /// Ignore the characters until this specific position
    skip_until_pos: Option<usize>,

    /// Current state of the machine
    state: State,
}

#[derive(Debug, Default)]
enum State {
    #[default]
    Idle,

    /// Parsing a string
    Parsing,
}

impl Machine for StringMachine {
    fn next(&mut self, cursor: &cursor::Cursor<'_>) -> MachineState {
        // Skipping characters until a specific position
        match self.skip_until_pos {
            Some(skip_until) if cursor.pos < skip_until => return MachineState::Parsing,
            Some(_) => self.skip_until_pos = None,
            None => {}
        }

        match self.state {
            State::Idle => match cursor.curr {
                // Start of a string
                b'"' | b'\'' | b'`' => {
                    self.start_pos = cursor.pos;
                    self.end_char = cursor.curr;
                    self.state = State::Parsing;
                    MachineState::Parsing
                }

                // Anything else is not a valid start of a string
                _ => MachineState::Idle,
            },

            State::Parsing => match cursor.curr {
                // An escaped character, skip ahead to the next character
                b'\\' if !cursor.at_end => {
                    self.skip_until_pos = Some(cursor.pos + 2);
                    MachineState::Parsing
                }

                // An escaped whitespace character is not allowed
                b'\\' if cursor.next.is_ascii_whitespace() => self.restart(),

                // End of the string
                x if x == self.end_char => self.done(self.start_pos, cursor),

                // Any kind of whitespace is not allowed
                x if x.is_ascii_whitespace() => self.restart(),

                // Everything else is valid
                _ => MachineState::Parsing,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::StringMachine;
    use crate::cursor::Cursor;
    use crate::extractor::machine::{Machine, MachineState};

    #[test]
    fn test_string_value_extraction() {
        for (input, expected) in [
            // Simple string
            ("'foo'", vec!["'foo'"]),
            // String as part of a candidate
            ("content-['hello_world']", vec!["'hello_world'"]),
            // With nested quotes
            (r#"'"`hello`"'"#, vec![r#"'"`hello`"'"#]),
            // Spaces are not allowed
            ("' hello world '", vec![]),
        ] {
            let mut machine = StringMachine::default();
            let mut cursor = Cursor::new(input.as_bytes());

            let mut actual: Vec<&str> = vec![];

            for i in 0..input.len() {
                cursor.move_to(i);

                if let MachineState::Done(span) = machine.next(&cursor) {
                    actual.push(unsafe { std::str::from_utf8_unchecked(span.slice(cursor.input)) });
                }
            }

            assert_eq!(actual, expected);
        }
    }
}
