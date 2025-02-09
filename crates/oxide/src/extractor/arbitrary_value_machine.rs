use crate::cursor;
use crate::extractor::machine::{Machine, MachineState};
use crate::extractor::string_machine::StringMachine;

#[derive(Debug, Default)]
pub(crate) struct ArbitraryValueMachine {
    /// Start position of the arbitrary value
    start_pos: usize,

    /// Bracket stack to ensure properly balanced brackets
    bracket_stack: Vec<u8>,

    /// Ignore the characters until this specific position
    skip_until_pos: Option<usize>,

    /// Current state of the machine
    state: State,

    string_machine: StringMachine,
}

#[derive(Debug, Default)]
enum State {
    #[default]
    Idle,

    /// Parsing an arbitrary value
    Parsing,

    /// Parsing a string, in this case the brackets don't need to be balanced when inside of a
    /// string.
    ParsingString,
}

impl Machine for ArbitraryValueMachine {
    fn next(&mut self, cursor: &cursor::Cursor<'_>) -> MachineState {
        // Skipping characters until a specific position
        match self.skip_until_pos {
            Some(skip_until) if cursor.pos < skip_until => return MachineState::Parsing,
            Some(_) => self.skip_until_pos = None,
            None => {}
        }

        match self.state {
            State::Idle => match cursor.curr {
                // Start of an arbitrary value
                b'[' => {
                    self.start_pos = cursor.pos;
                    self.state = State::Parsing;
                    MachineState::Parsing
                }

                // Anything else is not a valid start of an arbitrary value
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

                b'(' => {
                    self.bracket_stack.push(b')');
                    MachineState::Parsing
                }

                b'[' => {
                    self.bracket_stack.push(b']');
                    MachineState::Parsing
                }

                b'{' => {
                    self.bracket_stack.push(b'}');
                    MachineState::Parsing
                }

                b')' | b']' | b'}' if !self.bracket_stack.is_empty() => {
                    if let Some(&expected) = self.bracket_stack.last() {
                        if cursor.curr == expected {
                            self.bracket_stack.pop();
                        } else {
                            return self.restart();
                        }
                    }

                    MachineState::Parsing
                }

                // End of an arbitrary value
                // 1. All brackets must be balanced
                // 2. There must be at least a single character inside the brackets
                b']' if self.bracket_stack.is_empty() && self.start_pos + 1 != cursor.pos => {
                    self.done(self.start_pos, cursor)
                }

                // Start of a string
                b'"' | b'\'' | b'`' => {
                    self.string_machine.next(cursor);
                    self.state = State::ParsingString;
                    MachineState::Parsing
                }

                // Any kind of whitespace is not allowed
                x if x.is_ascii_whitespace() => self.restart(),

                // Everything else is valid
                _ => MachineState::Parsing,
            },

            State::ParsingString => match self.string_machine.next(cursor) {
                MachineState::Idle => self.restart(),
                MachineState::Parsing => MachineState::Parsing,
                MachineState::Done(_) => {
                    self.state = State::Parsing;
                    MachineState::Parsing
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ArbitraryValueMachine;
    use crate::cursor::Cursor;
    use crate::extractor::machine::{Machine, MachineState};

    #[test]
    fn test_arbitrary_value_extraction() {
        for (input, expected) in [
            // Simple variable
            ("[#0088cc]", vec!["[#0088cc]"]),
            // With parentheses
            (
                "[url(https://tailwindcss.com)]",
                vec!["[url(https://tailwindcss.com)]"],
            ),
            // With strings, where bracket balancing doesn't matter
            ("['[({])}']", vec!["['[({])}']"]),
            // With strings later in the input
            (
                "[url('https://tailwindcss.com?[{]}')]",
                vec!["[url('https://tailwindcss.com?[{]}')]"],
            ),
            // With nested brackets
            ("[[data-foo]]", vec!["[[data-foo]]"]),
            // Spaces are not allowed
            ("[ #0088cc ]", vec![]),
            // Unbalanced brackets are not allowed
            ("[foo[bar]", vec![]),
            // Empty brackets are not allowed
            ("[]", vec![]),
        ] {
            let mut machine = ArbitraryValueMachine::default();
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
