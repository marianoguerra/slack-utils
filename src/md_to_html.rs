use std::fs;
use std::path::Path;

use crate::{AppError, Result};

/// Options for markdown to HTML conversion
#[derive(Debug, Clone, Default)]
pub struct MdToHtmlOptions {
    /// Use GFM (GitHub Flavored Markdown) preset
    pub gfm: bool,
    /// Enable autolinks (URLs become links automatically)
    pub autolink: bool,
    /// Enable code (indented) blocks
    pub code_indented: bool,
    /// Enable code (fenced) blocks
    pub code_fenced: bool,
    /// Enable definition lists
    pub definition: bool,
    /// Enable frontmatter (YAML metadata)
    pub frontmatter: bool,
    /// Enable GFM autolink literals
    pub gfm_autolink_literal: bool,
    /// Enable GFM footnotes
    pub gfm_footnote_definition: bool,
    /// Enable GFM label start footnote
    pub gfm_label_start_footnote: bool,
    /// Enable GFM strikethrough
    pub gfm_strikethrough: bool,
    /// Enable GFM tables
    pub gfm_table: bool,
    /// Enable GFM task list items
    pub gfm_task_list_item: bool,
    /// Enable hard break (escape)
    pub hard_break_escape: bool,
    /// Enable hard break (trailing)
    pub hard_break_trailing: bool,
    /// Enable HTML (flow)
    pub html_flow: bool,
    /// Enable HTML (text)
    pub html_text: bool,
    /// Enable label end
    pub label_end: bool,
    /// Enable label start (image)
    pub label_start_image: bool,
    /// Enable label start (link)
    pub label_start_link: bool,
    /// Enable list items
    pub list_item: bool,
    /// Enable math (flow)
    pub math_flow: bool,
    /// Enable math (text)
    pub math_text: bool,
    /// Enable thematic break (---)
    pub thematic_break: bool,
    /// Use single tilde for strikethrough (~text~)
    pub gfm_strikethrough_single_tilde: bool,
    /// Use single dollar for math ($x$)
    pub math_text_single_dollar: bool,
}

impl MdToHtmlOptions {
    /// Create options with all constructs enabled (default preset)
    pub fn new() -> Self {
        Self {
            gfm: false,
            autolink: true,
            code_indented: true,
            code_fenced: true,
            definition: true,
            frontmatter: false,
            gfm_autolink_literal: false,
            gfm_footnote_definition: false,
            gfm_label_start_footnote: false,
            gfm_strikethrough: false,
            gfm_table: false,
            gfm_task_list_item: false,
            hard_break_escape: true,
            hard_break_trailing: true,
            html_flow: true,
            html_text: true,
            label_end: true,
            label_start_image: true,
            label_start_link: true,
            list_item: true,
            math_flow: false,
            math_text: false,
            thematic_break: true,
            gfm_strikethrough_single_tilde: false,
            math_text_single_dollar: true,
        }
    }

    /// Create options with GFM preset (enables GFM extensions)
    pub fn gfm() -> Self {
        Self {
            gfm: true,
            autolink: true,
            code_indented: true,
            code_fenced: true,
            definition: true,
            frontmatter: false,
            gfm_autolink_literal: true,
            gfm_footnote_definition: true,
            gfm_label_start_footnote: true,
            gfm_strikethrough: true,
            gfm_table: true,
            gfm_task_list_item: true,
            hard_break_escape: true,
            hard_break_trailing: true,
            html_flow: true,
            html_text: true,
            label_end: true,
            label_start_image: true,
            label_start_link: true,
            list_item: true,
            math_flow: false,
            math_text: false,
            thematic_break: true,
            gfm_strikethrough_single_tilde: false,
            math_text_single_dollar: true,
        }
    }

    fn to_markdown_options(&self) -> markdown::Options {
        if self.gfm {
            let mut opts = markdown::Options::gfm();
            opts.parse.gfm_strikethrough_single_tilde = self.gfm_strikethrough_single_tilde;
            opts.parse.math_text_single_dollar = self.math_text_single_dollar;
            opts
        } else {
            let constructs = markdown::Constructs {
                autolink: self.autolink,
                code_indented: self.code_indented,
                code_fenced: self.code_fenced,
                definition: self.definition,
                frontmatter: self.frontmatter,
                gfm_autolink_literal: self.gfm_autolink_literal,
                gfm_footnote_definition: self.gfm_footnote_definition,
                gfm_label_start_footnote: self.gfm_label_start_footnote,
                gfm_strikethrough: self.gfm_strikethrough,
                gfm_table: self.gfm_table,
                gfm_task_list_item: self.gfm_task_list_item,
                hard_break_escape: self.hard_break_escape,
                hard_break_trailing: self.hard_break_trailing,
                html_flow: self.html_flow,
                html_text: self.html_text,
                label_end: self.label_end,
                label_start_image: self.label_start_image,
                label_start_link: self.label_start_link,
                list_item: self.list_item,
                math_flow: self.math_flow,
                math_text: self.math_text,
                thematic_break: self.thematic_break,
                ..Default::default()
            };
            markdown::Options {
                parse: markdown::ParseOptions {
                    constructs,
                    gfm_strikethrough_single_tilde: self.gfm_strikethrough_single_tilde,
                    math_text_single_dollar: self.math_text_single_dollar,
                    ..Default::default()
                },
                compile: markdown::CompileOptions::default(),
            }
        }
    }
}

