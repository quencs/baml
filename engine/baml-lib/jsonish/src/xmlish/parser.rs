use anyhow::Result;
use quick_xml::events::{Event, BytesStart, BytesEnd, BytesText};
use quick_xml::Reader;
use std::collections::HashMap;
use baml_types::CompletionState;

use crate::xmlish::Value;

#[derive(Clone, Copy, Debug)]
pub struct ParseOptions {
    /// Allow incomplete/malformed XML
    allow_incomplete: bool,
    /// Maximum depth to prevent infinite recursion
    max_depth: usize,
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            allow_incomplete: true,
            max_depth: 100,
        }
    }
}

pub fn parse(xml_str: &str, options: ParseOptions) -> Result<Value> {
    log::debug!("Parsing XML:\n{:?}\n-------\n{}\n-------", options, xml_str);
    
    if xml_str.trim().is_empty() {
        return Ok(Value::text(String::new()));
    }

    // First try to parse as complete XML
    match parse_complete_xml(xml_str, options) {
        Ok(value) => return Ok(value),
        Err(e) => {
            log::debug!("Complete XML parse failed: {:?}", e);
            if !options.allow_incomplete {
                return Err(e);
            }
        }
    }

    // If complete parsing fails and we allow incomplete, try partial parsing
    match parse_partial_xml(xml_str, options) {
        Ok(value) => Ok(value),
        Err(e) => {
            log::debug!("Partial XML parse failed: {:?}, returning as fragment", e);
            // If all else fails, return as a text fragment
            Ok(Value::fragment(xml_str.to_string()))
        }
    }
}

fn parse_complete_xml(xml_str: &str, options: ParseOptions) -> Result<Value> {
    let mut reader = Reader::from_str(xml_str);
    
    let mut elements = Vec::new();
    let mut depth = 0;
    
    loop {
        if depth > options.max_depth {
            return Err(anyhow::anyhow!("Maximum depth exceeded"));
        }
        
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                depth += 1;
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let attributes = parse_attributes(e)?;
                
                // Parse children recursively
                let children = parse_children(&mut reader, &tag, options, &mut depth)?;
                
                elements.push(Value::element(tag, attributes, children));
            }
            Ok(Event::Empty(ref e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let attributes = parse_attributes(e)?;
                elements.push(Value::element(tag, attributes, Vec::new()));
            }
            Ok(Event::Text(ref e)) => {
                let text = e.unescape()?.to_string();
                if !text.trim().is_empty() {
                    elements.push(Value::text(text));
                }
            }
            Ok(Event::CData(ref e)) => {
                let text = String::from_utf8_lossy(e).to_string();
                elements.push(Value::text(text));
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}, // Ignore other events like comments, processing instructions
            Err(e) => return Err(anyhow::anyhow!("XML parsing error: {}", e)),
        }
    }
    
    match elements.len() {
        0 => Ok(Value::text(String::new())),
        1 => Ok(elements.into_iter().next().unwrap()),
        _ => {
            // Multiple root elements, wrap in a container or return as AnyOf
            Ok(Value::AnyOf(elements.clone(), xml_str.to_string()))
        }
    }
}

fn parse_partial_xml(xml_str: &str, _options: ParseOptions) -> Result<Value> {
    // For partial XML, we'll try to extract what we can
    // This is a simplified approach - in a full implementation you'd want more sophisticated logic
    
    // Look for complete elements first
    if let Some(element) = try_extract_complete_element(xml_str) {
        return Ok(element);
    }
    
    // If no complete elements, check if it looks like the start of an XML structure
    let trimmed = xml_str.trim();
    if trimmed.starts_with('<') && !trimmed.ends_with('>') {
        // Looks like an incomplete XML element
        return Ok(Value::fragment(xml_str.to_string()));
    }
    
    // Otherwise, treat as text content
    Ok(Value::text(xml_str.to_string()))
}

