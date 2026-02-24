//! Message rendering — user vs assistant styling, rich markdown.
//!
//! Supports: bold, italic, inline code, fenced code blocks with syntax
//! highlighting, markdown tables, ordered/unordered lists.

use gtk4::pango::WrapMode;
use gtk4::prelude::*;
use gtk4::{Align, Orientation};

use levsha_engine::types::MessageRole;
use levsha_engine::welcome::WELCOME_TEXT;

use crate::avatar;

pub mod syntax;

/// Create a complete message widget for a given role and content.
pub fn create_message_widget(role: MessageRole, content: &str) -> gtk4::Box {
    let row = gtk4::Box::new(Orientation::Vertical, 0);
    row.add_css_class("message-row");
    row.set_hexpand(true);

    // Determine max-width constraint.
    let max_width = 720;

    // Container for centering the bubble at max-width.
    let center_box = gtk4::Box::new(Orientation::Vertical, 0);
    center_box.set_halign(Align::Center);
    center_box.set_hexpand(true);
    center_box.set_size_request(max_width.min(max_width), -1);

    match role {
        MessageRole::User => {
            // Role label.
            let role_label = gtk4::Label::new(Some("You"));
            role_label.add_css_class("role-label");
            role_label.set_halign(Align::End);
            role_label.set_margin_bottom(4);
            center_box.append(&role_label);

            // User bubble.
            let bubble = gtk4::Box::new(Orientation::Vertical, 0);
            bubble.add_css_class("message-bubble");
            bubble.add_css_class("message-bubble-user");
            bubble.set_halign(Align::End);

            let label = create_text_label(content);
            bubble.append(&label);
            center_box.append(&bubble);
        }
        MessageRole::Assistant | MessageRole::System => {
            // Check if this is the welcome message — render as special card.
            if content == WELCOME_TEXT {
                let card = create_welcome_card(content);
                center_box.append(&card);
            } else {
                // Role label row with avatar.
                let role_row = gtk4::Box::new(Orientation::Horizontal, 6);
                role_row.set_halign(Align::Start);
                role_row.set_valign(Align::Center);
                role_row.set_margin_bottom(4);
                if let Some(av) = avatar::create_avatar(20) {
                    role_row.append(&av);
                }
                let role_label = gtk4::Label::new(Some("Levsha"));
                role_label.add_css_class("role-label");
                role_label.add_css_class("role-label-assistant");
                role_row.append(&role_label);
                center_box.append(&role_row);

                // Assistant bubble.
                let bubble = gtk4::Box::new(Orientation::Vertical, 8);
                bubble.add_css_class("message-bubble");
                bubble.add_css_class("message-bubble-assistant");
                bubble.set_halign(Align::Start);
                bubble.set_hexpand(true);

                // Parse content into blocks.
                let blocks = parse_content_blocks(content);
                for block in blocks {
                    match block {
                        ContentBlock::Text(text) => {
                            let label = create_text_label(&text);
                            bubble.append(&label);
                        }
                        ContentBlock::Code { language, code } => {
                            let code_widget = create_code_block(&language, &code);
                            bubble.append(&code_widget);
                        }
                        ContentBlock::Table { headers, rows } => {
                            let table_widget = create_table_widget(&headers, &rows);
                            bubble.append(&table_widget);
                        }
                        ContentBlock::List { ordered, items } => {
                            let list_widget = create_list_widget(ordered, &items);
                            bubble.append(&list_widget);
                        }
                    }
                }
                center_box.append(&bubble);
            }
        }
        _ => {
            // ToolCall / ToolResult -- render as system-style.
            let bubble = gtk4::Box::new(Orientation::Vertical, 0);
            bubble.add_css_class("message-bubble");
            bubble.add_css_class("message-bubble-system");
            bubble.set_halign(Align::Start);

            let label = create_text_label(content);
            bubble.append(&label);
            center_box.append(&bubble);
        }
    }

    row.append(&center_box);
    row
}

