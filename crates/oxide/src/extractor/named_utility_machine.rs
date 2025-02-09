use crate::cursor;
use crate::extractor::arbitrary_value_machine::ArbitraryValueMachine;
use crate::extractor::css_variable_machine::CssVariableMachine;
use crate::extractor::machine::{Machine, MachineState};

#[derive(Debug, Default)]
pub(crate) struct NamedUtilityMachine {
    /// Start position of the utility
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

    /// Parsing a utility
    Parsing,

    /// Parsing a functional utility with an arbitrary value
    ///
    /// E.g.: `text-[…]`.
    ParsingArbitraryValue,

    /// Parsing a functional utility with an arbitrary variable
    ///
    /// E.g.: `text-(--my-color)`.
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

impl Machine for NamedUtilityMachine {
    fn next(&mut self, cursor: &cursor::Cursor<'_>) -> MachineState {
        // Skipping characters until a specific position
        match self.skip_until_pos {
            Some(skip_until) if cursor.pos < skip_until => return MachineState::Parsing,
            Some(_) => self.skip_until_pos = None,
            None => {}
        }

        match self.state {
            State::Idle => match (cursor.curr, cursor.next) {
                // Utilities don't start with `--`
                //
                // E.g.: `--my-color`
                //        ^^
                (b'-', b'-') => self.restart(),

                // Utilities don't start with `/`
                //
                // E.g.: `</div`
                //         ^
                (b'/', _) => self.restart(),

                // Utilities don't start with `<`
                //
                // E.g.: `<div`
                //        ^
                (b'<', _) => self.restart(),

                // Valid single character utility
                //
                // Must be followed by a space or the end of the input
                //
                // E.g.: `<div class="a"></div>`
                //                    ^
                (b'a'..=b'z', x) if x.is_ascii_whitespace() || cursor.at_end => {
                    self.start_pos = cursor.pos;
                    self.done(cursor.pos, cursor)
                }

                // Valid start characters
                //
                // E.g.: `flex`
                //        ^
                // E.g.: `@container`
                //        ^
                (b'a'..=b'z' | b'@', _) => {
                    self.start_pos = cursor.pos;
                    self.state = State::Parsing;
                    MachineState::Parsing
                }

                // Valid start of a negative utility, if followed by another set of valid
                // characters. `@` as a second character is invalid.
                //
                // E.g.: `-mx-2.5`
                //        ^^
                (b'-', b'a'..=b'z' | b'A'..=b'Z') => {
                    self.start_pos = cursor.pos;
                    self.state = State::Parsing;
                    MachineState::Parsing
                }

                // Everything else, is not a valid start of the utility. But the next character
                // might be a valid start for a new utility.
                _ => MachineState::Idle,
            },

            State::Parsing => match (cursor.curr, cursor.next) {
                // Arbitrary value with bracket notation. `-` followed by `[`.
                (b'-', b'[') => {
                    self.state = State::ParsingArbitraryValue;
                    MachineState::Parsing
                }

                // Arbitrary value with CSS variable shorthand. `-` followed by `(`.
                (b'-', b'(') => {
                    // Arbitrary variable will only check inside of the `(…)`, start parsing until
                    // we are inside of the parens.
                    self.skip_until_pos = Some(cursor.pos + 2);
                    self.state = State::ParsingArbitraryVariable(ArbitraryVariableStage::Inside);
                    MachineState::Parsing
                }

                // Valid characters if followed by another valid character. These characters are
                // only valid inside of the utility but not at the end.
                //
                // E.g.: `flex-`
                //            ^
                // E.g.: `flex-!`
                //            ^
                // E.g.: `flex-/`
                //            ^
                (b'-', b'-' | b'_' | b'.' | b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9') => {
                    MachineState::Parsing
                }

                // Valid characters inside of a utility:
                //
                // At the end, we can stop parsing
                //
                // E.g.: `flex`
                //           ^
                (b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9', _) if cursor.at_end => {
                    self.done(self.start_pos, cursor)
                }

                // Followed by a character that is not going to be valid
                (b'_' | b'.' | b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9', next) if !matches!(next, b'-' | b'_' | b'.' | b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9') => {
                    self.done(self.start_pos, cursor)
                }

                // Still valid, but not at the end yet
                (b'_' | b'.' | b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9', _) => MachineState::Parsing,

                // Everything else is invalid
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

impl NamedUtilityMachine {
    #[inline(always)]
    fn parse_arbitrary_variable_end(&mut self) -> MachineState {
        self.state = State::ParsingArbitraryVariable(ArbitraryVariableStage::End);
        MachineState::Parsing
    }
}

#[cfg(test)]
mod tests {
    use super::NamedUtilityMachine;
    use crate::cursor::Cursor;
    use crate::extractor::machine::{Machine, MachineState};

    #[test]
    fn test_named_utility_extraction() {
        for (input, expected) in [
            // Simple utility
            ("flex", vec!["flex"]),
            // Simple single-character utility
            ("a", vec!["a"]),
            // With dashes
            ("items-center", vec!["items-center"]),
            // With numbers
            ("px-5", vec!["px-5"]),
            ("px-2.5", vec!["px-2.5"]),
            // Arbitrary value with bracket notation
            ("bg-[#0088cc]", vec!["bg-[#0088cc]"]),
            // Arbitrary variable
            ("bg-(--my-color)", vec!["bg-(--my-color)"]),
            // --------------------------------------------------------

            // Exceptions:
            // Arbitrary variable must be valid
            (r"bg-(--my-color\)", vec![]),
            // We get `color`, because we don't check boundaries as part of this state machine.
            (r"bg-(--my#color)", vec!["color"]),
            // Single letter utility with uppercase letter is invalid
            ("A", vec![]),
            // Spaces do not count
            (" a", vec!["a"]),
            ("a ", vec!["a"]),
            (" a ", vec!["a"]),
            (" flex", vec!["flex"]),
            ("flex ", vec!["flex"]),
            (" flex ", vec!["flex"]),
            // Random invalid utilities
            ("-$", vec![]),
            ("-_", vec![]),
            ("-foo-", vec![]),
            ("foo-=", vec![]),
            ("foo-#", vec![]),
            ("foo-!", vec![]),
            ("foo-/20", vec![]),
            ("-", vec![]),
            ("--", vec![]),
            ("---", vec![]),
        ] {
            let mut machine = NamedUtilityMachine::default();
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
