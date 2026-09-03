//! Mechanism-only XML parser. Product namespaces and element semantics stay in products.

use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XmlBudget {
    pub max_bytes: usize,
    pub max_depth: usize,
    pub max_nodes: usize,
    pub max_text_bytes: usize,
    pub max_parse_time: Duration,
}

pub fn parse_bounded<'a>(
    input: &'a str,
    budget: XmlBudget,
) -> Result<roxmltree::Document<'a>, Error> {
    if input.len() > budget.max_bytes {
        return Err(Error::Budget("bytes"));
    }
    let upper = input.to_ascii_uppercase();
    if upper.contains("<!DOCTYPE") || upper.contains("<!ENTITY") {
        return Err(Error::ForbiddenDeclaration);
    }
    let started = Instant::now();
    let document = roxmltree::Document::parse(input)?;
    let mut nodes = 0_usize;
    let mut text = 0_usize;
    for node in document.descendants() {
        nodes = nodes.checked_add(1).ok_or(Error::Budget("nodes"))?;
        if nodes > budget.max_nodes {
            return Err(Error::Budget("nodes"));
        }
        if node.ancestors().count() > budget.max_depth {
            return Err(Error::Budget("depth"));
        }
        if let Some(value) = node.text().filter(|_| node.is_text()) {
            text = text.checked_add(value.len()).ok_or(Error::Budget("text"))?;
            if text > budget.max_text_bytes {
                return Err(Error::Budget("text"));
            }
        }
        if started.elapsed() > budget.max_parse_time {
            return Err(Error::Budget("time"));
        }
    }
    Ok(document)
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("XML contains a forbidden DTD or entity declaration")]
    ForbiddenDeclaration,
    #[error("XML {0} budget exceeded")]
    Budget(&'static str),
    #[error(transparent)]
    Parse(#[from] roxmltree::Error),
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
}
