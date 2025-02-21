pub struct FormattingOptions {
    pub space_around_binop: bool,

    pub insert_spaces: bool,
    pub tab_size: usize,
}

impl Default for FormattingOptions {
    fn default() -> Self {
        Self {
            space_around_binop: true,
            insert_spaces: true,
            tab_size: 4,
        }
    }
}