/// Create a streaming message widget -- returns the row and the label to update.
pub fn create_streaming_message_widget() -> (gtk4::Box, gtk4::Label) {
    let row = gtk4::Box::new(Orientation::Vertical, 0);
    row.add_css_class("message-row");
    row.set_hexpand(true);

    let max_width = 720;

    let center_box = gtk4::Box::new(Orientation::Vertical, 0);
    center_box.set_halign(Align::Center);
    center_box.set_hexpand(true);
    center_box.set_size_request(max_width.min(max_width), -1);

    // Role label row with avatar.
    let role_row = gtk4::Box::new(Orientation::Horizontal, 6);
    role_row.set_halign(Align::Start);
    role_row.set_valign(Align::Center);
    role_row.set_margin_bottom(4);
    if let Some(av) = avatar::create_avatar(20) {
        role_row.append(&av);
    }
    let role_label = gtk4::Label::new(Some("Levsha"));
    role_label.add_css_class("role-label");
    role_label.add_css_class("role-label-assistant");
    role_row.append(&role_label);
    center_box.append(&role_row);

    // Bubble.
    let bubble = gtk4::Box::new(Orientation::Vertical, 0);
    bubble.add_css_class("message-bubble");
    bubble.add_css_class("message-bubble-assistant");
    bubble.set_halign(Align::Start);
    bubble.set_hexpand(true);

    // Label for streaming text.
    let label = gtk4::Label::new(None);
    label.add_css_class("message-text");
    label.set_wrap(true);
    label.set_wrap_mode(WrapMode::WordChar);
    label.set_xalign(0.0);
    label.set_use_markup(true);
    label.set_selectable(true);
    label.set_focusable(false);
    label.set_hexpand(true);

    bubble.append(&label);
    center_box.append(&bubble);
    row.append(&center_box);

    (row, label)
}

/// Create a styled text label with Pango markup for basic markdown.
fn create_text_label(content: &str) -> gtk4::Label {
    let label = gtk4::Label::new(None);
    label.add_css_class("message-text");
    label.set_wrap(true);
    label.set_wrap_mode(WrapMode::WordChar);
    label.set_xalign(0.0);
    label.set_use_markup(true);
    label.set_selectable(true);
    label.set_focusable(false);
    label.set_hexpand(true);
    label.set_markup(&markdown_to_pango(content));
    label
}

/// Convert basic markdown to Pango markup.
pub fn markdown_to_pango(text: &str) -> String {
    let escaped = glib::markup_escape_text(text);
    let mut result = escaped.to_string();

    // Bold: **text** -> <b>text</b>
    result = apply_inline_pattern(&result, "**", "<b>", "</b>");

    // Italic: *text* -> <i>text</i> (but not inside bold markers)
    result = apply_single_star_italic(&result);

    // Inline code: `text` -> <tt>text</tt>
    result = apply_inline_pattern(&result, "`", "<tt>", "</tt>");

    result
}

/// Apply a simple symmetric inline pattern like **bold** or `code`.
fn apply_inline_pattern(text: &str, marker: &str, open: &str, close: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let marker_chars: Vec<char> = marker.chars().collect();
    let marker_len = marker_chars.len();
    let mut inside = false;

    while chars.peek().is_some() {
        // Check if next chars match the marker.
        let remaining: String = chars.clone().collect();
        if remaining.starts_with(marker) {
            if inside {
                result.push_str(close);
                inside = false;
            } else {
                result.push_str(open);
                inside = true;
            }
            for _ in 0..marker_len {
                chars.next();
            }
        } else {
            if let Some(c) = chars.next() {
                result.push(c);
            }
        }
    }

    // Close unclosed markers.
    if inside {
        result.push_str(close);
    }

    result
}

