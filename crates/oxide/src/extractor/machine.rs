use crate::cursor;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Span {
    /// Inclusive start position of the span
    pub(crate) start: usize,

    /// Inclusive end position of the span
    pub(crate) end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    #[inline(always)]
    pub fn slice<'a>(&self, input: &'a [u8]) -> &'a [u8] {
        &input[self.start..=self.end]
    }
}

#[derive(Debug)]
pub enum MachineState {
    /// Machine is not doing anything at the moment
    Idle,

    /// Machine is currently parsing
    Parsing,

    /// Machine is done parsing and has extracted a span
    Done(Span),
}

pub(crate) trait Machine: Sized + Default {
    fn next(&mut self, cursor: &cursor::Cursor<'_>) -> MachineState;
    fn reset(&mut self) {
        *self = Default::default();
    }

    /// Reset the state machine, and mark the machine as [MachineState::Idle].
    #[inline(always)]
    fn restart(&mut self) -> MachineState {
        self.reset();
        MachineState::Idle
    }

    /// Reset the state machine, and mark the machine as [MachineState::Done(…)].
    #[inline(always)]
    fn done(&mut self, start: usize, cursor: &cursor::Cursor<'_>) -> MachineState {
        self.reset();
        MachineState::Done(Span::new(start, cursor.pos))
    }
}
