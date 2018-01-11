extern crate mediawiki_parser;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_yaml;

use mediawiki_parser::ast::Element;
use mediawiki_parser::transformations::TResult;

/// Structures for configuration of transformations.
pub mod settings;

pub mod latex;
pub mod deps;
pub mod sections;
mod util;
mod transformations;

/// Applies all MFNF-Specific transformations.
pub fn apply_transformations(mut root: Element, settings: &settings::Settings) -> TResult {
    root = transformations::include_sections(root, settings)?;
    root = transformations::normalize_template_names(root, settings)?;
    root = transformations::translate_templates(root, settings)?;
    root = transformations::normalize_template_title(root, settings)?;

    Ok(root)
}
