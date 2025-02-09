use crate::cursor;
use crate::extractor::machine::Span;
use candidate_machine::CandidateMachine;
use css_variable_machine::CssVariableMachine;
use machine::{Machine, MachineState};
use std::fmt;

mod arbitrary_property_machine;
mod arbitrary_value_machine;
mod candidate_machine;
mod css_variable_machine;
mod machine;
mod modifier_machine;
mod named_utility_machine;
mod string_machine;
mod utility_machine;
mod variant_machine;

#[derive(Debug)]
pub enum Extracted<'a> {
    Candidate(&'a [u8]),
    CssVariable(&'a [u8]),
}

impl fmt::Display for Extracted<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Extracted::Candidate(candidate) => {
                write!(f, "Candidate({})", std::str::from_utf8(candidate).unwrap())
            }
            Extracted::CssVariable(candidate) => {
                write!(
                    f,
                    "CssVariable({})",
                    std::str::from_utf8(candidate).unwrap()
                )
            }
        }
    }
}

#[derive(Debug)]
pub struct Extractor<'a> {
    cursor: cursor::Cursor<'a>,

    css_variable_machine: CssVariableMachine,
    candidate_machine: CandidateMachine,
    candidate_machines: Vec<CandidateMachine>,
}

impl<'a> Extractor<'a> {
    pub fn new(input: &'a [u8]) -> Self {
        Self {
            cursor: cursor::Cursor::new(input),
            css_variable_machine: Default::default(),
            candidate_machine: Default::default(),
            candidate_machines: Default::default(),
        }
    }

    pub fn extract(&mut self) -> Vec<Extracted<'a>> {
        // Candidates found by inner candidate machines. If the outer machine finds a solution, we
        // can discard the inner machines. Otherwise, we can keep the candidates from the inner
        // machines.
        let mut in_flight_spans = vec![];

        // All the extracted values
        let mut extracted = vec![];

        for i in 0..self.cursor.input.len() {
            self.cursor.move_to(i);

            // Nested candidate machines, the moment we see a `[`, we want to start a new machine
            // that could look inside of the brackets and detect candidates. If the outer machine
            // finds a solution, we can discard the inner machines.
            //
            // E.g.:
            //
            // ```
            // [CssClass("flex")]
            //            ^^^^
            // ```
            // This looks like an arbitrary property at first, but there is no `:` inside.
            // This can also look like a arbitrary variant, but it's not followed by a `:`.
            // This can be an class attribute/decorator, or an array in most programming languages.
            //
            // If the outer machine does not find a solution, we can keep the candidates from the inner
            // machines.
            //
            // E.g.:
            //
            // ```
            // has-[.italic]:flex
            //       ^^^^^^
            // ```
            // The outer machine will complete because `has-[.italic]:flex` is a valid candidate with a
            // variant and a utility. The inner machine will find `italic`, but the inner machine will
            // be discarded because the outer machine found a solution.
            {
                if self.cursor.prev == b'[' {
                    self.candidate_machines.push(Default::default());
                }

                for machine in &mut self.candidate_machines {
                    if let MachineState::Done(span) = machine.next(&self.cursor) {
                        in_flight_spans.push(span);
                    }
                }
            }

            if let MachineState::Done(span) = self.candidate_machine.next(&self.cursor) {
                dbg!(std::str::from_utf8(span.slice(self.cursor.input)));
                in_flight_spans.push(span);
            }

            if let MachineState::Done(span) = self.css_variable_machine.next(&self.cursor) {
                extracted.push(Extracted::CssVariable(span.slice(self.cursor.input)));
            }
        }

        if !in_flight_spans.is_empty() {
            let spans = naive_drop_covered_spans(in_flight_spans);
            extracted.extend(
                spans
                    .iter()
                    .map(|span| Extracted::Candidate(span.slice(self.cursor.input))),
            );
        }

        extracted
    }
}

