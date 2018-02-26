//! Render image galleries

use preamble::*;
use super::LatexRenderer;

impl<'e, 's: 'e, 't: 'e> LatexRenderer<'e, 't> {

    pub fn gallery(
        &mut self,
        root: &'e Element,
        settings: &'s Settings,
        out: &mut io::Write) -> io::Result<bool> {

        if let &Element::Gallery {
            ref position,
            ref content,
            ..
        } = root {

            let columns = "X".repeat(content.len());
            let doctitle = &settings.document_title;

            let mut rendered_images = vec![];

            for image in content {
                if let &Element::InternalReference {
                    ref position,
                    ref target,
                    ref options,
                    ref caption
                } = image {
                    let path = self.build_image_path(target, settings);
                    let caption = caption.render(self, settings)?;

                    // collect image options
                    let mut image_options = vec![];
                    for option in options {
                        image_options.push(extract_plain_text(option).trim().to_string());
                    }

                    let mut inner = format!(
                        GALLERY_CONTENT!(),
                        &image_options,
                        self.latex.image_width,
                        self.latex.image_height,
                        &path,
                        &caption,
                    );

                    let indent = self.latex.indentation_depth;
                    let line_width = self.latex.max_line_width;
                    inner = indent_and_trim(&inner, indent, line_width);
                    rendered_images.push(inner);
                }
            }

            self.write_def_location(position, &doctitle, out)?;
            writeln!(
                out,
                GALLERY!(),
                columns,
                rendered_images.join("\n")
            )?;
        }
        Ok(false)
    }
}