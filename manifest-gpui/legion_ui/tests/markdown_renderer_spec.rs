use speculate2::speculate;

speculate! {
    use legion_ui::MarkdownRenderer;

    describe "markdown renderer" {
        describe "heading parsing" {
            it "parses h1 headings" {
                let blocks = MarkdownRenderer::parse("# Hello World");
                assert_eq!(blocks.len(), 1);
                assert!(matches!(&blocks[0], legion_ui::MarkdownBlock::Heading { level: 1, text } if text == "Hello World"));
            }

            it "parses h2 through h6 headings" {
                for level in 2..=6 {
                    let prefix = "#".repeat(level);
                    let markdown = format!("{} Heading {}", prefix, level);
                    let blocks = MarkdownRenderer::parse(&markdown);
                    assert_eq!(blocks.len(), 1);
                    if let legion_ui::MarkdownBlock::Heading { level: parsed_level, text } = &blocks[0] {
                        assert_eq!(*parsed_level, level);
                        assert_eq!(text, &format!("Heading {}", level));
                    } else {
                        panic!("Expected heading block");
                    }
                }
            }
        }

        describe "paragraph parsing" {
            it "parses simple paragraphs" {
                let blocks = MarkdownRenderer::parse("This is a paragraph.");
                assert_eq!(blocks.len(), 1);
                assert!(matches!(&blocks[0], legion_ui::MarkdownBlock::Paragraph(text) if text == "This is a paragraph."));
            }

            it "joins consecutive lines into one paragraph" {
                let blocks = MarkdownRenderer::parse("Line one\nLine two");
                assert_eq!(blocks.len(), 1);
                if let legion_ui::MarkdownBlock::Paragraph(text) = &blocks[0] {
                    assert!(text.contains("Line one"));
                    assert!(text.contains("Line two"));
                }
            }
        }

        describe "code block parsing" {
            it "parses code blocks with language" {
                let markdown = "```rust\nfn main() {}\n```";
                let blocks = MarkdownRenderer::parse(markdown);
                assert_eq!(blocks.len(), 1);
                if let legion_ui::MarkdownBlock::CodeBlock { language, code } = &blocks[0] {
                    assert_eq!(language.as_deref(), Some("rust"));
                    assert!(code.contains("fn main()"));
                } else {
                    panic!("Expected code block");
                }
            }

            it "parses code blocks without language" {
                let markdown = "```\nsome code\n```";
                let blocks = MarkdownRenderer::parse(markdown);
                assert_eq!(blocks.len(), 1);
                if let legion_ui::MarkdownBlock::CodeBlock { language, code } = &blocks[0] {
                    assert!(language.is_none() || language.as_deref() == Some(""));
                    assert!(code.contains("some code"));
                } else {
                    panic!("Expected code block");
                }
            }
        }

        describe "list parsing" {
            it "parses unordered lists with dashes" {
                let markdown = "- Item 1\n- Item 2\n- Item 3";
                let blocks = MarkdownRenderer::parse(markdown);
                assert_eq!(blocks.len(), 1);
                if let legion_ui::MarkdownBlock::UnorderedList(items) = &blocks[0] {
                    assert_eq!(items.len(), 3);
                    assert_eq!(items[0], "Item 1");
                    assert_eq!(items[1], "Item 2");
                    assert_eq!(items[2], "Item 3");
                } else {
                    panic!("Expected unordered list");
                }
            }

            it "parses unordered lists with asterisks" {
                let markdown = "* Item A\n* Item B";
                let blocks = MarkdownRenderer::parse(markdown);
                assert_eq!(blocks.len(), 1);
                if let legion_ui::MarkdownBlock::UnorderedList(items) = &blocks[0] {
                    assert_eq!(items.len(), 2);
                } else {
                    panic!("Expected unordered list");
                }
            }

            it "parses ordered lists" {
                let markdown = "1. First\n2. Second\n3. Third";
                let blocks = MarkdownRenderer::parse(markdown);
                assert_eq!(blocks.len(), 1);
                if let legion_ui::MarkdownBlock::OrderedList(items) = &blocks[0] {
                    assert_eq!(items.len(), 3);
                    assert_eq!(items[0], "First");
                } else {
                    panic!("Expected ordered list");
                }
            }
        }

        describe "blockquote parsing" {
            it "parses single line blockquotes" {
                let markdown = "> This is a quote";
                let blocks = MarkdownRenderer::parse(markdown);
                assert_eq!(blocks.len(), 1);
                if let legion_ui::MarkdownBlock::Blockquote(text) = &blocks[0] {
                    assert!(text.contains("This is a quote"));
                } else {
                    panic!("Expected blockquote");
                }
            }
        }

        describe "horizontal rule parsing" {
            it "parses triple dashes as horizontal rule" {
                let blocks = MarkdownRenderer::parse("---");
                assert_eq!(blocks.len(), 1);
                assert!(matches!(&blocks[0], legion_ui::MarkdownBlock::HorizontalRule));
            }

            it "parses triple asterisks as horizontal rule" {
                let blocks = MarkdownRenderer::parse("***");
                assert_eq!(blocks.len(), 1);
                assert!(matches!(&blocks[0], legion_ui::MarkdownBlock::HorizontalRule));
            }

            it "parses triple underscores as horizontal rule" {
                let blocks = MarkdownRenderer::parse("___");
                assert_eq!(blocks.len(), 1);
                assert!(matches!(&blocks[0], legion_ui::MarkdownBlock::HorizontalRule));
            }
        }

        describe "mixed content" {
            it "parses document with multiple block types" {
                let markdown = r#"# Title

This is a paragraph.

- Item 1
- Item 2

```rust
fn main() {}
```

> A quote
"#;
                let blocks = MarkdownRenderer::parse(markdown);

                // Should have heading, paragraph, list, code block, and blockquote
                assert!(blocks.len() >= 4);

                // First should be heading
                assert!(matches!(&blocks[0], legion_ui::MarkdownBlock::Heading { level: 1, .. }));
            }
        }
    }
}
