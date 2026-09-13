//! Mechanism-only XML parser. Product namespaces and element semantics stay in products.

use quick_xml::{Reader, events::Event};
use std::time::{Duration, Instant};

pub use roxmltree::Document;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XmlBudget {
    pub max_bytes: usize,
    pub max_depth: usize,
    pub max_nodes: usize,
    pub max_text_bytes: usize,
    pub max_text_node_bytes: usize,
    pub max_parse_time: Duration,
}

pub fn parse_bounded<'a>(
    input: &'a str,
    budget: XmlBudget,
) -> Result<roxmltree::Document<'a>, Error> {
    if input.len() > budget.max_bytes {
        return Err(Error::Budget("bytes"));
    }
    let started = Instant::now();
    preflight(input, budget, started)?;
    let document = Document::parse(input)?;
    if started.elapsed() > budget.max_parse_time {
        return Err(Error::Budget("time"));
    }
    Ok(document)
}

fn preflight(input: &str, budget: XmlBudget, started: Instant) -> Result<(), Error> {
    let mut reader = Reader::from_str(input);
    reader.config_mut().check_end_names = true;
    let mut depth = 1_usize;
    let mut nodes = 1_usize;
    let mut text = 0_usize;
    loop {
        if started.elapsed() > budget.max_parse_time {
            return Err(Error::Budget("time"));
        }
        match reader.read_event().map_err(Error::Streaming)? {
            Event::Start(_) => {
                depth = depth.checked_add(1).ok_or(Error::Budget("depth"))?;
                add_node(&mut nodes, depth, budget)?;
            }
            Event::Empty(_) => add_node(
                &mut nodes,
                depth.checked_add(1).ok_or(Error::Budget("depth"))?,
                budget,
            )?,
            Event::End(_) => {
                depth = depth.checked_sub(1).ok_or(Error::Budget("depth"))?;
            }
            Event::Text(value) => {
                add_node(
                    &mut nodes,
                    depth.checked_add(1).ok_or(Error::Budget("depth"))?,
                    budget,
                )?;
                if value.len() > budget.max_text_node_bytes {
                    return Err(Error::Budget("text node"));
                }
                text = text.checked_add(value.len()).ok_or(Error::Budget("text"))?;
                if text > budget.max_text_bytes {
                    return Err(Error::Budget("text"));
                }
            }
            Event::CData(value) => {
                add_node(
                    &mut nodes,
                    depth.checked_add(1).ok_or(Error::Budget("depth"))?,
                    budget,
                )?;
                if value.len() > budget.max_text_node_bytes {
                    return Err(Error::Budget("text node"));
                }
                text = text.checked_add(value.len()).ok_or(Error::Budget("text"))?;
                if text > budget.max_text_bytes {
                    return Err(Error::Budget("text"));
                }
            }
            Event::Comment(_) | Event::PI(_) => add_node(
                &mut nodes,
                depth.checked_add(1).ok_or(Error::Budget("depth"))?,
                budget,
            )?,
            Event::DocType(_) => return Err(Error::ForbiddenDeclaration),
            Event::Eof => break,
            Event::Decl(_) | Event::GeneralRef(_) => {}
        }
    }
    Ok(())
}

fn add_node(nodes: &mut usize, depth: usize, budget: XmlBudget) -> Result<(), Error> {
    *nodes = nodes.checked_add(1).ok_or(Error::Budget("nodes"))?;
    if *nodes > budget.max_nodes {
        return Err(Error::Budget("nodes"));
    }
    if depth > budget.max_depth {
        return Err(Error::Budget("depth"));
    }
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("XML contains a forbidden DTD or entity declaration")]
    ForbiddenDeclaration,
    #[error("XML {0} budget exceeded")]
    Budget(&'static str),
    #[error(transparent)]
    Parse(#[from] roxmltree::Error),
    #[error("XML is malformed")]
    Streaming(quick_xml::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    fn budget() -> XmlBudget {
        XmlBudget {
            max_bytes: 100,
            max_depth: 4,
            max_nodes: 8,
            max_text_bytes: 10,
            max_text_node_bytes: 10,
            max_parse_time: Duration::from_secs(1),
        }
    }
    #[test]
    fn rejects_entities_and_depth() {
        assert!(matches!(
            parse_bounded("<!DOCTYPE x [<!ENTITY e 'x'>]><x>&e;</x>", budget()),
            Err(Error::ForbiddenDeclaration)
        ));
        assert!(matches!(
            parse_bounded("<a><b><c><d><e/></d></c></b></a>", budget()),
            Err(Error::Budget("depth"))
        ));
    }
    #[test]
    fn accepts_bounded_document() {
        assert_eq!(
            parse_bounded("<a><b>ok</b></a>", budget())
                .unwrap()
                .root_element()
                .tag_name()
                .name(),
            "a"
        );
    }

    #[test]
    fn rejects_node_and_text_budgets_during_streaming_preflight() {
        let mut small = budget();
        small.max_nodes = 2;
        assert!(matches!(
            parse_bounded("<a><b/></a>", small),
            Err(Error::Budget("nodes"))
        ));
        small = budget();
        small.max_text_bytes = 1;
        assert!(matches!(
            parse_bounded("<a>ok</a>", small),
            Err(Error::Budget("text"))
        ));
    }
}
