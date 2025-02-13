use crate::cursor;
use crate::extractor::machine::{Machine, MachineState};
use crate::extractor::Span;

#[derive(Clone, Copy)]
enum Class {
    Quote,
    Escape,
    Whitespace,
    End,
    Other,
}

const fn generate_table() -> [Class; 256] {
    let mut table = [Class::Other; 256];

    table[b'"' as usize] = Class::Quote;
    table[b'\'' as usize] = Class::Quote;
    table[b'`' as usize] = Class::Quote;

    table[b'\\' as usize] = Class::Escape;

    table[b' ' as usize] = Class::Whitespace;
    table[b'\t' as usize] = Class::Whitespace;
    table[b'\n' as usize] = Class::Whitespace;
    table[b'\r' as usize] = Class::Whitespace;
    table[b'\x0C' as usize] = Class::Whitespace;

    table[0x00 as usize] = Class::End;

    return table;
}

const CLASS_TABLE: [Class; 256] = generate_table();

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

            State::Parsing => match (cursor.curr, cursor.next) {
                // An escaped character, skip ahead to the next character
                (b'\\', _) if !cursor.at_end => {
                    self.skip_until_pos = Some(cursor.pos + 2);
                    MachineState::Parsing
                }

                // An escaped whitespace character is not allowed
                (b'\\', b'\t' | b' ') => self.restart(),

                // End of the string
                (x, _) if x == self.end_char => self.done(self.start_pos, cursor),

                // Any kind of whitespace is not allowed
                (b'\t' | b' ', _) => self.restart(),

                // Everything else is valid
                _ => MachineState::Parsing,
            },
        }
    }
}

impl StringMachine {
    fn next_different(&mut self, pos: usize, prev: u8, curr: u8, next: u8) -> MachineState {
        let class_prev = CLASS_TABLE[prev as usize];
        let class_curr = CLASS_TABLE[curr as usize];
        let class_next = CLASS_TABLE[next as usize];

        // Skipping characters until a specific position
        match self.skip_until_pos {
            Some(skip_until) if pos < skip_until => return MachineState::Parsing,
            Some(_) => self.skip_until_pos = None,
            None => {}
        }

        match self.state {
            State::Idle => match class_curr {
                // Start of a string
                Class::Quote => {
                    self.start_pos = pos;
                    self.end_char = curr;
                    self.state = State::Parsing;
                    MachineState::Parsing
                }

                // Anything else is not a valid start of a string
                _ => MachineState::Idle,
            },

            State::Parsing => match (class_curr, class_next) {
                // An escaped character, skip ahead to the next character
                (Class::Escape, x) if !matches!(x, Class::End) => {
                    self.skip_until_pos = Some(pos + 2);
                    MachineState::Parsing
                }

                // An escaped whitespace character is not allowed
                (Class::Escape, Class::Whitespace) => self.restart(),

                // End of the string
                // TODO: Ensure this is the correct quote
                (Class::Quote, _) => {
                    self.reset();
                    MachineState::Done(Span::new(self.start_pos, pos))
                }

                // Any kind of whitespace is not allowed
                (Class::Whitespace, _) => self.restart(),

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
    use crate::throughput::Throughput;
    use std::hint::black_box;

    #[test]
    fn test_string_value_throughput() {
        let iterations = 100_000;
        let input = "There will be a 'string' in this input ".repeat(100);
        let input = input.as_bytes();
        let len = input.len();

        let mut input_pre_computed = vec![];
        let mut prev = 0x00;
        for pos in 0..len {
            let curr = input[pos];
            let next = if pos + 1 < len { input[pos + 1] } else { 0x00 };

            input_pre_computed.push((pos, prev, curr, next));

            prev = curr;
        }

        let throughput = Throughput::compute(iterations, len, || {
            let mut machine = StringMachine::default();

            for (pos, prev, curr, next) in &input_pre_computed {
                _ = black_box(machine.next_different(*pos, *prev, *curr, *next));
            }
        });
        eprintln!("String value machine throughput: {:}", throughput);
        assert!(false);
    }

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
