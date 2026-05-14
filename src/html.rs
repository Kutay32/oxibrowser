/// HTML parser - kendi implementasyonumuz (SimpleHtmlParser)
/// html5ever kullanmak yerine kendi parser'ımızı yazıyoruz
/// çünkü html5ever'ın TreeSink trait'i sürümler arası çok değişiyor.
use crate::dom::{ElementData, Node, NodeType};

/// HTML metnini DOM ağacına parse et
pub fn parse_html_simple(html: &str) -> Node {
    let mut parser = SimpleHtmlParser::new(html);
    parser.parse()
}

/// Basit HTML parser
struct SimpleHtmlParser<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> SimpleHtmlParser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    fn remaining(&self) -> &'a str {
        &self.input[self.pos..]
    }

    fn peek(&self) -> Option<char> {
        self.remaining().chars().next()
    }

    fn advance(&mut self) {
        if let Some(c) = self.remaining().chars().next() {
            self.pos += c.len_utf8();
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_ascii_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn starts_with(&self, s: &str) -> bool {
        self.remaining().starts_with(s)
    }

    fn parse(&mut self) -> Node {
        let mut doc = Node::document();
        let mut html = Node::element("html");
        let mut head = Node::element("head");
        let mut body = Node::element("body");

        while self.pos < self.input.len() {
            self.skip_whitespace();

            if self.starts_with("<!") {
                if self.starts_with("<!--") {
                    self.skip_comment(&mut doc);
                } else {
                    self.skip_doctype();
                }
            } else if self.starts_with("</") {
                self.skip_tag();
            } else if self.starts_with("<") {
                let tag = self.parse_element();
                let tag_name = tag.tag_name().unwrap_or("").to_string();
                match tag_name.as_str() {
                    "html" => html = tag,
                    "head" => head = tag,
                    "body" => body = tag,
                    _ => {
                        let head_tags = [
                            "meta", "link", "title", "style", "script", "base", "noscript",
                        ];
                        if head_tags.contains(&tag_name.as_str()) {
                            head.append_child_owned(tag);
                        } else {
                            body.append_child_owned(tag);
                        }
                    }
                }
            } else {
                // Metin
                let text = self.parse_text();
                if !text.trim().is_empty() {
                    body.append_child_owned(Node::text(&text));
                }
            }
        }

        html.append_child_owned(head);
        html.append_child_owned(body);
        doc.append_child_owned(html);
        doc
    }

    fn skip_comment(&mut self, parent: &mut Node) {
        if self.starts_with("<!--") {
            self.pos += 4;
            let end = self
                .remaining()
                .find("-->")
                .unwrap_or(self.input.len() - self.pos);
            let comment = &self.input[self.pos..self.pos + end];
            self.pos += end + 3;
            parent
                .children
                .push(Node::new(NodeType::Comment(comment.to_string())));
        }
    }

    fn skip_doctype(&mut self) {
        if self.starts_with("<!") {
            let end = self
                .remaining()
                .find('>')
                .unwrap_or(self.input.len() - self.pos);
            self.pos += end + 1;
        }
    }

    fn skip_tag(&mut self) {
        let end = self
            .remaining()
            .find('>')
            .unwrap_or(self.input.len() - self.pos);
        self.pos += end + 1;
    }

    fn parse_element(&mut self) -> Node {
        self.pos += 1; // skip <
        let tag_name = self.read_tag_name();
        let mut self_closing = false;

        let mut element = ElementData::new(&tag_name);
        loop {
            self.skip_whitespace();
            match self.peek() {
                Some('>') => {
                    self.pos += 1;
                    break;
                }
                Some('/') if self.remaining().starts_with("/>") => {
                    self.pos += 2;
                    self_closing = true;
                    break;
                }
                Some(c) if !c.is_ascii_whitespace() && c != '>' => {
                    let (name, value) = self.parse_attribute();
                    if !name.is_empty() {
                        element.attributes.insert(name, value);
                    }
                }
                _ => {
                    self.advance();
                }
            }
        }

        let mut node = Node::new(NodeType::Element(element));

        let void_elements = [
            "br", "hr", "img", "input", "meta", "link", "area", "base", "col", "embed", "source",
            "track", "wbr",
        ];

        if self_closing || void_elements.contains(&tag_name.as_str()) {
            return node;
        }

        let closing_tag = format!("</{}", tag_name);
        loop {
            if self.pos >= self.input.len() {
                break;
            }
            if self.starts_with(&closing_tag) {
                let end = self
                    .remaining()
                    .find('>')
                    .unwrap_or(self.input.len() - self.pos);
                self.pos += end + 1;
                break;
            }
            if self.starts_with("<!--") {
                self.skip_comment(&mut node);
                continue;
            }
            if self.starts_with("<") {
                let child = self.parse_element();
                node.append_child_owned(child);
                continue;
            }
            let text = self.parse_text();
            if !text.is_empty() {
                node.append_child_owned(Node::text(&text));
            }
        }

        node
    }

    fn read_tag_name(&mut self) -> String {
        let mut name = String::new();
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == ':' {
                name.push(c);
                self.advance();
            } else {
                break;
            }
        }
        name.to_lowercase()
    }

    fn parse_attribute(&mut self) -> (String, String) {
        self.skip_whitespace();
        let name = self.read_tag_name();
        if name.is_empty() {
            return (String::new(), String::new());
        }

        self.skip_whitespace();
        if self.peek() != Some('=') {
            return (name, String::new());
        }
        self.pos += 1;
        self.skip_whitespace();

        let value = match self.peek() {
            Some('"') => {
                self.pos += 1;
                let end = self
                    .remaining()
                    .find('"')
                    .unwrap_or(self.input.len() - self.pos);
                let v = self.input[self.pos..self.pos + end].to_string();
                self.pos += end + 1;
                v
            }
            Some('\'') => {
                self.pos += 1;
                let end = self
                    .remaining()
                    .find('\'')
                    .unwrap_or(self.input.len() - self.pos);
                let v = self.input[self.pos..self.pos + end].to_string();
                self.pos += end + 1;
                v
            }
            Some(_) => {
                let mut v = String::new();
                while let Some(c) = self.peek() {
                    if c.is_ascii_whitespace() || c == '>' || c == '/' {
                        break;
                    }
                    v.push(c);
                    self.advance();
                }
                v
            }
            None => String::new(),
        };

        let value = decode_html_entities(&value);
        (name, value)
    }

    fn parse_text(&mut self) -> String {
        let mut text = String::new();
        while let Some(c) = self.peek() {
            if c == '<' {
                break;
            }
            text.push(c);
            self.advance();
        }
        decode_html_entities(&text)
    }
}

