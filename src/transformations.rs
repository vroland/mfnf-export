use mediawiki_parser::ast::*;
use mediawiki_parser::transformations::*;
use mediawiki_parser::error::TransformationError;
use settings::Settings;
use util::*;
use std::path;
use std::fs::File;
use serde_yaml;


/// Convert template name paragraphs to lowercase text only.
pub fn normalize_template_names(mut root: Element, settings: &Settings) -> TResult {
    if let &mut Element::Template { ref mut name, ref mut content, ref position, .. } = &mut root {

        let new_text = match name.drain(..).next() {
            Some(Element::Paragraph { content, .. }) => {
                content
            },
            Some(e) => { vec![e] },
            None => { return Ok(Element::Error {
                        position: position.clone(),
                        message: "MFNF template name must not be empty!".to_string(),
                    })
            }
        };

        for child in content {
            if let &mut Element::TemplateArgument { ref mut name, .. } = child {
                let lowercase = name.trim().to_lowercase();
                name.clear();
                name.push_str(&lowercase);
            } else {
                return Ok(Element::Error {
                    position: position.clone(),
                    message: "Only TemplateArguments are allowed as children of templates!".to_string(),
                })
            }
        }

        if let Some(&Element::Text { ref position, ref text }) = new_text.first() {
            name.clear();
            name.push(
                Element::Text {
                    position: position.clone(),
                    text: if text.starts_with("#") {
                                String::from(text.trim())
                            } else {
                                // convert to lowercase and remove prefixes
                                let mut temp_text = &text.trim().to_lowercase()[..];
                                for prefix in &settings.template_prefixes[..] {
                                    temp_text = trim_prefix(temp_text, prefix);
                                }
                                String::from(temp_text)
                            },
                }
            );
        } else {
            return Ok(Element::Error {
                position: if let Some(e) = new_text.first() {
                    e.get_position().clone()
                } else {
                    position.clone()
                },
                message: "MFNF Template names must be plain strings \
                        With no markup!".to_string(),
            });
        }
    };
    recurse_inplace(&normalize_template_names, root, settings)
}

/// Translate template names and template attribute names.
pub fn translate_templates(mut root: Element, settings: &Settings) -> TResult {
    if let &mut Element::Template { ref mut name, ref mut content, .. } = &mut root {
        if let Some(&mut Element::Text { ref mut text, .. }) = name.first_mut() {
            if let Some(translation) = settings.translations.get(text) {
                text.clear();
                text.push_str(translation);
            }
        }
        for child in content {
            if let &mut Element::TemplateArgument { ref mut name, .. } = child {
                if let Some(translation) = settings.translations.get(name) {
                    name.clear();
                    name.push_str(translation);
                }
            }
        }
    }
    recurse_inplace(&translate_templates, root, settings)
}

/// Convert template attribute `title` to text only.
pub fn normalize_template_title(mut root: Element, settings: &Settings) -> TResult {
    if let &mut Element::TemplateArgument { ref name, ref mut value, ref position } = &mut root {
        if name == "title" {
            let mut last_value = value.pop();
            // title is empty
            if let None = last_value {
                return Err(TransformationError {
                    cause: "A template title must not be empty!".to_string(),
                    position: position.clone(),
                    transformation_name: "normalize_template_title".to_string(),
                    tree: Element::TemplateArgument {
                        name: name.clone(),
                        value: vec![],
                        position: position.clone(),
                    }
                })
            }
            if let Some(Element::Paragraph { ref mut content, .. }) = last_value {
                if let Some(&Element::Text { ref text, ref position  }) = content.last() {
                    value.clear();
                    value.push(Element::Text {
                        text: String::from(text.trim()),
                        position: position.clone(),
                    });
                }
            } else {
                value.push(last_value.unwrap());
            }
        }
    }
    recurse_inplace(&normalize_template_title, root, settings)
}


pub fn include_sections(
    mut root: Element,
    settings: &Settings) -> TResult {
    root = recurse_inplace_template(&include_sections, root, settings, &include_sections_vec)?;
    Ok(root)
}

pub fn include_sections_vec<'a>(
    trans: &TFuncInplace<&'a Settings>,
    root_content: &mut Vec<Element>,
    settings: &'a Settings) -> TListResult {

    // search for section inclusion in children
    let mut result = vec![];
    for mut child in root_content.drain(..) {

        if let &mut Element::Template {
            ref name,
            ref content,
            ref position
        } = &mut child {

            let prefix = &settings.deps_settings.section_inclusion_prefix;
            let template_name = extract_plain_text(&name);

            // section transclusion
            if template_name.to_lowercase().trim().starts_with(prefix) {
                let article = trim_prefix(template_name.trim(), prefix);
                if content.len() < 1 {
                    return Err(TransformationError {
                        cause: "A section inclusion must specify article \
                                name and section name!".to_string(),
                        position: position.clone(),
                        transformation_name: "include_sections".to_string(),
                        tree: Element::Template {
                            name: name.clone(),
                            position: position.clone(),
                            content: content.clone(),
                        }
                    });
                }

                let section_name = extract_plain_text(content);
                let mut section_file = settings.deps_settings.section_rev.clone();
                section_file.push('.');
                section_file.push_str(&settings.deps_settings.section_ext);

                let path = path::Path::new(&settings.deps_settings.section_path)
                    .join(&filename_to_make(&article))
                    .join(&filename_to_make(&section_name))
                    .join(&filename_to_make(&section_file));

                // error returned when the section file is faulty
                let file_error = TransformationError {
                    cause: format!("section file `{}` could not be read or parsed!",
                                &path.to_string_lossy()),
                    position: position.clone(),
                    transformation_name: "include_sections".to_string(),
                    tree: Element::Template {
                        name: name.clone(),
                        position: position.clone(),
                        content: content.clone(),
                    }
                };

                let section_str = File::open(&path);
                if section_str.is_err() {
                    return Err(file_error)
                }

                let mut section_tree: Vec<Element>
                    = match serde_yaml::from_reader(&section_str.unwrap()) {
                    Ok(root) => root,
                    Err(_) => return Err(file_error),
                };

                result.push(
                    Element::Comment {
                        position: position.clone(),
                        text: format!("included from: {}|{}", article, section_name),
                    }
                );

                // recursively include sections
                section_tree = include_sections_vec(
                    &include_sections,
                    &mut section_tree,
                    settings,
                )?;
                result.append(&mut section_tree);
                continue
            }
        }
        result.push(trans(child, settings)?);
    }
    Ok(result)
}
