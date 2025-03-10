// See: - https://docs.ruby-lang.org/en/3.4/syntax/literals_rdoc.html#label-Percent+Literals
//      - https://docs.ruby-lang.org/en/3.4/syntax/literals_rdoc.html#label-25w+and+-25W-3A+String-Array+Literals
use crate::cursor;
use crate::extractor::bracket_stack;
use crate::extractor::pre_processors::pre_processor::PreProcessor;

#[derive(Debug, Default)]
pub struct Ruby;

impl PreProcessor for Ruby {
    fn process(&self, content: &[u8]) -> Vec<u8> {
        let len = content.len();
        let mut result = content.to_vec();
        let mut cursor = cursor::Cursor::new(content);
        let mut bracket_stack = bracket_stack::BracketStack::default();

        while cursor.pos < len {
            // Looking for `%w` or `%W`
            if cursor.curr != b'%' && !matches!(cursor.next, b'w' | b'W') {
                cursor.advance();
                continue;
            }

            cursor.advance_twice();

            // Boundary character
            let boundary = match cursor.curr {
                b'[' => b']',
                b'(' => b')',
                b'{' => b'}',
                _ => {
                    cursor.advance();
                    continue;
                }
            };

            bracket_stack.reset();

            // Replace the current character with a space
            result[cursor.pos] = b' ';

            // Skip the boundary character
            cursor.advance();

            while cursor.pos < len {
                match cursor.curr {
                    // Skip escaped characters
                    b'\\' => {
                        // Use backslash to embed spaces in the strings.
                        if cursor.next == b' ' {
                            result[cursor.pos] = b' ';
                        }

                        cursor.advance();
                    }

                    // Start of a nested bracket
                    b'[' | b'(' | b'{' => {
                        bracket_stack.push(cursor.curr);
                    }

                    // End of a nested bracket
                    b']' | b')' | b'}' if !bracket_stack.is_empty() => {
                        if !bracket_stack.pop(cursor.curr) {
                            // Unbalanced
                            cursor.advance();
                        }
                    }

                    // End of the pattern, replace the boundary character with a space
                    _ if cursor.curr == boundary => {
                        result[cursor.pos] = b' ';
                        break;
                    }

                    // Everything else is valid
                    _ => {}
                }

                cursor.advance();
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::Ruby;
    use crate::extractor::pre_processors::pre_processor::PreProcessor;

    #[test]
    fn test_ruby_pre_processor() {
        for (input, expected) in [
            // %w[…]
            ("%w[flex px-2.5]", "%w flex px-2.5 "),
            (
                "%w[flex data-[state=pending]:bg-[#0088cc] flex-col]",
                "%w flex data-[state=pending]:bg-[#0088cc] flex-col ",
            ),
            // %w{…}
            ("%w{flex px-2.5}", "%w flex px-2.5 "),
            (
                "%w{flex data-[state=pending]:bg-(--my-color) flex-col}",
                "%w flex data-[state=pending]:bg-(--my-color) flex-col ",
            ),
            // %w(…)
            ("%w(flex px-2.5)", "%w flex px-2.5 "),
            (
                "%w(flex data-[state=pending]:bg-(--my-color) flex-col)",
                "%w flex data-[state=pending]:bg-(--my-color) flex-col ",
            ),
            // Use backslash to embed spaces in the strings.
            (r#"%w[foo\ bar baz\ bat]"#, r#"%w foo  bar baz  bat "#),
            (r#"%W[foo\ bar baz\ bat]"#, r#"%W foo  bar baz  bat "#),
            // The nested delimiters evaluated to a flat array of strings
            // (not nested array).
            (r#"%w[foo[bar baz]qux]"#, r#"%w foo[bar baz]qux "#),
        ] {
            Ruby::test(input, expected);
        }
    }

    #[test]
    fn test_ruby_extraction() {
        for (input, expected) in [
            // %w[…]
            ("%w[flex px-2.5]", vec!["flex", "px-2.5"]),
            ("%w[px-2.5 flex]", vec!["flex", "px-2.5"]),
            ("%w[2xl:flex]", vec!["2xl:flex"]),
            (
                "%w[flex data-[state=pending]:bg-[#0088cc] flex-col]",
                vec!["flex", "data-[state=pending]:bg-[#0088cc]", "flex-col"],
            ),
            // %w{…}
            ("%w{flex px-2.5}", vec!["flex", "px-2.5"]),
            ("%w{px-2.5 flex}", vec!["flex", "px-2.5"]),
            ("%w{2xl:flex}", vec!["2xl:flex"]),
            (
                "%w{flex data-[state=pending]:bg-(--my-color) flex-col}",
                vec!["flex", "data-[state=pending]:bg-(--my-color)", "flex-col"],
            ),
            // %w(…)
            ("%w(flex px-2.5)", vec!["flex", "px-2.5"]),
            ("%w(px-2.5 flex)", vec!["flex", "px-2.5"]),
            ("%w(2xl:flex)", vec!["2xl:flex"]),
            (
                "%w(flex data-[state=pending]:bg-(--my-color) flex-col)",
                vec!["flex", "data-[state=pending]:bg-(--my-color)", "flex-col"],
            ),
        ] {
            Ruby::test_extract_contains(input, expected);
        }
    }
}
