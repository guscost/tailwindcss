use super::arbitrary_value_machine::ArbitraryValueMachine;
use super::named_utility_machine::NamedUtilityMachine;
use crate::cursor;
use crate::extractor::machine::{Machine, MachineState};
use crate::extractor::modifier_machine::ModifierMachine;

#[derive(Debug, Default)]
pub(crate) struct VariantMachine {
    /// Start position of the variant
    start_pos: usize,

    /// Ignore the characters until this specific position
    skip_until_pos: Option<usize>,

    /// Current state of the machine
    state: State,

    arbitrary_value_machine: ArbitraryValueMachine,
    named_utility_machine: NamedUtilityMachine,
    modifier_machine: ModifierMachine,
}

#[derive(Debug, Default)]
enum State {
    #[default]
    Idle,

    /// Parsing a named variant
    ///
    /// E.g.:
    ///
    /// ```
    /// hover:
    /// ^^^^^
    /// ```
    ParsingNamedVariant,

    /// Parsing an arbitrary variant
    ///
    /// E.g.:
    ///
    /// ```
    /// [&:hover]:
    /// ^^^^^^^^^
    /// ```
    ParsingArbitraryVariant,

    /// Parsing a modifier
    ///
    /// E.g.:
    ///
    /// ```
    /// group-hover/name:
    ///            ^^^^^
    /// ```
    ///
    ParsingModifier,

    /// Parsing the end of a variant
    ///
    /// E.g.:
    ///
    /// ```
    /// hover:
    ///      ^
    /// ```
    ParseEnd,
}

impl Machine for VariantMachine {
    fn next(&mut self, cursor: &cursor::Cursor<'_>) -> MachineState {
        // Skipping characters until a specific position
        match self.skip_until_pos {
            Some(skip_until) if cursor.pos < skip_until => return MachineState::Parsing,
            Some(_) => self.skip_until_pos = None,
            None => {}
        }

        match self.state {
            State::Idle => match (cursor.curr, cursor.next) {
                // Start of an arbitrary variant
                //
                // E.g.: `[&:hover]:`
                //        ^
                (b'[', _) => {
                    self.start_pos = cursor.pos;
                    self.arbitrary_value_machine.next(cursor);
                    self.state = State::ParsingArbitraryVariant;
                    MachineState::Parsing
                }

                // Valid single character variant
                //
                // Must be followed by a `:`
                (b'a'..=b'z', b':') => {
                    self.start_pos = cursor.pos;
                    self.parse_end()
                }

                // Valid start characters for a named variant
                //
                // E.g.: `hover:`
                //        ^
                (b'-' | b'_' | b'a'..=b'z' | b'@', _) => {
                    self.start_pos = cursor.pos;
                    self.named_utility_machine.next(cursor);
                    self.state = State::ParsingNamedVariant;
                    MachineState::Parsing
                }

                // Everything else, is not a valid start of a variant.
                _ => MachineState::Idle,
            },

            State::ParsingNamedVariant => match self.named_utility_machine.next(cursor) {
                MachineState::Idle => self.restart(),
                MachineState::Parsing => MachineState::Parsing,
                MachineState::Done(_) => match cursor.next {
                    // Named variant can be followed by a modifier
                    //
                    // E.g.:
                    //
                    // ```
                    // group-hover/foo:
                    //            ^
                    // ```
                    b'/' => self.parse_modifier(),

                    // Named variant must be followed by a `:`
                    //
                    // E.g.:
                    //
                    // ```
                    // hover:
                    //      ^
                    // ```
                    b':' => self.parse_end(),

                    // Everything else is invalid
                    _ => self.restart(),
                },
            },

            State::ParsingArbitraryVariant => match self.arbitrary_value_machine.next(cursor) {
                MachineState::Idle => self.restart(),
                MachineState::Parsing => MachineState::Parsing,
                MachineState::Done(_) => match cursor.next {
                    // End of an arbitrary variant, must be followed by a `:`
                    //
                    // E.g.: `[&:hover]:`
                    //                 ^
                    b':' => self.parse_end(),

                    // Everything else is invalid
                    _ => self.restart(),
                },
            },

            State::ParsingModifier => match self.modifier_machine.next(cursor) {
                MachineState::Idle => self.restart(),
                MachineState::Parsing => MachineState::Parsing,
                MachineState::Done(_) => match cursor.next {
                    // Modifier must be followed by a `:`
                    //
                    // E.g.: `group-hover/name:`
                    //                        ^
                    b':' => self.parse_end(),

                    // Everything else is invalid
                    _ => self.restart(),
                },
            },

            State::ParseEnd => match cursor.curr {
                // The end of a variant must be the `:`
                //
                // E.g.: `hover:`
                //             ^
                b':' => self.done(self.start_pos, cursor),

                // Everything else is invalid
                _ => self.restart(),
            },
        }
    }
}

impl VariantMachine {
    #[inline(always)]
    fn parse_modifier(&mut self) -> MachineState {
        self.state = State::ParsingModifier;
        MachineState::Parsing
    }

    #[inline(always)]
    fn parse_end(&mut self) -> MachineState {
        self.state = State::ParseEnd;
        MachineState::Parsing
    }
}

#[cfg(test)]
mod tests {
    use super::VariantMachine;
    use crate::cursor::Cursor;
    use crate::extractor::machine::{Machine, MachineState};

    #[test]
    fn test_variant_extraction() {
        for (input, expected) in [
            // Simple variant
            ("hover:flex", vec!["hover:"]),
            // With dashes
            ("data-disabled:flex", vec!["data-disabled:"]),
            // Multiple variants
            ("hover:focus:flex", vec!["hover:", "focus:"]),
            // Arbitrary variant
            ("[&:hover]:flex", vec!["[&:hover]:"]),
            // Modifiers
            ("group-hover/foo:flex", vec!["group-hover/foo:"]),
            ("group-hover/[.parent]:flex", vec!["group-hover/[.parent]:"]),
            // Arbitrary variant with bracket notation
            ("data-[state=pending]:flex", vec!["data-[state=pending]:"]),
            // Arbitrary variant with CSS property shorthand
            ("supports-(--my-color):flex", vec!["supports-(--my-color):"]),
            // -------------------------------------------------------------

            // Exceptions
            // Empty arbitrary variant is not allowed
            ("[]:flex", vec![]),
            // Named variant must be followed by `:`
            ("hover", vec![]),
            // Modifier cannot be followed by another modifier. However, we don't check boundary
            // characters in this state machine so we will get `bar:`.
            ("group-hover/foo/bar:flex", vec!["bar:"]),
        ] {
            let mut machine = VariantMachine::default();
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
