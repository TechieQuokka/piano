//! Focus areas and Display views.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Keys,
    Code,
    Cmd,
}

impl Focus {
    pub fn next(self) -> Focus {
        match self {
            Focus::Keys => Focus::Code,
            Focus::Code => Focus::Cmd,
            Focus::Cmd => Focus::Keys,
        }
    }

    pub fn prev(self) -> Focus {
        match self {
            Focus::Keys => Focus::Cmd,
            Focus::Code => Focus::Keys,
            Focus::Cmd => Focus::Code,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Focus::Keys => "Keys",
            Focus::Code => "Code",
            Focus::Cmd => "Cmd",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Played,
    Ref,
    Output,
    Help,
    Man,
    Song,
    Tutorial,
}

impl View {
    pub fn name(self) -> &'static str {
        match self {
            View::Played => "played",
            View::Ref => "ref",
            View::Output => "output",
            View::Help => "help",
            View::Man => "man",
            View::Song => "song",
            View::Tutorial => "tutorial",
        }
    }
}
