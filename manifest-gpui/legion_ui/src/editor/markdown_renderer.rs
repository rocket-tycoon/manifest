use gpui::*;

use crate::theme::Theme;

/// Simple markdown renderer that converts markdown text into GPUI elements
pub struct MarkdownRenderer;

/// Parsed markdown block
#[derive(Debug, Clone, PartialEq)]
pub enum MarkdownBlock {
    Heading { level: usize, text: String },
    Paragraph(String),
    CodeBlock { language: Option<String>, code: String },
    UnorderedList(Vec<String>),
    OrderedList(Vec<String>),
    Blockquote(String),
    HorizontalRule,
}

impl MarkdownRenderer {
    /// Render markdown content to GPUI elements
    pub fn render(content: &str) -> impl IntoElement {
        let blocks = Self::parse(content);

        div()
            .flex()
            .flex_col()
            .gap_3()
            .children(blocks.into_iter().map(|block| Self::render_block(block)))
    }

    /// Parse markdown text into blocks
    pub fn parse(content: &str) -> Vec<MarkdownBlock> {
        let mut blocks = Vec::new();
        let mut lines = content.lines().peekable();
        let mut current_paragraph = String::new();

        while let Some(line) = lines.next() {
            // Headings
            if line.starts_with('#') {
                // Flush any pending paragraph
                if !current_paragraph.is_empty() {
                    blocks.push(MarkdownBlock::Paragraph(current_paragraph.trim().to_string()));
                    current_paragraph.clear();
                }

                let level = line.chars().take_while(|c| *c == '#').count();
                let text = line.trim_start_matches('#').trim().to_string();
                blocks.push(MarkdownBlock::Heading { level, text });
                continue;
            }

            // Horizontal rule
            if line.trim() == "---" || line.trim() == "***" || line.trim() == "___" {
                if !current_paragraph.is_empty() {
                    blocks.push(MarkdownBlock::Paragraph(current_paragraph.trim().to_string()));
                    current_paragraph.clear();
                }
                blocks.push(MarkdownBlock::HorizontalRule);
                continue;
            }

            // Code block
            if line.starts_with("```") {
                if !current_paragraph.is_empty() {
                    blocks.push(MarkdownBlock::Paragraph(current_paragraph.trim().to_string()));
                    current_paragraph.clear();
                }

                let language = line.trim_start_matches('`').trim();
                let language = if language.is_empty() { None } else { Some(language.to_string()) };
                let mut code = String::new();

                while let Some(code_line) = lines.next() {
                    if code_line.starts_with("```") {
                        break;
                    }
                    if !code.is_empty() {
                        code.push('\n');
                    }
                    code.push_str(code_line);
                }

                blocks.push(MarkdownBlock::CodeBlock { language, code });
                continue;
            }

            // Blockquote
            if line.starts_with('>') {
                if !current_paragraph.is_empty() {
                    blocks.push(MarkdownBlock::Paragraph(current_paragraph.trim().to_string()));
                    current_paragraph.clear();
                }

                let mut quote_text = line.trim_start_matches('>').trim().to_string();
                while let Some(next_line) = lines.peek() {
                    if next_line.starts_with('>') {
                        quote_text.push(' ');
                        quote_text.push_str(lines.next().unwrap().trim_start_matches('>').trim());
                    } else {
                        break;
                    }
                }
                blocks.push(MarkdownBlock::Blockquote(quote_text));
                continue;
            }

            // Unordered list
            if line.starts_with("- ") || line.starts_with("* ") || line.starts_with("+ ") {
                if !current_paragraph.is_empty() {
                    blocks.push(MarkdownBlock::Paragraph(current_paragraph.trim().to_string()));
                    current_paragraph.clear();
                }

                let mut items = vec![line[2..].to_string()];
                while let Some(next_line) = lines.peek() {
                    if next_line.starts_with("- ") || next_line.starts_with("* ") || next_line.starts_with("+ ") {
                        items.push(lines.next().unwrap()[2..].to_string());
                    } else {
                        break;
                    }
                }
                blocks.push(MarkdownBlock::UnorderedList(items));
                continue;
            }

            // Ordered list
            if line.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                if let Some(dot_pos) = line.find(". ") {
                    let prefix = &line[..dot_pos];
                    if prefix.chars().all(|c| c.is_ascii_digit()) {
                        if !current_paragraph.is_empty() {
                            blocks.push(MarkdownBlock::Paragraph(current_paragraph.trim().to_string()));
                            current_paragraph.clear();
                        }

                        let mut items = vec![line[dot_pos + 2..].to_string()];
                        while let Some(next_line) = lines.peek() {
                            if next_line.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                                if let Some(next_dot) = next_line.find(". ") {
                                    let next_prefix = &next_line[..next_dot];
                                    if next_prefix.chars().all(|c| c.is_ascii_digit()) {
                                        items.push(lines.next().unwrap()[next_dot + 2..].to_string());
                                        continue;
                                    }
                                }
                            }
                            break;
                        }
                        blocks.push(MarkdownBlock::OrderedList(items));
                        continue;
                    }
                }
            }