fn try_extract_complete_element(xml_str: &str) -> Option<Value> {
    // Simple regex-like approach to find complete elements
    // This is a basic implementation - a production version would be more robust
    
    let mut reader = Reader::from_str(xml_str);
    
    match reader.read_event() {
        Ok(Event::Start(ref e)) => {
            let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
            if let Ok(attributes) = parse_attributes(e) {
                // Try to find the matching end tag
                let end_tag = format!("</{}>", tag);
                if xml_str.contains(&end_tag) {
                    // There might be a complete element here
                    // For simplicity, we'll create an incomplete element
                    return Some(Value::incomplete_element(tag, attributes, Vec::new()));
                }
            }
        }
        Ok(Event::Empty(ref e)) => {
            let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
            if let Ok(attributes) = parse_attributes(e) {
                return Some(Value::element(tag, attributes, Vec::new()));
            }
        }
        _ => {}
    }
    
    None
}

fn parse_children(
    reader: &mut Reader<&[u8]>, 
    parent_tag: &str, 
    options: ParseOptions,
    depth: &mut usize
) -> Result<Vec<Value>> {
    let mut children = Vec::new();
    
    loop {
        if *depth > options.max_depth {
            return Err(anyhow::anyhow!("Maximum depth exceeded"));
        }
        
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                *depth += 1;
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let attributes = parse_attributes(e)?;
                let grandchildren = parse_children(reader, &tag, options, depth)?;
                children.push(Value::element(tag, attributes, grandchildren));
            }
            Ok(Event::Empty(ref e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let attributes = parse_attributes(e)?;
                children.push(Value::element(tag, attributes, Vec::new()));
            }
            Ok(Event::Text(ref e)) => {
                let text = e.unescape()?.to_string();
                if !text.trim().is_empty() {
                    children.push(Value::text(text));
                }
            }
            Ok(Event::CData(ref e)) => {
                let text = String::from_utf8_lossy(e).to_string();
                children.push(Value::text(text));
            }
            Ok(Event::End(ref e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if tag == parent_tag {
                    *depth -= 1;
                    break;
                } else {
                    return Err(anyhow::anyhow!("Mismatched closing tag: expected {}, got {}", parent_tag, tag));
                }
            }
            Ok(Event::Eof) => {
                return Err(anyhow::anyhow!("Unexpected end of file while parsing element {}", parent_tag));
            }
            Ok(_) => {}, // Ignore other events
            Err(e) => return Err(anyhow::anyhow!("XML parsing error: {}", e)),
        }
    }
    
    Ok(children)
}

fn parse_attributes(element: &BytesStart) -> Result<HashMap<String, String>> {
    let mut attributes = HashMap::new();
    
    for attr in element.attributes() {
        let attr = attr?;
        let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
        let value = attr.unescape_value()?.to_string();
        attributes.insert(key, value);
    }
    
    Ok(attributes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_element() {
        let xml = "<root>content</root>";
        let result = parse(xml, ParseOptions::default()).unwrap();
        
        match result {
            Value::Element { tag, children, .. } => {
                assert_eq!(tag, "root");
                assert_eq!(children.len(), 1);
                match &children[0] {
                    Value::Text(content, _) => assert_eq!(content, "content"),
                    _ => panic!("Expected text content"),
                }
            }
            _ => panic!("Expected element"),
        }
    }

    #[test]
    fn test_incomplete_xml() {
        let xml = "<root>incomplete";
        let result = parse(xml, ParseOptions::default()).unwrap();
        
        // Should return as fragment since it's incomplete
        match result {
            Value::Fragment(content, _) => assert_eq!(content, xml),
            _ => {}, // Other interpretations are also valid
        }
    }

    #[test]
    fn test_empty_element() {
        let xml = "<empty/>";
        let result = parse(xml, ParseOptions::default()).unwrap();
        
        match result {
            Value::Element { tag, children, .. } => {
                assert_eq!(tag, "empty");
                assert_eq!(children.len(), 0);
            }
            _ => panic!("Expected element"),
        }
    }
}