//! The few strings the Rust side shows (menu-bar menu, window titles,
//! notifications), in the language the UI picked from the Mac's settings.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lang {
    #[default]
    En,
    It,
}

impl Lang {
    /// "it", "it-IT" → Italian; anything else → English.
    pub fn from_code(code: &str) -> Lang {
        if code.to_ascii_lowercase().split('-').next() == Some("it") {
            Lang::It
        } else {
            Lang::En
        }
    }
}

pub struct Text {
    /// Tray menu item that opens quick entry (Linux trays can't report clicks).
    pub quick_entry: &'static str,
    pub open: &'static str,
    pub settings: &'static str,
    pub quit: &'static str,
    pub all_day: &'static str,
    /// Prefix for a task's due time, e.g. "Due 18:00".
    pub due: &'static str,
}

const EN: Text = Text {
    quick_entry: "Quick Entry",
    open: "Open TaskCraft",
    settings: "Settings…",
    quit: "Quit TaskCraft",
    all_day: "All day",
    due: "Due",
};

const IT: Text = Text {
    quick_entry: "Inserimento rapido",
    open: "Apri TaskCraft",
    settings: "Impostazioni…",
    quit: "Esci da TaskCraft",
    all_day: "Tutto il giorno",
    due: "Scade alle",
};

pub fn text(lang: Lang) -> &'static Text {
    match lang {
        Lang::En => &EN,
        Lang::It => &IT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_language_from_code() {
        assert_eq!(Lang::from_code("it"), Lang::It);
        assert_eq!(Lang::from_code("it-IT"), Lang::It);
        assert_eq!(Lang::from_code("en-GB"), Lang::En);
        assert_eq!(Lang::from_code("de"), Lang::En);
    }

    #[test]
    fn menu_text() {
        assert_eq!(text(Lang::It).quit, "Esci da TaskCraft");
        assert_eq!(text(Lang::En).settings, "Settings…");
    }
}
