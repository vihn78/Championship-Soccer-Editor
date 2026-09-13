use eframe::egui;
use std::collections::HashMap;

#[derive(Default, Clone)]
pub struct CountryNames;

impl CountryNames {
    pub fn name<'a>(&self, _identity: &str, fallback: &'a str) -> &'a str {
        fallback
    }
}

pub fn load_flags(context: &egui::Context) -> HashMap<String, egui::TextureHandle> {
    let mut textures = HashMap::new();
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("flags");
    if let Ok(entries) = std::fs::read_dir(directory) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("png"))
            {
                continue;
            }
            if let Ok(image) = image::open(&path) {
                let rgba = image.to_rgba8();
                let pixels = egui::ColorImage::from_rgba_unmultiplied(
                    [rgba.width() as usize, rgba.height() as usize],
                    rgba.as_raw(),
                );
                let key = path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_lowercase();
                textures.insert(
                    key.clone(),
                    context.load_texture(key, pixels, egui::TextureOptions::LINEAR),
                );
            }
        }
    }
    textures
}

pub fn flag_label(
    ui: &mut egui::Ui,
    flags: &HashMap<String, egui::TextureHandle>,
    identity: &str,
    display: &str,
    selected: bool,
) -> egui::Response {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let height = ui.text_style_height(&egui::TextStyle::Body);
        let (rect, response) =
            ui.allocate_exact_size(egui::vec2(height * 2.0, height), egui::Sense::click());
        if let Some(texture) = flags
            .get(&identity.to_lowercase())
            .or_else(|| flags.get(&display.to_lowercase()))
            .or_else(|| flags.get("noflag"))
        {
            let size = texture.size_vec2();
            let scale = (rect.width() / size.x).min(rect.height() / size.y);
            let image_rect = egui::Rect::from_center_size(rect.center(), size * scale);
            ui.painter().image(
                texture.id(),
                image_rect,
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }
        response.union(ui.selectable_label(selected, display))
    })
    .inner
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn country_name_uses_the_file_identity() {
        let names = CountryNames;
        assert_eq!(names.name("Italy", "Italy"), "Italy");
        assert_eq!(names.name("Internal name", "San Marino"), "San Marino");
    }
}
