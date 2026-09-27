use std::{env, error::Error, fs, path::PathBuf};

const LOGO: &str = include_str!("../assets/icons/logo.svg");
const ICON_SIZES: [u32; 7] = [16, 24, 32, 48, 64, 128, 256];

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=../assets/icons/logo.svg");

    if env::var("CARGO_CFG_TARGET_OS")?.as_str() == "windows" {
        let icon_path = PathBuf::from(env::var_os("OUT_DIR").ok_or("OUT_DIR is missing")?)
            .join("localcraft.ico");
        fs::write(&icon_path, make_icon()?)?;
        winres::WindowsResource::new()
            .set_icon(icon_path.to_str().ok_or("Icon path is not valid UTF-8")?)
            .compile()?;
    }

    Ok(())
}

fn make_icon() -> Result<Vec<u8>, Box<dyn Error>> {
    let svg_start = LOGO.find('>').ok_or("Logo SVG has no root element")? + 1;
    let svg_end = LOGO.rfind("</svg>").ok_or("Logo SVG is not closed")?;
    let logo_content = &LOGO[svg_start..svg_end];
    let svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"200\" height=\"200\" viewBox=\"0 0 200 200\"><rect width=\"200\" height=\"200\" rx=\"44\" fill=\"#0a0a0a\"/><svg x=\"20\" y=\"20\" width=\"160\" height=\"160\" viewBox=\"40 75 135 165\" fill=\"none\" stroke=\"#f5f5f5\" stroke-width=\"3\" stroke-linecap=\"round\" stroke-linejoin=\"round\">{logo_content}</svg></svg>"
    );
    let tree = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default())?;
    let mut pngs = Vec::with_capacity(ICON_SIZES.len());

    for size in ICON_SIZES {
        let mut pixmap =
            resvg::tiny_skia::Pixmap::new(size, size).ok_or("Could not allocate icon image")?;
        let scale = size as f32 / 200.0;
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::from_scale(scale, scale),
            &mut pixmap.as_mut(),
        );
        pngs.push(pixmap.encode_png()?);
    }

    let count = u16::try_from(pngs.len())?;
    let mut icon = Vec::new();
    icon.extend_from_slice(&[0, 0, 1, 0]);
    icon.extend_from_slice(&count.to_le_bytes());

    let mut image_offset = 6 + 16 * pngs.len();
    for (size, png) in ICON_SIZES.iter().zip(&pngs) {
        icon.push(if *size == 256 { 0 } else { *size as u8 });
        icon.push(if *size == 256 { 0 } else { *size as u8 });
        icon.extend_from_slice(&[0, 0]);
        icon.extend_from_slice(&1_u16.to_le_bytes());
        icon.extend_from_slice(&32_u16.to_le_bytes());
        icon.extend_from_slice(&u32::try_from(png.len())?.to_le_bytes());
        icon.extend_from_slice(&u32::try_from(image_offset)?.to_le_bytes());
        image_offset += png.len();
    }
    for png in pngs {
        icon.extend_from_slice(&png);
    }

    Ok(icon)
}
