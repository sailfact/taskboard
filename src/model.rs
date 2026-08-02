#[derive(Debug, Clone)]
pub struct Card {
    pub title: String,
    pub tag: Option<String>,
    pub notes: String,
}

impl Card {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            tag: None,
            notes: String::new(),
        }
    }

    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = Some(tag.into());
        self
    }

    pub fn notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = notes.into();
        self
    }
}

#[derive(Debug, Clone)]
pub struct Column {
    pub title: String,
    pub cards: Vec<Card>,
}

impl Column {
    pub fn new(title: impl Into<String>, cards: impl IntoIterator<Item = Card>) -> Self {
        Self {
            title: title.into(),
            cards: cards.into_iter().collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Board {
    pub columns: [Column; 3],
}

impl Board {
    pub fn open(&self) -> usize {
        self.columns[0].cards.len() + self.columns[1].cards.len()
    }

    pub fn done(&self) -> usize {
        self.columns[2].cards.len()
    }

    pub fn total(&self) -> usize {
        self.columns.iter().map(|c| c.cards.len()).sum()
    }

    /// Fraction of the board that is complete, in the range 0.0 to 1.0.
    pub fn ratio(&self) -> f64 {
        if self.total() == 0 {
            0.0
        } else {
            self.done() as f64 / self.total() as f64
        }
    }
}

impl Default for Board {
    fn default() -> Self {
        Self {
            columns: [
                Column::new(
                    "Todo",
                    [
                        Card::new("Read the constraint docs").tag("layout").notes(
                            "Cassowary is a soft constraint solver. Conflicting constraints \
                                 resolve to a compromise rather than raising an error, which is \
                                 why a Length in a too-small terminal simply clips.",
                        ),
                        Card::new("Sketch the board layout").tag("design").notes(
                            "Three columns, a details pane past 100 columns, help floating on top.",
                        ),
                        Card::new("Pick a colour palette").tag("design").notes(
                            "Named colours respect the user's terminal theme. RGB does not.",
                        ),
                    ],
                ),
                Column::new(
                    "Doing",
                    [Card::new("Split the frame into regions")
                        .tag("layout")
                        .notes("AppLayout::compute is a pure function from one Rect to many.")],
                ),
                Column::new(
                    "Done",
                    [
                        Card::new("cargo new taskboard").tag("setup"),
                        Card::new("cargo add ratatui").tag("setup"),
                    ],
                ),
            ],
        }
    }
}
