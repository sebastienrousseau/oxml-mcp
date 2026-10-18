// SPDX-License-Identifier: MIT OR Apache-2.0
//! Bounded output conversion, separate from `XPath` evaluation.

use oxml::{Document, NodeId, NodeKind};

fn append(
    out: &mut String,
    text: &str,
    remaining: &mut usize,
) -> Result<(), String> {
    *remaining = remaining
        .checked_sub(text.len())
        .ok_or("query output exceeds the string byte limit")?;
    out.push_str(text);
    Ok(())
}

pub(crate) fn text(
    doc: &Document,
    id: NodeId,
    remaining: &mut usize,
) -> Result<String, String> {
    let mut out = String::new();
    if let Some(text) = direct_text(doc, id) {
        append(&mut out, text, remaining)?;
    } else if matches!(
        doc.kind(id),
        Some(NodeKind::Root | NodeKind::Element { .. })
    ) {
        for child in doc.descendants_of(id) {
            if let Some(NodeKind::Text(text)) = doc.kind(child) {
                append(&mut out, text, remaining)?;
            }
        }
    }
    Ok(out)
}

fn direct_text(doc: &Document, id: NodeId) -> Option<&str> {
    match doc.kind(id)? {
        NodeKind::Text(text) => Some(text),
        NodeKind::Attr(attribute) => Some(attribute.value),
        NodeKind::Namespace { uri, .. } => Some(uri),
        _ => None,
    }
}
