//! herdr's Kitty graphics, as a client shell gets them ([`GraphicsScene`]). herdr's own terminal keeps the images and
//! places, scrolls and clips them; Shepherd keeps the bytes herdr sends once, and per pane, the part of each image
//! shown and the cells it covers. Decoding happens here, on the shell's reader thread.
use super::wire::{AssetKey, GraphicsFormat, GraphicsPlacement, GraphicsScene, GraphicsSource, GraphicsTarget};
use gpui::RenderImage;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// One image on a pane: the part shown (BGRA, as gpui draws), at cell `x`,`y` from the pane's top-left, over
/// `cols`×`rows` cells less the device-pixel offset into its first cell.
#[derive(Clone)]
pub struct PaneImage {
    pub image: Arc<RenderImage>,
    pub x: u16,
    pub y: u16,
    pub cols: u32,
    pub rows: u32,
    pub x_offset: u32,
    pub y_offset: u32,
    /// Kitty's z-index: below zero under the text, else over it.
    pub z: i32,
}

impl std::fmt::Debug for PaneImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PaneImage({}x{} cells at {},{} z {})", self.cols, self.rows, self.x, self.y, self.z)
    }
}

/// The images on each pane, by herdr's pane id.
pub type PaneImages = HashMap<String, Vec<PaneImage>>;

#[derive(Default)]
pub struct Graphics {
    assets: HashMap<AssetKey, Arc<image::RgbaImage>>,
    /// The parts shown, reused while herdr shows the same part: gpui keeps one texture per image.
    cuts: HashMap<(AssetKey, [u32; 4]), Arc<RenderImage>>,
    placements: Vec<GraphicsPlacement>,
}

impl Graphics {
    /// herdr's next scene; the images per pane when they changed. herdr sends the scene with every whole surface.
    pub fn apply(&mut self, scene: GraphicsScene) -> Option<PaneImages> {
        let fresh = !scene.assets.is_empty();
        for asset in scene.assets {
            if asset.data.len() as u64 != asset.key.data_len {
                continue;
            }
            if let Some(image) = decode(&asset.key, asset.data) {
                self.assets.insert(asset.key, Arc::new(image));
            }
        }
        if !fresh && scene.placements == self.placements {
            return None;
        }
        let kept: HashSet<&AssetKey> = scene.placements.iter().map(|p| &p.asset).chain(&scene.retained_assets).collect();
        self.assets.retain(|key, _| kept.contains(key));
        let mut cuts = HashMap::new();
        let mut panes = PaneImages::new();
        // ponytail: popup images are left out; the popup is replayed as cells
        for p in &scene.placements {
            let GraphicsSource::Terminal { target: GraphicsTarget::Pane { pane_id }, .. } = &p.asset.source else { continue };
            let Some(full) = self.assets.get(&p.asset) else { continue };
            let key = (p.asset.clone(), [p.source_x, p.source_y, p.source_width, p.source_height]);
            let Some(image) = self.cuts.remove(&key).or_else(|| cuts.get(&key).cloned()).or_else(|| cut(full, key.1).map(Arc::new)) else {
                continue;
            };
            cuts.insert(key, image.clone());
            panes.entry(pane_id.clone()).or_default().push(PaneImage {
                image,
                x: p.x,
                y: p.y,
                cols: p.cols,
                rows: p.rows,
                x_offset: p.x_offset,
                y_offset: p.y_offset,
                z: p.z,
            });
        }
        for images in panes.values_mut() {
            images.sort_by_key(|i| i.z);
        }
        self.cuts = cuts;
        self.placements = scene.placements;
        Some(panes)
    }
}

/// herdr's image bytes as RGBA: raw RGB or RGBA at the key's size, or a PNG (herdr has inflated zlib already).
fn decode(key: &AssetKey, data: Vec<u8>) -> Option<image::RgbaImage> {
    let (w, h) = (key.image_width, key.image_height);
    match key.format {
        GraphicsFormat::Rgba => image::RgbaImage::from_raw(w, h, data),
        GraphicsFormat::Rgb => image::RgbImage::from_raw(w, h, data).map(|i| image::DynamicImage::ImageRgb8(i).into_rgba8()),
        GraphicsFormat::Png => image::load_from_memory_with_format(&data, image::ImageFormat::Png).ok().map(|i| i.into_rgba8()),
    }
}