/// HTML entity decoder
fn decode_html_entities(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '&' {
            let mut entity = String::new();
            while let Some(&next) = chars.peek() {
                if next == ';' {
                    chars.next();
                    break;
                }
                entity.push(next);
                chars.next();
            }

            let decoded = match entity.as_str() {
                "amp" => "&",
                "lt" => "<",
                "gt" => ">",
                "quot" => "\"",
                "apos" => "'",
                "nbsp" => "\u{00A0}",
                "copy" => "\u{00A9}",
                "reg" => "\u{00AE}",
                "#x27" => "'",
                "#x2F" => "/",
                "#39" => "'",
                _ => {
                    if let Some(num) = entity.strip_prefix('#') {
                        if let Ok(code) = num.parse::<u32>() {
                            if let Some(ch) = char::from_u32(code) {
                                result.push(ch);
                                continue;
                            }
                        }
                    }
                    result.push('&');
                    result.push_str(&entity);
                    result.push(';');
                    continue;
                }
            };
            result.push_str(decoded);
        } else {
            result.push(c);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom::NodeType;

    #[test]
    fn decodes_entities_in_text_nodes() {
        let dom = parse_html_simple("<p>Tom &amp; Jerry &lt;3</p>");
        assert!(dom.text_content().contains("Tom & Jerry <3"));
    }

    #[test]
    fn assigns_parent_to_text_nodes() {
        let dom = parse_html_simple("<a href=\"/x\">Docs</a>");
        let body = dom.elements_by_tag("body")[0];
        let anchor = body.elements_by_tag("a")[0];
        let text = anchor
            .children
            .iter()
            .find(|child| matches!(child.node_type, NodeType::Text(_)))
            .unwrap();
        assert_eq!(text.parent, Some(anchor.id));
    }
}

/// DOM'u hiyerarşik yazdır (debug)
pub fn print_dom(node: &Node, indent: usize) {
    let prefix = "  ".repeat(indent);
    match &node.node_type {
        NodeType::Document => {
            println!("{}#document", prefix);
            for child in &node.children {
                print_dom(child, indent + 1);
            }
        }
        NodeType::Element(el) => {
            let id_attr = el.id().map(|id| format!("#{}", id)).unwrap_or_default();
            let class_attr = if el.attributes.contains_key("class") {
                format!(".{}", el.attributes["class"].replace(' ', "."))
            } else {
                String::new()
            };
            println!("{}<{}{}{}>", prefix, el.tag_name, id_attr, class_attr);
            for child in &node.children {
                print_dom(child, indent + 1);
            }
        }
        NodeType::Text(t) => {
            let trimmed = t.trim();
            if !trimmed.is_empty() {
                println!("{}\"{}\"", prefix, trimmed);
            }
        }
        NodeType::Comment(c) => {
            println!("{}<!-- {} -->", prefix, c);
        }
    }
}