/// Apply single-star italic (*text*) without conflicting with bold (**).
fn apply_single_star_italic(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut inside = false;

    while i < len {
        if chars[i] == '*' {
            // Skip if this is part of a bold tag <b> or </b>.
            let before = &result;
            if before.ends_with('<') || before.ends_with("</") {
                result.push(chars[i]);
                i += 1;
                continue;
            }
            if inside {
                result.push_str("</i>");
                inside = false;
            } else {
                result.push_str("<i>");
                inside = true;
            }
            i += 1;
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    if inside {
        result.push_str("</i>");
    }

    result
}

// ── Content Block Parsing ──────────────────────────────────────────

/// Content block types for parsing message bodies.
enum ContentBlock {
    Text(String),
    Code {
        language: String,
        code: String,
    },
    Table {
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    List {
        ordered: bool,
        items: Vec<String>,
    },
}

/// Parse content into text blocks, fenced code blocks, tables, and lists.
fn parse_content_blocks(content: &str) -> Vec<ContentBlock> {
    let mut blocks = Vec::new();
    let mut current_text = String::new();
    let mut in_code_block = false;
    let mut code_language = String::new();
    let mut code_content = String::new();

    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        // Code block fences.
        if !in_code_block && line.starts_with("```") {
            if !current_text.is_empty() {
                blocks.push(ContentBlock::Text(current_text.trim_end().to_string()));
                current_text.clear();
            }
            code_language = line.trim_start_matches('`').trim().to_string();
            code_content.clear();
            in_code_block = true;
            i += 1;
            continue;
        }
        if in_code_block && line.starts_with("```") {
            blocks.push(ContentBlock::Code {
                language: code_language.clone(),
                code: code_content.trim_end().to_string(),
            });
            code_language.clear();
            code_content.clear();
            in_code_block = false;
            i += 1;
            continue;
        }
        if in_code_block {
            if !code_content.is_empty() {
                code_content.push('\n');
            }
            code_content.push_str(line);
            i += 1;
            continue;
        }

        // Try to parse a markdown table starting at this line.
        if is_table_line(line) {
            if let Some((table_block, consumed)) = try_parse_table(&lines[i..]) {
                if !current_text.is_empty() {
                    blocks.push(ContentBlock::Text(current_text.trim_end().to_string()));
                    current_text.clear();
                }
                blocks.push(table_block);
                i += consumed;
                continue;
            }
        }

        // Try to parse a list starting at this line.
        if is_list_line(line) {
            if let Some((list_block, consumed)) = try_parse_list(&lines[i..]) {
                if !current_text.is_empty() {
                    blocks.push(ContentBlock::Text(current_text.trim_end().to_string()));
                    current_text.clear();
                }
                blocks.push(list_block);
                i += consumed;
                continue;
            }
        }

        // Plain text.
        if !current_text.is_empty() {
            current_text.push('\n');
        }
        current_text.push_str(line);
        i += 1;
    }

    // Handle unclosed code block.
    if in_code_block && !code_content.is_empty() {
        blocks.push(ContentBlock::Code {
            language: code_language,
            code: code_content.trim_end().to_string(),
        });
    }

    if !current_text.is_empty() {
        blocks.push(ContentBlock::Text(current_text.trim_end().to_string()));
    }

    blocks
}

// ── Table Parsing ──────────────────────────────────────────────────

/// Check if a line looks like it could be part of a markdown table.
fn is_table_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with('|') && trimmed.ends_with('|') && trimmed.len() > 2
}

/// Check if a line is a table separator (e.g. `|---|---|`).
fn is_separator_line(line: &str) -> bool {
    let trimmed = line.trim();
    if !trimmed.starts_with('|') || !trimmed.ends_with('|') {
        return false;
    }
    let inner = &trimmed[1..trimmed.len() - 1];
    inner.split('|').all(|cell| {
        let c = cell.trim();
        !c.is_empty() && c.chars().all(|ch| ch == '-' || ch == ':' || ch == ' ')
    })
}

/// Parse cells from a table row like `| a | b | c |`.
fn parse_table_cells(line: &str) -> Vec<String> {
    let trimmed = line.trim();
    // Strip leading/trailing pipes.
    let inner = if trimmed.starts_with('|') && trimmed.ends_with('|') {
        &trimmed[1..trimmed.len() - 1]
    } else {
        trimmed
    };
    inner.split('|').map(|c| c.trim().to_string()).collect()
}

/// Try to parse a markdown table from a slice of lines.
/// Returns the ContentBlock and number of lines consumed, or None.
fn try_parse_table(lines: &[&str]) -> Option<(ContentBlock, usize)> {
    if lines.len() < 3 {
        return None;
    }

    // Line 0: header row, line 1: separator, line 2+: data rows.
    if !is_table_line(lines[0]) || !is_separator_line(lines[1]) {
        return None;
    }

    let headers = parse_table_cells(lines[0]);
    let col_count = headers.len();

    let mut rows = Vec::new();
    let mut consumed = 2; // header + separator

    for line in &lines[2..] {
        if !is_table_line(line) {
            break;
        }
        let mut cells = parse_table_cells(line);
        // Pad or truncate to match header count.
        cells.resize(col_count, String::new());
        rows.push(cells);
        consumed += 1;
    }

    if rows.is_empty() {
        return None;
    }

    Some((ContentBlock::Table { headers, rows }, consumed))
}

/// Create a table widget from parsed headers and rows.
fn create_table_widget(headers: &[String], rows: &[Vec<String>]) -> gtk4::Box {
    let outer = gtk4::Box::new(Orientation::Vertical, 0);
    outer.add_css_class("markdown-table");

    let col_count = headers.len() as i32;

    // Header row.
    let header_row = gtk4::Box::new(Orientation::Horizontal, 0);
    header_row.add_css_class("markdown-table-header");
    for header in headers {
        let cell = gtk4::Label::new(None);
        cell.set_markup(&format!("<b>{}</b>", glib::markup_escape_text(header)));
        cell.add_css_class("markdown-table-cell");
        cell.add_css_class("markdown-table-header-cell");
        cell.set_hexpand(true);
        cell.set_xalign(0.0);
        cell.set_wrap(true);
        cell.set_wrap_mode(WrapMode::WordChar);
        cell.set_size_request(80.max(600 / col_count), -1);
        header_row.append(&cell);
    }
    outer.append(&header_row);

    // Data rows.
    for (row_idx, row) in rows.iter().enumerate() {
        let row_box = gtk4::Box::new(Orientation::Horizontal, 0);
        row_box.add_css_class("markdown-table-row");
        if row_idx % 2 == 1 {
            row_box.add_css_class("markdown-table-row-alt");
        }
        for cell_text in row {
            let cell = gtk4::Label::new(None);
            cell.set_markup(&markdown_to_pango(cell_text));
            cell.add_css_class("markdown-table-cell");
            cell.set_hexpand(true);
            cell.set_xalign(0.0);
            cell.set_wrap(true);
            cell.set_wrap_mode(WrapMode::WordChar);
            cell.set_selectable(true);
            cell.set_focusable(false);
            cell.set_size_request(80.max(600 / col_count), -1);
            row_box.append(&cell);
        }
        outer.append(&row_box);
    }

    outer
}

// ── List Parsing ───────────────────────────────────────────────────

/// Check if a line starts a list item (unordered or ordered).
fn is_list_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
        return true;
    }
    // Ordered: `1. `, `2. `, etc.
    if let Some(dot_pos) = trimmed.find(". ") {
        let prefix = &trimmed[..dot_pos];
        return !prefix.is_empty() && prefix.chars().all(|c| c.is_ascii_digit());
    }
    false
}