/// `[x, y, width, height]` of `image` (clamped to it) as gpui's BGRA image.
fn cut(image: &image::RgbaImage, [x, y, w, h]: [u32; 4]) -> Option<RenderImage> {
    let (x, y) = (x.min(image.width()), y.min(image.height()));
    let (w, h) = (w.min(image.width() - x), h.min(image.height() - y));
    if w == 0 || h == 0 {
        return None;
    }
    let mut part = image::imageops::crop_imm(image, x, y, w, h).to_image();
    for pixel in part.pixels_mut() {
        pixel.0.swap(0, 2);
    }
    Some(RenderImage::new(vec![image::Frame::new(part)]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::herdr::wire::GraphicsAsset;

    fn key(pane: &str, format: GraphicsFormat, w: u32, h: u32, data: &[u8]) -> AssetKey {
        let target = GraphicsTarget::Pane { pane_id: pane.into() };
        AssetKey {
            source: GraphicsSource::Terminal { target, image_id: 7 },
            image_width: w,
            image_height: h,
            format,
            data_len: data.len() as u64,
            data_fingerprint: 1,
        }
    }

    fn placed(asset: &AssetKey, x: u16, y: u16, source: [u32; 4]) -> GraphicsPlacement {
        let [source_x, source_y, source_width, source_height] = source;
        GraphicsPlacement {
            asset: asset.clone(),
            logical_placement_id: 1,
            x,
            y,
            cols: 2,
            rows: 1,
            source_x,
            source_y,
            source_width,
            source_height,
            x_offset: 0,
            y_offset: 0,
            z: 0,
            scrollback_offset: 0,
        }
    }

    /// 3×1 PNG: red, green, blue.
    fn png() -> Vec<u8> {
        let rgb = image::RgbImage::from_raw(3, 1, vec![255, 0, 0, 0, 255, 0, 0, 0, 255]).unwrap();
        let mut out = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(rgb).write_to(&mut out, image::ImageFormat::Png).unwrap();
        out.into_inner()
    }

    #[test]
    fn a_png_is_cut_to_the_part_shown_and_kept_once_sent() {
        let data = png();
        let k = key("w:p", GraphicsFormat::Png, 3, 1, &data);
        let mut g = Graphics::default();
        let scene = |assets: Vec<GraphicsAsset>, placements| GraphicsScene { assets, placements, retained_assets: vec![] };

        // the right two pixels at cell 4,2 of pane w:p
        let shown = g.apply(scene(vec![GraphicsAsset { key: k.clone(), data }], vec![placed(&k, 4, 2, [1, 0, 2, 1])])).unwrap();
        let image = &shown["w:p"][0];
        assert_eq!((image.x, image.y, image.cols, image.rows), (4, 2, 2, 1));
        assert_eq!(image.image.size(0), gpui::size(gpui::DevicePixels(2), gpui::DevicePixels(1)));
        assert_eq!(image.image.as_bytes(0).unwrap(), [0, 255, 0, 255, 255, 0, 0, 255], "green, blue as BGRA");

        // the same scene again (herdr resends it with each whole surface): nothing changed
        assert!(g.apply(scene(vec![], vec![placed(&k, 4, 2, [1, 0, 2, 1])])).is_none());

        // scrolled a row up, without the bytes: the same texture moves
        let moved = g.apply(scene(vec![], vec![placed(&k, 4, 1, [1, 0, 2, 1])])).unwrap();
        assert!(Arc::ptr_eq(&moved["w:p"][0].image, &image.image));
        assert_eq!(moved["w:p"][0].y, 1);

        // deleted: the bytes go too, so a later placement of the key shows nothing until herdr sends them again
        assert!(g.apply(scene(vec![], vec![])).unwrap().is_empty());
        assert!(g.apply(scene(vec![], vec![placed(&k, 0, 0, [0, 0, 3, 1])])).unwrap().is_empty());
    }

    #[test]
    fn raw_rgb_and_rgba_and_retained_images() {
        let rgb = vec![10, 20, 30, 40, 50, 60];
        let rgba = vec![1, 2, 3, 4];
        let (a, b) = (key("w:a", GraphicsFormat::Rgb, 2, 1, &rgb), key("w:b", GraphicsFormat::Rgba, 1, 1, &rgba));
        let mut g = Graphics::default();
        let assets = vec![GraphicsAsset { key: a.clone(), data: rgb }, GraphicsAsset { key: b.clone(), data: rgba }];
        let mut low = placed(&b, 0, 0, [0, 0, 1, 1]);
        low.z = -1;
        let both = vec![placed(&a, 0, 0, [0, 0, 2, 1]), placed(&b, 1, 0, [0, 0, 1, 1]), low];
        let shown = g.apply(GraphicsScene { assets, placements: both, retained_assets: vec![] }).unwrap();
        assert_eq!(shown["w:a"][0].image.as_bytes(0).unwrap(), [30, 20, 10, 255, 60, 50, 40, 255]);
        assert_eq!(shown["w:b"].iter().map(|i| i.z).collect::<Vec<_>>(), [-1, 0], "under the text first");
        assert_eq!(shown["w:b"][1].image.as_bytes(0).unwrap(), [3, 2, 1, 4]);

        // w:a's tab is hidden: herdr retains its image, so it shows again without the bytes
        let hidden = g.apply(GraphicsScene { assets: vec![], placements: vec![], retained_assets: vec![a.clone()] }).unwrap();
        assert!(hidden.is_empty());
        let back = g.apply(GraphicsScene { assets: vec![], placements: vec![placed(&a, 0, 0, [0, 0, 2, 1])], retained_assets: vec![] });
        assert_eq!(back.unwrap()["w:a"].len(), 1);
    }

    #[test]
    fn short_or_broken_bytes_are_skipped() {
        let mut short = key("w:p", GraphicsFormat::Rgba, 2, 2, &[0; 16]);
        short.data_len = 4;
        let broken = key("w:p", GraphicsFormat::Png, 1, 1, b"nope");
        let mut g = Graphics::default();
        let assets = vec![GraphicsAsset { key: short.clone(), data: vec![0; 4] }, GraphicsAsset { key: broken.clone(), data: b"nope".to_vec() }];
        let placements = vec![placed(&short, 0, 0, [0, 0, 2, 2]), placed(&broken, 0, 0, [0, 0, 1, 1])];
        assert!(g.apply(GraphicsScene { assets, placements, retained_assets: vec![] }).unwrap().is_empty());
    }
}
