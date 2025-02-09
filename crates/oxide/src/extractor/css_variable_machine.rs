use crate::cursor;
use crate::extractor::machine::{Machine, MachineState};

#[derive(Debug, Default)]
pub(crate) struct CssVariableMachine {
    /// Start position of the CSS variable
    start_pos: usize,

    /// Ignore the characters until this specific position
    skip_until_pos: Option<usize>,

    /// Current state of the machine
    state: State,
}

#[derive(Debug, Default)]
enum State {
    #[default]
    Idle,

    /// Parsing a CSS variable
    Parsing,
}

impl Machine for CssVariableMachine {
    fn next(&mut self, cursor: &cursor::Cursor<'_>) -> MachineState {
        // Skipping characters until a specific position
        match self.skip_until_pos {
            Some(skip_until) if cursor.pos < skip_until => return MachineState::Parsing,
            Some(_) => self.skip_until_pos = None,
            None => {}
        }

        match self.state {
            State::Idle => match (cursor.curr, cursor.next) {
                // Start of a CSS variable
                (b'-', b'-') => {
                    self.start_pos = cursor.pos;
                    self.skip_until_pos = Some(cursor.pos + 2);
                    self.state = State::Parsing;
                    MachineState::Parsing
                }

                // Anything else is not a valid start of a CSS variable
                _ => MachineState::Idle,
            },

            State::Parsing => match (cursor.curr, cursor.next) {
                // https://drafts.csswg.org/css-syntax-3/#ident-token-diagram
                //
                // Valid character at the end of the input
                (b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_', 0x00) => {
                    self.done(self.start_pos, cursor)
                }

                // Valid character followed by a valid character or an escape character
                //
                // E.g.: `--my-variable`
                //                ^^
                // E.g.: `--my-\#variable`
                //            ^^
                (
                    b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_',
                    b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'\\',
                ) => MachineState::Parsing,

                // Valid character followed by an invalid character
                (b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_', _) => {
                    self.done(self.start_pos, cursor)
                }

                // An escaped whitespace character is not allowed
                //
                // In CSS it is allowed, but in the context of a class it's not because then we
                // would have spaces in the class. E.g.: `bg-(--my-\ color)`
                (b'\\', x) if x.is_ascii_whitespace() => self.restart(),

                // An escaped character, skip ahead to the next character
                (b'\\', _) if !cursor.at_end => {
                    self.skip_until_pos = Some(cursor.pos + 2);
                    MachineState::Parsing
                }

                // Character is not valid anymore
                _ => self.restart(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CssVariableMachine;
    use crate::cursor::Cursor;
    use crate::extractor::machine::{Machine, MachineState};

    #[test]
    fn test_css_variable_extraction() {
        for (input, expected) in [
            // Simple variable
            ("--foo", vec!["--foo"]),
            // With dashes
            ("--my-variable", vec!["--my-variable"]),
            // Inside `var(…)`
            ("var(--my-variable)", vec!["--my-variable"]),
            // Inside a string
            ("'--my-variable'", vec!["--my-variable"]),
            // Multiple variables
            (
                "calc(var(--first) + var(--second))",
                vec!["--first", "--second"],
            ),
            // Escaped character in the middle, skips the next character
            (r#"--spacing-1\/2"#, vec![r#"--spacing-1\/2"#]),
            // Escaped whitespace is not allowed
            (r#"--my-\ variable"#, vec![]),
        ] {
            let mut machine = CssVariableMachine::default();
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
