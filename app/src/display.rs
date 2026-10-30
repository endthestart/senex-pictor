//! What the window's surface turned out to be, and how the canvas must write to it: linear
//! light to float and sRGB targets, encoded light to 8-bit targets egui treats as gamma space,
//! scaled so paper white matches egui's own white.

use crate::gpu::canvas::MODE_ENCODE;
use eframe::egui_wgpu::RenderState;

#[derive(Clone, Debug)]
pub struct Display {
    pub scale: f32,
    pub mode: u32,
    /// How far above paper white the screen can show light, when it reports it.
    pub headroom: Option<f32>,
    pub lines: Vec<String>,
}

impl Display {
    pub fn of(rs: &RenderState, gpu_name: &str, shared: bool) -> Display {
        let format = rs.target_format;
        let scale = rs.renderer.read().output_scale();
        let mode = if format == wgpu::TextureFormat::Rgba16Float || format.is_srgb() {
            0
        } else {
            MODE_ENCODE
        };
        let headroom = rs
            .display_hdr_info
            .read()
            .as_ref()
            .and_then(|i| i.headroom)
            .and_then(|h| h.potential.or(h.current));
        let lines = vec![
            format!(
                "GPU: {gpu_name}{}",
                if shared {
                    ""
                } else {
                    " (not shared with the window)"
                }
            ),
            format!("surface: {format:?}, {:?}", rs.target_color_space),
            match headroom {
                Some(h) if h > 1.0 => format!("HDR headroom: {h:.1}× paper white"),
                Some(_) => "HDR headroom: none (SDR or a reference mode)".into(),
                None => "HDR headroom: not reported".into(),
            },
        ];
        Display {
            scale,
            mode,
            headroom,
            lines,
        }
    }
}