/// Determine if a line is an unordered list item.
fn is_unordered(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("- ") || trimmed.starts_with("* ")
}

/// Determine if a line is an ordered list item.
fn is_ordered(line: &str) -> bool {
    let trimmed = line.trim_start();
    if let Some(dot_pos) = trimmed.find(". ") {
        let prefix = &trimmed[..dot_pos];
        return !prefix.is_empty() && prefix.chars().all(|c| c.is_ascii_digit());
    }
    false
}

/// Extract the text content of a list item line.
fn list_item_text(line: &str) -> &str {
    let trimmed = line.trim_start();
    if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
        return &trimmed[2..];
    }
    if let Some(dot_pos) = trimmed.find(". ") {
        return &trimmed[dot_pos + 2..];
    }
    trimmed
}

/// Try to parse a list (ordered or unordered) from a slice of lines.
fn try_parse_list(lines: &[&str]) -> Option<(ContentBlock, usize)> {
    if lines.is_empty() || !is_list_line(lines[0]) {
        return None;
    }

    let ordered = is_ordered(lines[0]);
    let mut items = Vec::new();
    let mut consumed = 0;

    for line in lines {
        if !is_list_line(line) {
            break;
        }
        // Check consistency: all items should be same type.
        if ordered && !is_ordered(line) {
            break;
        }
        if !ordered && !is_unordered(line) {
            break;
        }
        items.push(list_item_text(line).to_string());
        consumed += 1;
    }

    if items.is_empty() {
        return None;
    }

    Some((ContentBlock::List { ordered, items }, consumed))
}

