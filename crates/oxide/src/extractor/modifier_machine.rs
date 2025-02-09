use crate::cursor;
use crate::extractor::arbitrary_value_machine::ArbitraryValueMachine;
use crate::extractor::machine::{Machine, MachineState};
use crate::extractor::CssVariableMachine;

#[derive(Debug, Default)]
pub(crate) struct ModifierMachine {
    /// Start position of the modifier
    start_pos: usize,

    /// Ignore the characters until this specific position
    skip_until_pos: Option<usize>,

    /// Current state of the machine
    state: State,

    arbitrary_value_machine: ArbitraryValueMachine,
    css_variable_machine: CssVariableMachine,
}

#[derive(Debug, Default)]
enum State {
    #[default]
    Idle,

    /// Parsing a named modifier, e.g.:
    ///
    /// ```
    /// bg-red-500/20
    ///           ^^^
    /// ```
    ParsingNamed,

    /// Parsing an arbitrary value, e.g.:
    ///
    /// ```
    /// bg-red-500/[20%]
    ///           ^^^^^^
    /// ```
    ParsingArbitraryValue,

    /// Parsing an arbitrary variable, e.g.:
    ///
    /// ```
    /// bg-red-500/(--my-opacity)
    ///           ^^^^^^^^^^^^^^^
    /// ```
    ParsingArbitraryVariable(ArbitraryVariableStage),
}

#[derive(Debug)]
enum ArbitraryVariableStage {
    /// Currently parsing the inside of the arbitrary variable
    ///
    /// ```
    /// bg-red-500/(--my-opacity)
    ///             ^^^^^^^^^^^^
    /// ```
    Inside,

    /// Currently parsing the end of the arbitrary variable
    ///
    /// ```
    /// bg-red-500/(--my-opacity)
    ///                         ^
    /// ```
    End,
}

impl Machine for ModifierMachine {
    fn next(&mut self, cursor: &cursor::Cursor<'_>) -> MachineState {
        // Skipping characters until a specific position
        match self.skip_until_pos {
            Some(skip_until) if cursor.pos < skip_until => return MachineState::Parsing,
            Some(_) => self.skip_until_pos = None,
            None => {}
        }

        match self.state {
            State::Idle => match (cursor.curr, cursor.next) {
                // Start of an arbitrary value
                (b'/', b'[') => {
                    self.start_pos = cursor.pos;
                    self.state = State::ParsingArbitraryValue;
                    MachineState::Parsing
                }

                // Start of an arbitrary variable
                (b'/', b'(') => {
                    self.start_pos = cursor.pos;
                    self.skip_until_pos = Some(cursor.pos + 2);
                    self.state = State::ParsingArbitraryVariable(ArbitraryVariableStage::Inside);
                    MachineState::Parsing
                }

                // Start of a named modifier
                (b'/', b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9') => {
                    self.start_pos = cursor.pos;
                    self.state = State::ParsingNamed;
                    MachineState::Parsing
                }

                // Anything else is not a valid start of a modifier
                _ => MachineState::Idle,
            },

            State::ParsingNamed => match (cursor.curr, cursor.next) {
                // Only valid characters are allowed, if followed by another valid character
                (
                    b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.',
                    b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.',
                ) => MachineState::Parsing,

                // Valid character, but at the end of the modifier
                (b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.', _) => {
                    self.done(self.start_pos, cursor)
                }

                // Anything else is invalid, end of the modifier
                _ => self.restart(),
            },

            State::ParsingArbitraryValue => match self.arbitrary_value_machine.next(cursor) {
                MachineState::Idle => self.restart(),
                MachineState::Parsing => MachineState::Parsing,
                MachineState::Done(_) => self.done(self.start_pos, cursor),
            },

            State::ParsingArbitraryVariable(ArbitraryVariableStage::Inside) => {
                match self.css_variable_machine.next(cursor) {
                    MachineState::Idle => self.restart(),
                    MachineState::Parsing => MachineState::Parsing,
                    MachineState::Done(_) => self.parse_arbitrary_variable_end(),
                }
            }

            State::ParsingArbitraryVariable(ArbitraryVariableStage::End) => match cursor.curr {
                // End of an arbitrary variable must be followed by `)`
                b')' => self.done(self.start_pos, cursor),

                // Invalid modifier, not ending at `)`
                _ => self.restart(),
            },
        }
    }
}

impl ModifierMachine {
    #[inline(always)]
    fn parse_arbitrary_variable_end(&mut self) -> MachineState {
        self.state = State::ParsingArbitraryVariable(ArbitraryVariableStage::End);
        MachineState::Parsing
    }
}

#[cfg(test)]
mod tests {
    use super::ModifierMachine;
    use crate::cursor::Cursor;
    use crate::extractor::machine::{Machine, MachineState};

    #[test]
    fn test_modifier_extraction() {
        for (input, expected) in [
            // Simple modifier
            ("foo/bar", vec!["/bar"]),
            ("foo/bar-baz", vec!["/bar-baz"]),
            // Simple modifier with numbers
            ("foo/20", vec!["/20"]),
            // Simple modifier with numbers
            ("foo/20", vec!["/20"]),
            // Arbitrary value
            ("foo/[20]", vec!["/[20]"]),
            // Arbitrary value with CSS variable shorthand
            ("foo/(--x)", vec!["/(--x)"]),
            ("foo/(--foo-bar)", vec!["/(--foo-bar)"]),
            // --------------------------------------------------------

            // Empty arbitrary value is not allowed
            ("foo/[]", vec![]),
            // Empty arbitrary value shorthand is not allowed
            ("foo/()", vec![]),
            // A CSS variable must start with `--` and must have at least a single character
            ("foo/(-)", vec![]),
            ("foo/(--)", vec![]),
            // Arbitrary value shorthand should be a valid CSS variable
            ("foo/(--my#color)", vec![]),
        ] {
            let mut machine = ModifierMachine::default();
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