            // Empty line = end of paragraph
            if line.trim().is_empty() {
                if !current_paragraph.is_empty() {
                    blocks.push(MarkdownBlock::Paragraph(current_paragraph.trim().to_string()));
                    current_paragraph.clear();
                }
                continue;
            }

            // Regular text, accumulate into paragraph
            if !current_paragraph.is_empty() {
                current_paragraph.push(' ');
            }
            current_paragraph.push_str(line);
        }

        // Flush any remaining paragraph
        if !current_paragraph.is_empty() {
            blocks.push(MarkdownBlock::Paragraph(current_paragraph.trim().to_string()));
        }

        blocks
    }

    /// Render a single block to a GPUI element
    fn render_block(block: MarkdownBlock) -> AnyElement {
        match block {
            MarkdownBlock::Heading { level, text } => {
                let (size, weight) = match level {
                    1 => (px(28.0), FontWeight::BOLD),
                    2 => (px(24.0), FontWeight::BOLD),
                    3 => (px(20.0), FontWeight::SEMIBOLD),
                    4 => (px(18.0), FontWeight::SEMIBOLD),
                    5 => (px(16.0), FontWeight::MEDIUM),
                    _ => (px(14.0), FontWeight::MEDIUM),
                };

                div()
                    .text_size(size)
                    .font_weight(weight)
                    .text_color(Theme::text())
                    .mt_2()
                    .mb_1()
                    .child(text)
                    .into_any_element()
            }

            MarkdownBlock::Paragraph(text) => {
                div()
                    .text_sm()
                    .line_height(px(22.0))
                    .text_color(Theme::text_editor())
                    .child(Self::render_inline_formatting(&text))
                    .into_any_element()
            }

            MarkdownBlock::CodeBlock { code, .. } => {
                div()
                    .w_full()
                    .p_3()
                    .rounded_md()
                    .bg(Theme::surface())
                    .border_1()
                    .border_color(Theme::border())
                    .child(
                        div()
                            .font_family("monospace")
                            .text_sm()
                            .text_color(Theme::text_editor())
                            .whitespace_nowrap()
                            .child(code)
                    )
                    .into_any_element()
            }

            MarkdownBlock::UnorderedList(items) => {
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .pl_4()
                    .children(items.into_iter().map(|item| {
                        div()
                            .flex()
                            .flex_row()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(Theme::text_muted())
                                    .child("•")
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(Theme::text_editor())
                                    .child(item)
                            )
                    }))
                    .into_any_element()
            }

            MarkdownBlock::OrderedList(items) => {
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .pl_4()
                    .children(items.into_iter().enumerate().map(|(i, item)| {
                        div()
                            .flex()
                            .flex_row()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(Theme::text_muted())
                                    .child(format!("{}.", i + 1))
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(Theme::text_editor())
                                    .child(item)
                            )
                    }))
                    .into_any_element()
            }

            MarkdownBlock::Blockquote(text) => {
                div()
                    .pl_3()
                    .py_1()
                    .border_l_2()
                    .border_color(Theme::text_muted())
                    .child(
                        div()
                            .text_sm()
                            .italic()
                            .text_color(Theme::text_muted())
                            .child(text)
                    )
                    .into_any_element()
            }

            MarkdownBlock::HorizontalRule => {
                div()
                    .w_full()
                    .h(px(1.0))
                    .my_4()
                    .bg(Theme::border())
                    .into_any_element()
            }
        }
    }

    /// Render inline formatting (bold, italic, code)
    fn render_inline_formatting(text: &str) -> String {
        // For now, just return the text as-is
        // Full inline formatting would require a more complex parser
        // that outputs multiple styled spans
        text.to_string()
    }
}