/// Create a list widget from parsed items.
fn create_list_widget(ordered: bool, items: &[String]) -> gtk4::Box {
    let container = gtk4::Box::new(Orientation::Vertical, 4);
    container.add_css_class("markdown-list");

    for (idx, item) in items.iter().enumerate() {
        let row = gtk4::Box::new(Orientation::Horizontal, 8);
        row.add_css_class("markdown-list-item");

        let bullet_text = if ordered {
            format!("{}.", idx + 1)
        } else {
            "\u{2022}".to_string() // bullet character
        };

        let bullet = gtk4::Label::new(Some(&bullet_text));
        bullet.add_css_class("markdown-list-bullet");
        bullet.set_valign(Align::Start);
        bullet.set_xalign(1.0);
        bullet.set_size_request(24, -1);

        let content = gtk4::Label::new(None);
        content.set_markup(&markdown_to_pango(item));
        content.add_css_class("message-text");
        content.set_wrap(true);
        content.set_wrap_mode(WrapMode::WordChar);
        content.set_xalign(0.0);
        content.set_use_markup(true);
        content.set_selectable(true);
        content.set_focusable(false);
        content.set_hexpand(true);

        row.append(&bullet);
        row.append(&content);
        container.append(&row);
    }

    container
}

// ── Welcome Card ────────────────────────────────────────────────────

/// Embedded hero banner for the welcome card.
const HERO_BANNER: &[u8] = include_bytes!("../../../assets/levsha_fanart.png");

/// Create the welcome card with avatar header, body text, and hero banner.
fn create_welcome_card(content: &str) -> gtk4::Box {
    let card = gtk4::Box::new(Orientation::Vertical, 12);
    card.add_css_class("welcome-card");
    card.set_halign(Align::Start);
    card.set_hexpand(true);

    // Header row: avatar + title.
    let header = gtk4::Box::new(Orientation::Horizontal, 8);
    header.set_valign(Align::Center);
    if let Some(av) = avatar::create_avatar(28) {
        header.append(&av);
    }
    let title = gtk4::Label::new(Some("Levsha OS"));
    title.add_css_class("welcome-title");
    header.append(&title);
    card.append(&header);

    // Body text.
    let body = create_text_label(content);
    card.append(&body);

    // Hero banner.
    if let Some(banner) = create_hero_banner() {
        card.append(&banner);
    }

    card
}

/// Create the hero banner image for the welcome card.
fn create_hero_banner() -> Option<gtk4::Box> {
    let bytes = glib::Bytes::from_static(HERO_BANNER);
    let texture = match gdk4::Texture::from_bytes(&bytes) {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!("Failed to load hero banner texture: {e}");
            return None;
        }
    };

    let picture = gtk4::Picture::for_paintable(&texture);
    picture.set_content_fit(gtk4::ContentFit::Cover);
    picture.set_hexpand(true);
    picture.set_size_request(-1, 280);

    let wrapper = gtk4::Box::new(Orientation::Vertical, 0);
    wrapper.add_css_class("welcome-banner");
    wrapper.set_hexpand(true);
    wrapper.set_overflow(gtk4::Overflow::Hidden);
    wrapper.append(&picture);

    Some(wrapper)
}

// ── Syntax-Highlighted Code Blocks ─────────────────────────────────

/// Create a styled code block widget with syntax highlighting.
fn create_code_block(language: &str, code: &str) -> gtk4::Box {
    let container = gtk4::Box::new(Orientation::Vertical, 0);
    container.add_css_class("code-block");

    if !language.is_empty() {
        let header_box = gtk4::Box::new(Orientation::Horizontal, 4);
        header_box.add_css_class("code-block-header");
        header_box.set_halign(Align::Start);

        let terminal_icon = gtk4::Label::new(Some("\u{276F}"));
        terminal_icon.add_css_class("code-block-terminal-icon");
        header_box.append(&terminal_icon);

        let lang_label = gtk4::Label::new(Some(language));
        header_box.append(&lang_label);

        container.append(&header_box);
    }

    let code_label = gtk4::Label::new(None);
    code_label.add_css_class("code-block-text");
    code_label.set_wrap(true);
    code_label.set_wrap_mode(WrapMode::WordChar);
    code_label.set_xalign(0.0);
    code_label.set_selectable(true);
    code_label.set_focusable(false);
    code_label.set_hexpand(true);
    code_label.set_use_markup(true);

    let highlighted = syntax::highlight_code(language, code);
    code_label.set_markup(&highlighted);

    container.append(&code_label);
    container
}
