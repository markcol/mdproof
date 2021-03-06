
use pdf_canvas::{BuiltinFont, FontSource};
use cmark::{Event, Tag};
use {
    DEFAULT_FONT, DEFAULT_FONT_SIZE, ITALIC_FONT, BOLD_FONT,
    H1_FONT_SIZE, H2_FONT_SIZE, H3_FONT_SIZE, H4_FONT_SIZE,
    QUOTE_INDENTATION, LIST_INDENTATION,
};
use span::Span;
use section::Section;

pub enum SubsectionType {
    List,
    Quote,
}

pub struct Sectioner {
    pub x: f32,
    lines: Vec<Section>,
    current_line: Vec<Span>,
    current_font: BuiltinFont,
    current_size: f32,
    max_width: f32,
    subsection: Option<Box<Sectioner>>,
    pub is_code: bool,
}

impl Sectioner {
    pub fn new(max_width: f32) -> Self {
        Self {
            x: 0.0,
            lines: Vec::new(),
            current_line: Vec::new(),
            current_font: DEFAULT_FONT,
            current_size: DEFAULT_FONT_SIZE,
            max_width: max_width,
            subsection: None,
            is_code: false,
        }
    }

    pub fn parse_event(&mut self, event: Event) -> Option<SubsectionType> {
        if self.subsection.is_some() {
            let mut subsection = self.subsection.take().expect("Checked if the subsection was `Some`");
            if let Some(sub_type) = subsection.parse_event(event) {
                let section = match sub_type {
                    SubsectionType::List => Section::list_item(subsection.get_vec()),
                    SubsectionType::Quote => Section::block_quote(subsection.get_vec()),
                };
                self.push_section(section);
            } else {
                self.subsection = Some(subsection);
            };
            return None;
        }
        match event {
            Event::Start(Tag::Strong) => self.current_font = BOLD_FONT,
            Event::End(Tag::Strong) => self.current_font = DEFAULT_FONT,
            Event::Start(Tag::Emphasis) => self.current_font = ITALIC_FONT,
            Event::End(Tag::Emphasis) => self.current_font = DEFAULT_FONT,

            Event::Start(Tag::Header(size)) => self.current_size = match size {
                1 => H1_FONT_SIZE,
                2 => H2_FONT_SIZE,
                3 => H3_FONT_SIZE,
                _ => H4_FONT_SIZE,
            },
            Event::End(Tag::Header(_)) => {
                self.current_size = DEFAULT_FONT_SIZE;
                self.new_line();
            },

            Event::Start(Tag::List(_)) => self.new_line(),
            Event::Start(Tag::Item) => self.subsection = Some(Box::new(Sectioner::new(self.max_width - LIST_INDENTATION))),
            Event::End(Tag::Item) => return Some(SubsectionType::List),

            Event::Start(Tag::BlockQuote) => {
                self.new_line();
                self.subsection = Some(Box::new(Sectioner::new(self.max_width - QUOTE_INDENTATION)))
            },
            Event::End(Tag::BlockQuote) => return Some(SubsectionType::Quote),

            Event::Text(ref text) if self.is_code => {
                let mut start = 0;
                for (pos, c) in text.chars().enumerate() {
                    if c == '\n' {
                        self.write(&text[start..pos]);
                        self.new_line();
                        start = pos + 1;
                    }
                }
                if start < text.len() {
                        self.write(&text[start..]);
                }
            },
            Event::Text(text) => self.write_left_aligned(&text),

            Event::Start(Tag::Code) => self.current_font = BuiltinFont::Courier,
            Event::End(Tag::Code) => self.current_font = DEFAULT_FONT,

            Event::Start(Tag::CodeBlock(_src_type)) => {
                self.is_code = true;
                self.current_font = BuiltinFont::Courier;
                self.current_size = DEFAULT_FONT_SIZE;
            },
            Event::End(Tag::CodeBlock(_)) => {
                self.push_section(Section::space(DEFAULT_FONT_SIZE));
                self.is_code = false;
                self.current_font = DEFAULT_FONT;
            },

            Event::Start(Tag::Paragraph) => {},
            Event::End(Tag::Paragraph) => {
                self.new_line();
                self.push_section(Section::space(DEFAULT_FONT_SIZE));
            },

            Event::SoftBreak => self.write(" "),
            Event::HardBreak => self.new_line(),

            _ => {}
        };
        None
    }

    pub fn push_section(&mut self, section: Section) {
        self.lines.push(section);
    }

    pub fn write_left_aligned(&mut self, text: &str) {
        let space_width = self.current_font.get_width(self.current_size, " ");

        let mut buffer = String::new();
        let mut buffer_width = 0.0;
        let mut pos = 0;
        while pos < text.len() {
            let idx = text[pos..].find(char::is_whitespace).unwrap_or(text.len()-pos-1)+pos+1;
            let word = &text[pos..idx];
            pos = idx;
            let word_width = self.current_font.get_width(self.current_size, word);
            if self.x + buffer_width + word_width > self.max_width {
                self.write(&buffer);
                self.new_line();
                buffer.clear();
                buffer_width = 0.0;
            }
            if buffer.len() > 0 {
                buffer.push(' ');
                buffer_width += space_width;
            }
            buffer.push_str(word);
            buffer_width += word_width;
        }
        let span = Span::text(buffer, self.current_font, self.current_size);
        self.push_span(span);
    }

    pub fn write(&mut self, text: &str) {
        let span = Span::text(text.into(), self.current_font, self.current_size);
        self.push_span(span);
    }

    pub fn push_span(&mut self, span: Span) {
        self.x += span.width();
        self.current_line.push(span);
    }

    pub fn new_line(&mut self) {
        if self.current_line.len() == 0 { return }
        self.lines.push(Section::plain(self.current_line.clone()));
        self.current_line.clear();
        self.x = 0.0;
    }

    pub fn get_vec(mut self) -> Vec<Section> {
        // Make sure that current_line is put into the output
        if self.current_line.len() != 0 {
            self.lines.push(Section::plain(self.current_line));
        }
        self.lines
    }
}