fn naive_drop_covered_spans(mut spans: Vec<Span>) -> Vec<Span> {
    // Step 1: Sort spans by start, and by end in descending order for ties
    spans.sort_by(|a, b| a.start.cmp(&b.start).then(b.end.cmp(&a.end)));

    let mut result = Vec::new();
    let mut max_end = 0;

    // Step 2: Iterate and filter spans
    for span in spans {
        if span.end > max_end {
            result.push(span);
            max_end = span.end;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::{Extracted, Extractor};
    use crate::parser;
    use crate::throughput::Throughput;
    use std::hint::black_box;

    #[test]
    fn test_extract_performance() {
        let iterations = 100_000;

        // let input = " px-(--my-variable) ";
        // let input = "flex items-center justify-center rounded-full ";
        let input = "mx-auto flex size-7 var(--my-variable) bg-red-500/(--my-opacity) items-center justify-center rounded-full ";
        // let input = "a b";
        // let input = "--my-variable var(--other-variable) calc(var(--foo)*var(--bar))";

        let throughput = Throughput::compute(iterations, input.len(), || {
            _ = black_box(parser::Extractor::all(input.as_bytes(), Default::default()));
        });
        eprintln!("Old extractor: {:}", throughput);

        let throughput = Throughput::compute(iterations * 4, input.len(), || {
            let mut extractor = Extractor::new(input.as_bytes());
            _ = black_box(extractor.extract());
        });
        eprintln!("New extractor: {:}", throughput);

        let mut extractor = Extractor::new(input.as_bytes());
        let start = std::time::Instant::now();
        let new_extractor = extractor
            .extract()
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>();
        let end = start.elapsed();
        eprintln!("Time elapsed (new extractor): {:?}", end);
        // dbg!(new_extractor);

        assert!(false);
    }

    #[test]
    fn test_candidates_extraction() {
        for (input, expected) in [
            // Simple utility
            ("flex", vec!["flex"]),
            ("_blank", vec!["blank"]),
            ("hover:_blank", vec!["hover:_blank"]),
            ("hover:focus:_blank", vec!["hover:focus:_blank"]),
            // Single character utility
            ("a", vec!["a"]),
            // Simple variant with simple utility
            ("hover:flex", vec!["hover:flex"]),
            // Multiple utilities
            ("flex block", vec!["flex", "block"]),
            // Simple utility with dashes
            ("items-center", vec!["items-center"]),
            // Simple utility with numbers
            ("px-2.5", vec!["px-2.5"]),
            // Arbitrary properties
            ("[color:red]", vec!["[color:red]"]),
            ("![color:red]", vec!["![color:red]"]),
            ("[color:red]!", vec!["[color:red]!"]),
            ("[color:red]/20", vec!["[color:red]/20"]),
            ("![color:red]/20", vec!["![color:red]/20"]),
            ("[color:red]/20!", vec!["[color:red]/20!"]),
            // In HTML
            (
                r#"<div class="flex items-center px-2.5 bg-[#0088cc] text-(--my-color)"></div>"#,
                vec![
                    "flex",
                    "items-center",
                    "px-2.5",
                    "bg-[#0088cc]",
                    "text-(--my-color)",
                ],
            ),
            // In an array, looks like an arbitrary property (because it starts with `[`).
            (r#"[flex]"#, vec!["flex"]),
            (r#"["flex"]"#, vec!["flex"]),
            (r#"[p-2.5]"#, vec!["p-2.5"]),
            (r#"["p-2.5"]"#, vec!["p-2.5"]),
            (r#"["flex","p-2.5"]"#, vec!["flex", "p-2.5"]),
            (r#"["flex", "p-2.5"]"#, vec!["flex", "p-2.5"]),
            // Ruby syntax
            (r#"%w[flex]"#, vec!["flex"]),
            (r#"%w[flex items-center]"#, vec!["flex", "items-center"]),
            (r#"%w[[color:red]]"#, vec!["[color:red]"]),
            // C# syntax. See: https://github.com/tailwindlabs/tailwindcss/issues/16189#issue-2826350984
            (r#"[CssClass("gap-y-4")]"#, vec!["gap-y-4"]),
            (r#"[CssClass("hover:gap-y-4")]"#, vec!["hover:gap-y-4"]),
            (
                r#"[CssClass("gap-y-4")]:flex"#,
                vec![r#"[CssClass("gap-y-4")]:flex"#],
            ),
            // Overlapping candidates, outer candidate should win
            (
                r#"[CssClass("[&:hover]:flex",'italic')]"#,
                vec!["[&:hover]:flex", "italic"],
            ),
            (
                r#"[flex,[italic,[underline]]]"#,
                vec!["flex", "italic", "underline"],
            ),
            // (
            //     r#"[:is(italic):is(underline)]"#,
            //     vec!["italic", "underline"],
            // ),
            // (
            //     r#"[:is(italic):is(underline)]:flex"#,
            //     vec!["[:is(italic):is(underline)]:flex"],
            // ),
            // Clojure syntax. See: https://github.com/tailwindlabs/tailwindcss/issues/16189#issuecomment-2642438176
            (r#"[:div {:class ["p-2"]}"#, vec!["div", "class", "p-2"]),
            (
                r#"[:div {:class ["p-2" "text-green"]}"#,
                vec!["div", "class", "p-2", "text-green"],
            ),
            (r#"[:div {:class ["p-2""#, vec!["div", "class", "p-2"]),
            (r#"               "text-green"]}"#, vec!["text-green"]),
            // (r#"[:div.p-2]"#, vec!["div", "p-2"]),
            // --------------------------------------------------------

            // Exceptions:
            //
            // Keys inside of a JS object could be a variant-less candidate. Vue example.
            // ("{ underline: true }", vec!["underline"]),
            // You can have variants but in a string. Vue example.
            (
                "{ 'hover:underline': true }",
                vec!["hover:underline", "true"],
            ),
            // Important marker on both sides is invalid
            ("!flex!", vec![]),
            // Important marker before a modifier is invalid
            ("bg-red-500!/20", vec![]),
            // HTML start of a tag
            ("<div", vec![]),
            // HTML end of a tag
            ("</div>", vec![]),
            // HTML element on its own
            ("<div></div>", vec![]),
            // Modifier followed by a modifier is invalid
            ("bg-red-500/20/20", vec![]),
            ("bg-red-500/20/[20%]", vec![]),
            ("bg-red-500/20/(--my-opacity)", vec![]),
            ("bg-red-500/[20%]/20", vec![]),
            ("bg-red-500/[20%]/[20%]", vec![]),
            ("bg-red-500/[20%]/(--my-opacity)", vec![]),
            ("bg-red-500/(--my-opacity)/20", vec![]),
            ("bg-red-500/(--my-opacity)/[20%]", vec![]),
            ("bg-red-500/(--my-opacity)/(--my-opacity)", vec![]),
            // Arbitrary value followed by an arbitrary value is invalid
            ("bg-[red]-[blue]", vec!["red", "blue"]),
            ("bg-[red][blue]", vec!["red", "blue"]),
            // Arbitrary value followed by an arbitrary variable is invalid
            ("bg-[red]-(--my-color)", vec!["red"]),
            ("bg-[red](--my-color)", vec!["red"]),
            // Important looking utility cannot be followed by another utility
            // ("flex!block", vec![]),
            // Invalid variants make the whole candidate invalid
            ("[foo]/bar:flex", vec!["foo"]),
        ] {
            let mut machine = Extractor::new(input.as_bytes());
            let actual = machine
                .extract()
                .iter()
                .filter_map(|x| match x {
                    Extracted::Candidate(candidate) => std::str::from_utf8(candidate).ok(),
                    Extracted::CssVariable(_) => None,
                })
                .collect::<Vec<_>>();

            assert_eq!(actual, expected);
        }
    }
}