/// Convert markdown string to HTML
pub fn convert_md_to_html(input: &str, options: &MdToHtmlOptions) -> Result<String> {
    let md_options = options.to_markdown_options();
    markdown::to_html_with_options(input, &md_options)
        .map_err(|e| AppError::MarkdownConvert(e.to_string()))
}

/// Convert markdown file to HTML file
pub fn convert_md_file_to_html(
    input_path: &str,
    output_path: Option<&str>,
    options: &MdToHtmlOptions,
) -> Result<String> {
    let input = Path::new(input_path);

    // Read input file
    let content = fs::read_to_string(input).map_err(|e| AppError::ReadFile {
        path: input_path.to_string(),
        source: e,
    })?;

    // Convert to HTML
    let html = convert_md_to_html(&content, options)?;

    // Determine output path
    let output = match output_path {
        Some(p) => p.to_string(),
        None => {
            let stem = input.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
            let parent = input.parent().unwrap_or(Path::new("."));
            parent.join(format!("{}.html", stem)).to_string_lossy().to_string()
        }
    };

    // Write output file
    fs::write(&output, &html).map_err(|e| AppError::WriteFile {
        path: output.clone(),
        source: e,
    })?;

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_basic_markdown() {
        let input = "# Title\n\nHello **world**!";
        let options = MdToHtmlOptions::new();
        let html = convert_md_to_html(input, &options).unwrap();

        assert!(html.contains("<h1>Title</h1>"));
        assert!(html.contains("<strong>world</strong>"));
    }

    #[test]
    fn test_convert_gfm_table() {
        let input = "| A | B |\n|---|---|\n| 1 | 2 |";
        let options = MdToHtmlOptions::gfm();
        let html = convert_md_to_html(input, &options).unwrap();

        assert!(html.contains("<table>"));
        assert!(html.contains("<th>A</th>"));
    }

    #[test]
    fn test_convert_gfm_strikethrough() {
        let input = "~~deleted~~";
        let options = MdToHtmlOptions::gfm();
        let html = convert_md_to_html(input, &options).unwrap();

        assert!(html.contains("<del>deleted</del>"));
    }

    #[test]
    fn test_convert_gfm_task_list() {
        let input = "- [x] Done\n- [ ] Todo";
        let options = MdToHtmlOptions::gfm();
        let html = convert_md_to_html(input, &options).unwrap();

        assert!(html.contains("checked"));
    }

    #[test]
    fn test_convert_code_block() {
        let input = "```rust\nfn main() {}\n```";
        let options = MdToHtmlOptions::new();
        let html = convert_md_to_html(input, &options).unwrap();

        assert!(html.contains("<code"));
        assert!(html.contains("fn main()"));
    }

    #[test]
    fn test_options_default() {
        let options = MdToHtmlOptions::new();
        assert!(!options.gfm);
        assert!(options.code_fenced);
        assert!(options.autolink);
    }

    #[test]
    fn test_options_gfm() {
        let options = MdToHtmlOptions::gfm();
        assert!(options.gfm);
        assert!(options.gfm_table);
        assert!(options.gfm_strikethrough);
        assert!(options.gfm_task_list_item);
    }

    #[test]
    fn test_convert_list() {
        let input = "* Item 1\n* Item 2";
        let options = MdToHtmlOptions::new();
        let html = convert_md_to_html(input, &options).unwrap();

        assert!(html.contains("<ul>"));
        assert!(html.contains("<li>Item 1</li>"));
    }

    #[test]
    fn test_convert_link() {
        let input = "[text](https://example.com)";
        let options = MdToHtmlOptions::new();
        let html = convert_md_to_html(input, &options).unwrap();

        assert!(html.contains("<a href=\"https://example.com\">text</a>"));
    }

    #[test]
    fn test_convert_image() {
        let input = "![alt](image.png)";
        let options = MdToHtmlOptions::new();
        let html = convert_md_to_html(input, &options).unwrap();

        assert!(html.contains("<img"));
        assert!(html.contains("src=\"image.png\""));
        assert!(html.contains("alt=\"alt\""));
    }
}
