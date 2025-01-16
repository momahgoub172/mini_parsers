use std::collections::HashMap;
use serde_json::{Map, Value};

#[derive(Debug, Clone)]
pub struct XmlNode {
    tag: String,
    attributes: HashMap<String, String>,
    children: Vec<XmlNode>,
    text: Option<String>,
}

impl XmlNode {
    fn new(tag: String) -> Self {
        XmlNode {
            tag,
            attributes: HashMap::new(),
            children: Vec::new(),
            text: None,
        }
    }

    pub fn to_json(&self) -> Value {
        let mut map = Map::new();

        // Handle attributes
        if !self.attributes.is_empty() {
            let mut attrs = Map::new();
            for (key, value) in self.attributes.iter() {
                attrs.insert(key.clone(), Value::String(value.clone()));
            }
            map.insert("@attributes".to_string(), Value::Object(attrs));
        }

        // Handle text
        if let Some(text) = &self.text {
            if self.children.is_empty() && self.attributes.is_empty() {
                return Value::String(text.clone());
            } else {
                map.insert("#text".to_string(), Value::String(text.clone()));
            }
        }

        // Handle children
        let mut children_map: HashMap<String, Vec<Value>> = HashMap::new();
        for child in &self.children {
            children_map
                .entry(child.tag.clone())
                .or_insert_with(Vec::new)
                .push(child.to_json());
        }

        for (tag, values) in children_map {
            let json_val = if values.len() == 1 {
                values.into_iter().next().unwrap()
            } else {
                Value::Array(values)
            };
            map.insert(tag, json_val);
        }

        if map.is_empty() {
            return Value::Null;
        }

        Value::Object(map)
    }
}

pub struct XmlParser {
    input: Vec<char>,
    position: usize,
}

impl XmlParser {
    pub fn new(input: &str) -> Self {
        XmlParser {
            input: input.chars().collect(),
            position: 0,
        }
    }

    pub fn parse(&mut self) -> Result<XmlNode, String> {
        self.skip_whitespace();
        self.expect_char('<')?;
        
        let tag = self.parse_tag_name()?;
        let mut node = XmlNode::new(tag);
        
        // Parse attributes
        node.attributes = self.parse_attributes()?;
        
        // Check if it's a self-closing tag
        self.skip_whitespace();
        if self.peek_char() == Some('/') {
            self.next_char();
            self.expect_char('>')?;
            return Ok(node);
        }
        
        self.expect_char('>')?;
        
        // Parse content (text and child nodes)
        loop {
            self.skip_whitespace();
            
            if self.peek_char() == Some('<') {
                if self.peek_next_char() == Some('/') {
                    self.next_char(); // Skip '<'
                    self.next_char(); // Skip '/'
                    let close_tag = self.parse_tag_name()?;
                    
                    if close_tag != node.tag {
                        return Err(format!("Mismatched tags: {} and {}", node.tag, close_tag));
                    }
                    
                    self.expect_char('>')?;
                    break;
                } else {
                    let child = self.parse()?;
                    node.children.push(child);
                }
            } else {
                let text = self.parse_text()?;
                if !text.trim().is_empty() {
                    node.text = Some(text);
                }
            }
        }
        
        Ok(node)
    }
    
    fn parse_tag_name(&mut self) -> Result<String, String> {
        let mut name = String::new();
        
        while let Some(c) = self.peek_char() {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                name.push(self.next_char().unwrap());
            } else {
                break;
            }
        }
        
        if name.is_empty() {
            return Err("Expected tag name".to_string());
        }
        
        Ok(name)
    }
    
    fn parse_attributes(&mut self) -> Result<HashMap<String, String>, String> {
        let mut attributes = HashMap::new();
        
        loop {
            self.skip_whitespace();
            
            if self.peek_char() == Some('>') || self.peek_char() == Some('/') {
                break;
            }
            
            let name = self.parse_tag_name()?;
            self.skip_whitespace();
            self.expect_char('=')?;
            self.skip_whitespace();
            self.expect_char('"')?;
            
            let value = self.parse_attribute_value()?;
            attributes.insert(name, value);
        }
        
        Ok(attributes)
    }
    
    fn parse_attribute_value(&mut self) -> Result<String, String> {
        let mut value = String::new();
        
        while let Some(c) = self.next_char() {
            if c == '"' {
                return Ok(value);
            }
            value.push(c);
        }
        
        Err("Unterminated attribute value".to_string())
    }
    
    fn parse_text(&mut self) -> Result<String, String> {
        let mut text = String::new();
        
        while let Some(c) = self.peek_char() {
            if c == '<' {
                break;
            }
            text.push(self.next_char().unwrap());
        }
        
        Ok(text)
    }
    
    fn peek_char(&self) -> Option<char> {
        self.input.get(self.position).copied()
    }

    fn peek_next_char(&self) -> Option<char> {
        self.input.get(self.position + 1).copied()
    }

    fn next_char(&mut self) -> Option<char> {
        let c = self.peek_char();
        self.position += 1;
        c
    }

    fn expect_char(&mut self, expected: char) -> Result<(), String> {
        match self.next_char() {
            Some(c) if c == expected => Ok(()),
            Some(c) => Err(format!("Expected '{}', found '{}'", expected, c)),
            None => Err(format!("Expected '{}', found end of input", expected)),
        }
    }
    
    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek_char() {
            if !c.is_whitespace() {
                break;
            }
            self.next_char();
        }
    }
}

