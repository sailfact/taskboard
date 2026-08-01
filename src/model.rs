#![allow(dead_code)]

#[derive(Debug, Clone)]
pub struct Card {
    pub title: String,
}

impl Card {
    pub fn new(title: impl Into<String>) -> Self {
        Self { title: title.into() }
    }
}

#[derive(Debug, Clone)]
pub struct Column {
    pub title: String,
    pub cards: Vec<Card>,
}

impl Column {
    pub fn new<T: Into<String>>(
        title: impl Into<String>,
        cards: impl IntoIterator<Item = T>,
    ) -> Self {
        Self {
            title: title.into(),
            cards: cards.into_iter().map(Card::new).collect(),
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
}

impl Default for Board {
    fn default() -> Self {
        Self {
            columns: [
                Column::new(
                    "Todo",
                    [
                        "Read the constraint docs",
                        "Sketch the board layout",
                        "Pick a colour palette",
                    ],
                ),
                Column::new("Doing", ["Split the frame into regions"]),
                Column::new("Done", ["cargo new taskboard", "cargo add ratatui"]),
            ],
        }
    }
}