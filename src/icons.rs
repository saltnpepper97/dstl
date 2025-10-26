use nerd_font_symbols::{fa, md, oct};

pub fn category_icon(category: &str) -> &'static str {
    match category {
        "Recent" => fa::FA_CLOCK_ROTATE_LEFT,
        "Utilities" => fa::FA_GEAR,
        "Development" => fa::FA_HAMMER,
        "Network" => md::MD_EARTH,
        "Audio/Video" => fa::FA_MUSIC,
        "Graphics" => fa::FA_PAINTBRUSH,
        "System" => fa::FA_DESKTOP,
        "Office" => md::MD_FILE_DOCUMENT,
        "Games" => fa::FA_GAMEPAD,
        "Education" => fa::FA_GRADUATION_CAP,
        "Settings" => fa::FA_SLIDERS,
        _ => oct::OCT_DASH,
    }
}
