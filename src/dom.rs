/// DOM ağacı veri yapıları
use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};

static NODE_ID_COUNTER: AtomicUsize = AtomicUsize::new(1);

/// Benzersiz düğüm kimliği
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub usize);

impl NodeId {
    pub fn new() -> Self {
        NodeId(NODE_ID_COUNTER.fetch_add(1, Ordering::SeqCst))
    }

    pub const fn zero() -> Self {
        NodeId(0)
    }
}

/// Düğüm türü
#[derive(Debug, Clone, PartialEq)]
pub enum NodeType {
    Document,
    Element(ElementData),
    Text(String),
    Comment(String),
}

/// Element verisi
#[derive(Debug, Clone, PartialEq)]
pub struct ElementData {
    pub tag_name: String,
    pub attributes: HashMap<String, String>,
    pub namespace: String,
}

impl ElementData {
    pub fn new(tag_name: &str) -> Self {
        Self {
            tag_name: tag_name.to_lowercase(),
            attributes: HashMap::new(),
            namespace: String::new(),
        }
    }

    pub fn id(&self) -> Option<&str> {
        self.attributes.get("id").map(|s| s.as_str())
    }

    pub fn classes(&self) -> Vec<&str> {
        self.attributes
            .get("class")
            .map(|s| s.split_whitespace().collect())
            .unwrap_or_default()
    }

    pub fn has_class(&self, class: &str) -> bool {
        self.attributes
            .get("class")
            .map(|s| s.split_whitespace().any(|c| c == class))
            .unwrap_or(false)
    }

    pub fn get_attr(&self, name: &str) -> Option<&str> {
        self.attributes.get(name).map(|s| s.as_str())
    }
}

/// DOM düğümü
#[derive(Debug, Clone)]
pub struct Node {
    pub id: NodeId,
    pub node_type: NodeType,
    pub children: Vec<Node>,
    pub parent: Option<NodeId>,
}

impl Node {
    pub fn new(node_type: NodeType) -> Self {
        Self {
            id: NodeId::new(),
            node_type,
            children: Vec::new(),
            parent: None,
        }
    }

    pub fn document() -> Self {
        Self::new(NodeType::Document)
    }

    pub fn element(tag_name: &str) -> Self {
        Self::new(NodeType::Element(ElementData::new(tag_name)))
    }

    pub fn text(content: &str) -> Self {
        Self::new(NodeType::Text(content.to_string()))
    }

    pub fn is_element(&self) -> bool {
        matches!(self.node_type, NodeType::Element(_))
    }

    pub fn is_text(&self) -> bool {
        matches!(self.node_type, NodeType::Text(_))
    }

    pub fn is_document(&self) -> bool {
        matches!(self.node_type, NodeType::Document)
    }

    pub fn as_element(&self) -> Option<&ElementData> {
        match &self.node_type {
            NodeType::Element(e) => Some(e),
            _ => None,
        }
    }

    pub fn as_element_mut(&mut self) -> Option<&mut ElementData> {
        match &mut self.node_type {
            NodeType::Element(e) => Some(e),
            _ => None,
        }
    }

    pub fn as_text(&self) -> Option<&str> {
        match &self.node_type {
            NodeType::Text(t) => Some(t.as_str()),
            _ => None,
        }
    }

    pub fn tag_name(&self) -> Option<&str> {
        self.as_element().map(|e| e.tag_name.as_str())
    }

    pub fn append_child(&mut self, child: Node) {
        self.children.push(child);
    }

    pub fn append_child_owned(&mut self, mut child: Node) {
        child.parent = Some(self.id);
        self.children.push(child);
    }

    /// Tüm alt düğümleri (kendisi dahil) recursive dolaş
    pub fn descendants(&self) -> Vec<&Node> {
        let mut result = vec![self];
        for child in &self.children {
            result.extend(child.descendants());
        }
        result
    }

    /// Elementleri recursive bul
    pub fn elements_by_tag(&self, tag: &str) -> Vec<&Node> {
        let mut result = Vec::new();
        if self.tag_name() == Some(tag) {
            result.push(self);
        }
        for child in &self.children {
            result.extend(child.elements_by_tag(tag));
        }
        result
    }

    /// ID'ye göre element bul
    pub fn get_element_by_id(&self, id: &str) -> Option<&Node> {
        if let Some(e) = self.as_element() {
            if e.id() == Some(id) {
                return Some(self);
            }
        }
        for child in &self.children {
            if let Some(found) = child.get_element_by_id(id) {
                return Some(found);
            }
        }
        None
    }

    /// Metin içeriğini topla
    pub fn text_content(&self) -> String {
        let mut s = String::new();
        for child in &self.children {
            match &child.node_type {
                NodeType::Text(t) => s.push_str(t),
                NodeType::Element(_) => s.push_str(&child.text_content()),
                _ => {}
            }
        }
        s
    }
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.node_type {
            NodeType::Document => write!(f, "#document")?,
            NodeType::Element(el) => {
                write!(f, "<{}", el.tag_name)?;
                for (name, val) in &el.attributes {
                    write!(f, " {}=\"{}\"", name, val)?;
                }
                write!(f, ">")?;
            }
            NodeType::Text(t) => {
                let trimmed = t.trim();
                if !trimmed.is_empty() {
                    write!(f, "\"{}\"", trimmed)?;
                }
            }
            NodeType::Comment(c) => write!(f, "<!-- {} -->", c)?,
        }
        Ok(())
    }
}

/// DOM'u hiyerarşik yazdır
pub fn print_dom(node: &Node, indent: usize) {
    let prefix = "  ".repeat(indent);
    match &node.node_type {
        NodeType::Document => {
            println!("{}📄 #document", prefix);
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
                println!("{}📝 \"{}\"", prefix, trimmed);
            }
        }
        NodeType::Comment(c) => {
            println!("{}💬 <!-- {} -->", prefix, c);
        }
    }
}