pub enum JsonValue {
    Null,
    Boolean(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(HashMap<String, JsonValue>),
}

impl JsonValue {
    pub fn to_xml(&self) -> String {
        self.to_xml_with_tag("root")
    }

    fn to_xml_with_tag(&self, tag: &str) -> String {
        match self {
            JsonValue::Null => format!("<{}>", tag),
            JsonValue::Boolean(b) => format!("<{}>{}</{}>", tag, b, tag),
            JsonValue::Number(n) => format!("<{}>{}</{}>", tag, n, tag),
            JsonValue::String(s) => format!("<{}>{}</{}>", tag, escape_xml_text(s), tag),
            JsonValue::Array(arr) => {
                let mut xml = String::new();
                xml.push_str(&format!("<{}>", tag));
                for value in arr.iter() {
                    xml.push_str("  ");
                    xml.push_str(&value.to_xml_with_tag("item"));
                }
                xml.push_str(&format!("</{}>", tag));
                xml
            }
            JsonValue::Object(obj) => {
                let mut xml = String::new();
                xml.push_str(&format!("<{}>", tag));
                for (key, value) in obj {
                    xml.push_str("  ");
                    xml.push_str(&value.to_xml_with_tag(key));
                }
                xml.push_str(&format!("</{}>", tag));
                xml
            }
        }
    }
}

fn escape_xml_text(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub struct JsonParser {
    input: Vec<char>,
    position: usize,
}

impl JsonParser {
    pub fn new(input: &str) -> Self {
        JsonParser {
            input: input.chars().collect(),
            position: 0,
        }
    }

    pub fn parse(&mut self) -> Result<JsonValue, String> {
        let value = self.parse_value()?;
        self.skip_whitespace();
        if self.position < self.input.len() {
            return Err("Unexpected characters after JSON value".to_string());
        }
        Ok(value)
    }

    fn parse_null(&mut self) -> Result<JsonValue, String> {
        if self.input[self.position..].starts_with(&['n', 'u', 'l', 'l']) {
            self.position += 4;
            Ok(JsonValue::Null)
        } else {
            Err("Expected null".to_string())
        }
    }

    fn parse_boolean(&mut self) -> Result<JsonValue, String> {
        if self.input[self.position..].starts_with(&['t', 'r', 'u', 'e']) {
            self.position += 4;
            Ok(JsonValue::Boolean(true))
        } else if self.input[self.position..].starts_with(&['f', 'a', 'l', 's', 'e']) {
            self.position += 5;
            Ok(JsonValue::Boolean(false))
        } else {
            Err("Expected true or false".to_string())
        }
    }

    fn parse_string(&mut self) -> Result<JsonValue, String> {
        self.next_char(); // Skip opening quote
        let mut string = String::new();
        
        while let Some(c) = self.next_char() {
            match c {
                '"' => return Ok(JsonValue::String(string)),
                '\\' => {
                    if let Some(next) = self.next_char() {
                        match next {
                            '"' | '\\' | '/' => string.push(next),
                            'b' => string.push('\x08'),
                            'f' => string.push('\x0c'),
                            'n' => string.push('\n'),
                            'r' => string.push('\r'),
                            't' => string.push('\t'),
                            _ => return Err("Invalid escape sequence".to_string()),
                        }
                    }
                }
                _ => string.push(c),
            }
        }
        
        Err("Unterminated string".to_string())
    }

    fn parse_number(&mut self) -> Result<JsonValue, String> {
        let mut number = String::new();
        
        if self.peek_char() == Some('-') {
            number.push(self.next_char().unwrap());
        }
        
        while let Some(c) = self.peek_char() {
            if c.is_digit(10) {
                number.push(self.next_char().unwrap());
            } else {
                break;
            }
        }

        if self.peek_char() == Some('.') {
            number.push(self.next_char().unwrap());
            let mut has_digit = false;

            while let Some(c) = self.peek_char() {
                if c.is_digit(10) {
                    number.push(self.next_char().unwrap());
                    has_digit = true;
                } else {
                    break;
                }
            }
            
            if !has_digit {
                return Err("Expected digit after decimal point".to_string());
            }
        }

        if let Some('e') | Some('E') = self.peek_char() {
            number.push(self.next_char().unwrap());
            
            if let Some('+') | Some('-') = self.peek_char() {
                number.push(self.next_char().unwrap());
            }

            let mut has_digit = false;
            while let Some(c) = self.peek_char() {
                if c.is_digit(10) {
                    number.push(self.next_char().unwrap());
                    has_digit = true;
                } else {
                    break;
                }
            }
            
            if !has_digit {
                return Err("Expected digit after exponent".to_string());
            }
        }
        
        number.parse::<f64>()
            .map(JsonValue::Number)
            .map_err(|_| "Invalid number".to_string())
    }

    fn parse_array(&mut self) -> Result<JsonValue, String> {
        self.next_char(); // Skip opening bracket
        let mut array = Vec::new();
        
        loop {
            self.skip_whitespace();
            
            if let Some(']') = self.peek_char() {
                self.next_char();
                return Ok(JsonValue::Array(array));
            }
            
            if !array.is_empty() {
                match self.peek_char() {
                    Some(',') => {
                        self.next_char();
                        self.skip_whitespace();
                    }
                    _ => return Err("Expected comma".to_string()),
                }
            }

            array.push(self.parse_value()?);
        }
    }

    fn parse_object(&mut self) -> Result<JsonValue, String> {
        self.next_char(); // Skip opening brace
        let mut object = HashMap::new();
        
        loop {
            self.skip_whitespace();
            
            if let Some('}') = self.peek_char() {
                self.next_char();
                return Ok(JsonValue::Object(object));
            }
            
            if !object.is_empty() {
                match self.peek_char() {
                    Some(',') => {
                        self.next_char();
                        self.skip_whitespace();
                    }
                    _ => return Err("Expected comma".to_string()),
                }
            }

            match self.parse_value()? {
                JsonValue::String(key) => {
                    self.skip_whitespace();
                    if self.next_char() != Some(':') {
                        return Err("Expected colon".to_string());
                    }
                    let value = self.parse_value()?;
                    object.insert(key, value);
                }
                _ => return Err("Expected string as object key".to_string()),
            }
        }
    }

    fn parse_value(&mut self) -> Result<JsonValue, String> {
        self.skip_whitespace();
        match self.peek_char() {
            Some('n') => self.parse_null(),
            Some('t') | Some('f') => self.parse_boolean(),
            Some('"') => self.parse_string(),
            Some('[') => self.parse_array(),
            Some('{') => self.parse_object(),
            Some(c) if c.is_digit(10) || c == '-' => self.parse_number(),
            Some(c) => Err(format!("Unexpected character '{}'", c)),
            None => Err("Unexpected end of input".to_string()),
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek_char() {
            if !c.is_whitespace() {
                break;
            }
            self.next_char();
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.input.get(self.position).copied()
    }

    fn next_char(&mut self) -> Option<char> {
        let c = self.peek_char();
        self.position += 1;
        c
    }
}

fn main() {
    // Example XML
    // let args: Vec<String> = std::env::args().collect();
    // if args.len() != 2 {
    //     println!("Usage: {} <xml-string>", args[0]);
    //     std::process::exit(1);
    // }
    // let xml = &args[1];
    
    // // Parse XML to JSON
    // let mut xml_parser = XmlParser::new(xml);
    // match xml_parser.parse() {
    //     Ok(xml_node) => {
    //         let json = xml_node.to_json();
    //         println!("XML parsed to JSON successfully:");
    //         println!("{}", serde_json::to_string_pretty(&json).unwrap());
    //     }
    //     Err(e) => println!("Error parsing XML: {}", e),
    // }





     // Parse JSON
     let args: Vec<String> = std::env::args().collect();
     if args.len() != 2 {
         println!("Usage: {} <json-string>", args[0]);
         std::process::exit(1);
     }
     let json_str = &args[1];
     let mut json_parser = JsonParser::new(json_str);
     match json_parser.parse() {
         Ok(json_value) => {
             println!("\nJSON parsed successfully:");
             let xml = json_value.to_xml();
             println!("{}", xml);
         }
         Err(e) => println!("Error parsing JSON: {}", e),
     }
 }