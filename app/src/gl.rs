//! WebGL2 renderer: uploads the photo once, then re-runs the develop shader
//! for every frame, thumbnail, histogram sample and export tile.

use std::collections::HashMap;

use mirror_core::curve::LUT_SIZE;
use mirror_core::model::{Adjustments, Geometry, Rect};
use mirror_core::pipeline::{gaussian_kernel, Prepared, BLUR_EDGE};
use wasm_bindgen::JsCast;
use web_sys::{
    HtmlCanvasElement, ImageBitmap, WebGl2RenderingContext as GL, WebGlContextAttributes, WebGlFramebuffer,
    WebGlPowerPreference, WebGlProgram, WebGlShader, WebGlTexture, WebGlUniformLocation,
};

const QUAD_VS: &str = include_str!("shaders/quad.vert");
const DEVELOP_FS: &str = include_str!("shaders/develop.frag");
const LUM_FS: &str = include_str!("shaders/lum.frag");
const BLUR_FS: &str = include_str!("shaders/blur.frag");

struct Program {
    program: WebGlProgram,
    uniforms: HashMap<&'static str, WebGlUniformLocation>,
}

impl Program {
    fn new(gl: &GL, fs: &str, names: &[&'static str]) -> Result<Self, String> {
        let vs = compile(gl, GL::VERTEX_SHADER, QUAD_VS)?;
        let fs = compile(gl, GL::FRAGMENT_SHADER, fs)?;
        let program = gl.create_program().ok_or("Could not create program")?;
        gl.attach_shader(&program, &vs);
        gl.attach_shader(&program, &fs);
        gl.link_program(&program);
        if !gl.get_program_parameter(&program, GL::LINK_STATUS).as_bool().unwrap_or(false) {
            return Err(gl.get_program_info_log(&program).unwrap_or_default());
        }
        let mut uniforms = HashMap::new();
        for n in names {
            if let Some(loc) = gl.get_uniform_location(&program, n) {
                uniforms.insert(*n, loc);
            }
        }
        Ok(Self { program, uniforms })
    }

    fn u(&self, name: &str) -> Option<&WebGlUniformLocation> {
        self.uniforms.get(name)
    }
}

fn compile(gl: &GL, kind: u32, src: &str) -> Result<WebGlShader, String> {
    let s = gl.create_shader(kind).ok_or("Could not create shader")?;
    gl.shader_source(&s, src);
    gl.compile_shader(&s);
    if gl.get_shader_parameter(&s, GL::COMPILE_STATUS).as_bool().unwrap_or(false) {
        Ok(s)
    } else {
        Err(gl.get_shader_info_log(&s).unwrap_or_default())
    }
}

const DEVELOP_UNIFORMS: &[&str] = &[
    "u_src", "u_blur", "u_lut", "u_target", "u_offset", "u_imgRect", "u_srcSize", "u_srcFull", "u_turns", "u_flip",
    "u_straighten", "u_crop", "u_split", "u_bypass", "u_gain", "u_tone", "u_contrast", "u_local", "u_color", "u_hsl",
    "u_hslOn", "u_grade", "u_gradeOn", "u_vig", "u_grain", "u_curveOn", "u_lutSize",
];

/// The uploaded photo.
pub struct Source {
    tex: WebGlTexture,
    pub width: u32,
    pub height: u32,
    /// Size before any downscale to fit the GPU.
    pub full_width: u32,
    pub full_height: u32,
}

/// Everything the develop shader needs for one draw besides the adjustments.
#[derive(Clone, Copy, Debug)]
pub struct Placement {
    /// Image rectangle inside the logical canvas, in pixels.
    pub rect: [f32; 4],
    /// Show the original left of this output x (`< 0` = off).
    pub split: f32,
    pub bypass: bool,
    /// Ignore the crop (crop tool shows the whole frame).
    pub uncropped: bool,
}

pub struct Renderer {
    gl: GL,
    canvas: HtmlCanvasElement,
    develop: Program,
    lum: Program,
    blur: Program,
    source: Option<Source>,
    blur_tex: [WebGlTexture; 2],
    blur_fbo: [WebGlFramebuffer; 2],
    lut_tex: WebGlTexture,
    lut_key: Option<String>,
    float_targets: bool,
    pub max_texture: u32,
    max_target: u32,
    scratch_fbo: WebGlFramebuffer,
    scratch_tex: WebGlTexture,
    scratch_size: (u32, u32),
}

impl Renderer {
    pub fn new(canvas: &HtmlCanvasElement) -> Result<Self, String> {
        let attrs = WebGlContextAttributes::new();
        attrs.set_alpha(true);
        attrs.set_premultiplied_alpha(true);
        attrs.set_antialias(false);
        attrs.set_depth(false);
        attrs.set_stencil(false);
        attrs.set_preserve_drawing_buffer(false);
        attrs.set_power_preference(WebGlPowerPreference::HighPerformance);
        let gl: GL = canvas
            .get_context_with_context_options("webgl2", &attrs)
            .map_err(|_| "WebGL2 is not available")?
            .ok_or("WebGL2 is not available on this device")?
            .dyn_into()
            .map_err(|_| "WebGL2 is not available")?;

        let float_targets = gl.get_extension("EXT_color_buffer_float").ok().flatten().is_some();
        let max_texture = gl.get_parameter(GL::MAX_TEXTURE_SIZE).ok().and_then(|v| v.as_f64()).unwrap_or(4096.0) as u32;
        let max_rb = gl.get_parameter(GL::MAX_RENDERBUFFER_SIZE).ok().and_then(|v| v.as_f64()).unwrap_or(4096.0) as u32;
        let dims = gl.get_parameter(GL::MAX_VIEWPORT_DIMS).ok();
        let max_vp = dims
            .and_then(|d| d.dyn_into::<js_sys::Int32Array>().ok())
            .map(|a| a.get_index(0).min(a.get_index(1)) as u32)
            .unwrap_or(4096);
        let max_target = max_texture.min(max_rb).min(max_vp).min(4096);

        let develop = Program::new(&gl, DEVELOP_FS, DEVELOP_UNIFORMS)?;
        let lum = Program::new(&gl, LUM_FS, &["u_src", "u_target", "u_footprint", "u_lod"])?;
        let blur = Program::new(&gl, BLUR_FS, &["u_in", "u_target", "u_dir", "u_w"])?;

        let mk_tex = || gl.create_texture().ok_or("Could not create texture");
        let mk_fbo = || gl.create_framebuffer().ok_or("Could not create framebuffer");
        let r = Self {
            blur_tex: [mk_tex()?, mk_tex()?],
            blur_fbo: [mk_fbo()?, mk_fbo()?],
            lut_tex: mk_tex()?,
            scratch_fbo: mk_fbo()?,
            scratch_tex: mk_tex()?,
            gl,
            canvas: canvas.clone(),
            develop,
            lum,
            blur,
            source: None,
            lut_key: None,
            float_targets,
            max_texture,
            max_target,
            scratch_size: (0, 0),
        };
        r.init_static_textures();
        Ok(r)
    }

    fn init_static_textures(&self) {
        let gl = &self.gl;
        // Identity curve until a real one is uploaded.
        let ident: Vec<f32> = (0..LUT_SIZE)
            .flat_map(|i| {
                let x = i as f32 / (LUT_SIZE - 1) as f32;
                [x, x, x, 1.0]
            })
            .collect();
        self.upload_lut(&ident);
        // 1×1 grey blur map so the shader always has something bound.
        for t in &self.blur_tex {
            gl.bind_texture(GL::TEXTURE_2D, Some(t));
            set_linear_clamp(gl, false);
            let _ = gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
                GL::TEXTURE_2D,
                0,
                GL::RGBA8 as i32,
                1,
                1,
                0,
                GL::RGBA,
                GL::UNSIGNED_BYTE,
                Some(&[128, 128, 128, 255]),
            );
        }
    }





    /// Upload an 8-bit bitmap decoded by the browser.
    pub fn set_source_bitmap(&mut self, bmp: &ImageBitmap, full: (u32, u32)) -> Result<(), String> {
        let gl = &self.gl;
        let tex = gl.create_texture().ok_or("Could not create texture")?;
        gl.bind_texture(GL::TEXTURE_2D, Some(&tex));
        gl.pixel_storei(GL::UNPACK_FLIP_Y_WEBGL, 0);
        gl.pixel_storei(GL::UNPACK_PREMULTIPLY_ALPHA_WEBGL, 0);
        gl.pixel_storei(GL::UNPACK_COLORSPACE_CONVERSION_WEBGL, GL::NONE as i32);
        gl.tex_image_2d_with_u32_and_u32_and_image_bitmap(GL::TEXTURE_2D, 0, GL::RGBA8 as i32, GL::RGBA, GL::UNSIGNED_BYTE, bmp)
            .map_err(|_| "Could not upload the photo to the GPU")?;
        gl.generate_mipmap(GL::TEXTURE_2D);
        set_linear_clamp(gl, true);
        self.replace_source(Source { tex, width: bmp.width(), height: bmp.height(), full_width: full.0, full_height: full.1 });
        Ok(())
    }

    /// Upload half-float RGBA pixels (RAW / TIFF decoded by the engine).
    pub fn set_source_f16(&mut self, w: u32, h: u32, data: &[u16], full: (u32, u32)) -> Result<(), String> {
        let gl = &self.gl;
        let tex = gl.create_texture().ok_or("Could not create texture")?;
        gl.bind_texture(GL::TEXTURE_2D, Some(&tex));
        gl.pixel_storei(GL::UNPACK_FLIP_Y_WEBGL, 0);
        // SAFETY: the view is consumed by texImage2D before any wasm allocation can move memory.
        let view = unsafe { js_sys::Uint16Array::view(data) };
        gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_array_buffer_view(
            GL::TEXTURE_2D,
            0,
            GL::RGBA16F as i32,
            w as i32,
            h as i32,
            0,
            GL::RGBA,
            GL::HALF_FLOAT,
            Some(&view),
        )
        .map_err(|_| "Could not upload the photo to the GPU")?;
        // Mipmaps of float textures need float render targets.
        let mips = self.float_targets;
        if mips {
            gl.generate_mipmap(GL::TEXTURE_2D);
        }
        set_linear_clamp(gl, mips);
        self.replace_source(Source { tex, width: w, height: h, full_width: full.0, full_height: full.1 });
        Ok(())
    }

    fn replace_source(&mut self, s: Source) {
        if let Some(old) = self.source.take() {
            self.gl.delete_texture(Some(&old.tex));
        }
        self.source = Some(s);
        self.compute_blur();
    }

    /// Build the low-frequency luminance map (same recipe as `BlurMap::new`).
    fn compute_blur(&self) {
        let Some(src) = &self.source else { return };
        let gl = &self.gl;
        let scale = (BLUR_EDGE as f32 / src.width.max(src.height) as f32).min(1.0);
        let w = ((src.width as f32 * scale).round() as i32).max(1);
        let h = ((src.height as f32 * scale).round() as i32).max(1);
        let (ifmt, ty) = if self.float_targets { (GL::RGBA16F, GL::HALF_FLOAT) } else { (GL::RGBA8, GL::UNSIGNED_BYTE) };
        for i in 0..2 {
            gl.bind_texture(GL::TEXTURE_2D, Some(&self.blur_tex[i]));
            let _ = gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
                GL::TEXTURE_2D, 0, ifmt as i32, w, h, 0, GL::RGBA, ty, None,
            );
            set_linear_clamp(gl, false);
            gl.bind_framebuffer(GL::FRAMEBUFFER, Some(&self.blur_fbo[i]));
            gl.framebuffer_texture_2d(GL::FRAMEBUFFER, GL::COLOR_ATTACHMENT0, GL::TEXTURE_2D, Some(&self.blur_tex[i]), 0);
        }
        gl.viewport(0, 0, w, h);
        gl.disable(GL::BLEND);

        // Pass 1: luminance, area averaged.
        gl.bind_framebuffer(GL::FRAMEBUFFER, Some(&self.blur_fbo[0]));
        gl.use_program(Some(&self.lum.program));
        gl.active_texture(GL::TEXTURE0);
        gl.bind_texture(GL::TEXTURE_2D, Some(&src.tex));
        gl.uniform1i(self.lum.u("u_src"), 0);
        gl.uniform2f(self.lum.u("u_target"), w as f32, h as f32);
        gl.uniform2f(self.lum.u("u_footprint"), 1.0 / w as f32, 1.0 / h as f32);
        let lod = ((src.width as f32 / w as f32).log2() - 2.0).max(0.0);
        gl.uniform1f(self.lum.u("u_lod"), lod);
        gl.draw_arrays(GL::TRIANGLES, 0, 3);

        // Passes 2 and 3: separable Gaussian.
        let k = gaussian_kernel();
        gl.use_program(Some(&self.blur.program));
        gl.uniform1fv_with_f32_array(self.blur.u("u_w"), &k);
        gl.uniform2f(self.blur.u("u_target"), w as f32, h as f32);
        gl.uniform1i(self.blur.u("u_in"), 0);
        for (from, to, dir) in [(0usize, 1usize, (1.0 / w as f32, 0.0)), (1, 0, (0.0, 1.0 / h as f32))] {
            gl.bind_framebuffer(GL::FRAMEBUFFER, Some(&self.blur_fbo[to]));
            gl.bind_texture(GL::TEXTURE_2D, Some(&self.blur_tex[from]));
            gl.uniform2f(self.blur.u("u_dir"), dir.0, dir.1);
            gl.draw_arrays(GL::TRIANGLES, 0, 3);
        }
        gl.bind_framebuffer(GL::FRAMEBUFFER, None);
    }

    fn upload_lut(&self, data: &[f32]) {
        let gl = &self.gl;
        gl.bind_texture(GL::TEXTURE_2D, Some(&self.lut_tex));
        // SAFETY: consumed immediately by texImage2D.
        let view = unsafe { js_sys::Float32Array::view(data) };
        let _ = gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_array_buffer_view(
            GL::TEXTURE_2D,
            0,
            GL::RGBA16F as i32,
            LUT_SIZE as i32,
            1,
            0,
            GL::RGBA,
            GL::FLOAT,
            Some(&view),
        );
        set_linear_clamp(gl, false);
    }

    fn bind_develop(&mut self, adj: &Adjustments, p: &Prepared, place: &Placement, target: (f32, f32), offset: (f32, f32)) {
        let Some(src) = &self.source else { return };
        if p.curves_active {
            let key = serde_json::to_string(&adj.curves).unwrap_or_default();
            if self.lut_key.as_deref() != Some(key.as_str()) {
                self.upload_lut(&p.lut);
                self.lut_key = Some(key);
            }
        }
        let gl = &self.gl;
        let d = &self.develop;
        gl.use_program(Some(&d.program));
        gl.active_texture(GL::TEXTURE0);
        gl.bind_texture(GL::TEXTURE_2D, Some(&src.tex));
        gl.active_texture(GL::TEXTURE1);
        gl.bind_texture(GL::TEXTURE_2D, Some(&self.blur_tex[0]));
        gl.active_texture(GL::TEXTURE2);
        gl.bind_texture(GL::TEXTURE_2D, Some(&self.lut_tex));
        gl.active_texture(GL::TEXTURE0);
        gl.uniform1i(d.u("u_src"), 0);
        gl.uniform1i(d.u("u_blur"), 1);
        gl.uniform1i(d.u("u_lut"), 2);

        gl.uniform2f(d.u("u_target"), target.0, target.1);
        gl.uniform2f(d.u("u_offset"), offset.0, offset.1);
        gl.uniform4f(d.u("u_imgRect"), place.rect[0], place.rect[1], place.rect[2], place.rect[3]);
        gl.uniform2f(d.u("u_srcSize"), src.width as f32, src.height as f32);
        gl.uniform2f(d.u("u_srcFull"), src.full_width as f32, src.full_height as f32);

        let g: Geometry = adj.geometry;
        let crop = if place.uncropped { Rect::FULL } else { g.crop };
        gl.uniform1i(d.u("u_turns"), (g.quarter_turns % 4) as i32);
        gl.uniform2f(d.u("u_flip"), g.flip_h as i32 as f32, g.flip_v as i32 as f32);
        gl.uniform1f(d.u("u_straighten"), g.straighten.to_radians());
        gl.uniform4f(d.u("u_crop"), crop.x, crop.y, crop.w, crop.h);
        gl.uniform1f(d.u("u_split"), place.split);
        gl.uniform1i(d.u("u_bypass"), place.bypass as i32);

        gl.uniform3f(d.u("u_gain"), p.gain[0], p.gain[1], p.gain[2]);
        gl.uniform4f(d.u("u_tone"), p.highlights, p.shadows, p.whites, p.blacks);
        gl.uniform1f(d.u("u_contrast"), p.contrast);
        gl.uniform3f(d.u("u_local"), p.clarity, p.dehaze, p.sharpen);
        gl.uniform2f(d.u("u_color"), p.saturation, p.vibrance);
        let hsl: Vec<f32> = p.hsl.iter().flatten().copied().collect();
        gl.uniform3fv_with_f32_array(d.u("u_hsl"), &hsl);
        gl.uniform1i(d.u("u_hslOn"), p.hsl_active as i32);
        let grade: Vec<f32> = p.grade.iter().flatten().copied().collect();
        gl.uniform4fv_with_f32_array(d.u("u_grade"), &grade);
        gl.uniform2f(d.u("u_gradeOn"), p.grade_active as i32 as f32, p.balance);
        gl.uniform2f(d.u("u_vig"), p.vignette, p.feather);
        gl.uniform2f(d.u("u_grain"), p.grain, p.grain_cell);
        gl.uniform1i(d.u("u_curveOn"), p.curves_active as i32);
        gl.uniform1f(d.u("u_lutSize"), LUT_SIZE as f32);
    }

    /// Draw to the on-screen canvas.
    pub fn render(&mut self, adj: &Adjustments, place: &Placement) {
        let gl = self.gl.clone();
        let (w, h) = (self.canvas.width(), self.canvas.height());
        gl.bind_framebuffer(GL::FRAMEBUFFER, None);
        gl.viewport(0, 0, w as i32, h as i32);
        gl.clear_color(0.0, 0.0, 0.0, 0.0);
        gl.clear(GL::COLOR_BUFFER_BIT);
        if self.source.is_none() {
            return;
        }
        let p = Prepared::new(adj);
        self.bind_develop(adj, &p, place, (w as f32, h as f32), (0.0, 0.0));
        gl.draw_arrays(GL::TRIANGLES, 0, 3);
    }

    fn ensure_scratch(&mut self, w: u32, h: u32) {
        if self.scratch_size == (w, h) {
            return;
        }
        let gl = &self.gl;
        gl.bind_texture(GL::TEXTURE_2D, Some(&self.scratch_tex));
        let _ = gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
            GL::TEXTURE_2D, 0, GL::RGBA8 as i32, w as i32, h as i32, 0, GL::RGBA, GL::UNSIGNED_BYTE, None,
        );
        set_linear_clamp(gl, false);
        gl.bind_framebuffer(GL::FRAMEBUFFER, Some(&self.scratch_fbo));
        gl.framebuffer_texture_2d(GL::FRAMEBUFFER, GL::COLOR_ATTACHMENT0, GL::TEXTURE_2D, Some(&self.scratch_tex), 0);
        self.scratch_size = (w, h);
    }

    /// Render the developed output at `w`×`h` and read it back as top-down RGBA rows.
    /// Large sizes are rendered in tiles so any output size works.
    pub fn render_pixels(&mut self, adj: &Adjustments, w: u32, h: u32) -> Vec<u8> {
        let mut out = vec![0u8; w as usize * h as usize * 4];
        if self.source.is_none() || w == 0 || h == 0 {
            return out;
        }
        let tile = self.max_target.max(256);
        let p = Prepared::new(adj);
        let place = Placement { rect: [0.0, 0.0, w as f32, h as f32], split: -1.0, bypass: false, uncropped: false };
        let tw = tile.min(w);
        let th = tile.min(h);
        self.ensure_scratch(tw, th);
        let gl = self.gl.clone();
        let mut buf = vec![0u8; tw as usize * th as usize * 4];
        let mut ty = 0;
        while ty < h {
            let rh = th.min(h - ty);
            let mut tx = 0;
            while tx < w {
                let rw = tw.min(w - tx);
                gl.bind_framebuffer(GL::FRAMEBUFFER, Some(&self.scratch_fbo));
                gl.viewport(0, 0, rw as i32, rh as i32);
                gl.clear_color(0.0, 0.0, 0.0, 0.0);
                gl.clear(GL::COLOR_BUFFER_BIT);
                self.bind_develop(adj, &p, &place, (rw as f32, rh as f32), (tx as f32, ty as f32));
                gl.draw_arrays(GL::TRIANGLES, 0, 3);
                let n = rw as usize * rh as usize * 4;
                let _ = gl.read_pixels_with_opt_u8_array(0, 0, rw as i32, rh as i32, GL::RGBA, GL::UNSIGNED_BYTE, Some(&mut buf[..n]));
                // GL rows are bottom-up.
                for row in 0..rh as usize {
                    let src = &buf[(rh as usize - 1 - row) * rw as usize * 4..][..rw as usize * 4];
                    let dst = ((ty as usize + row) * w as usize + tx as usize) * 4;
                    out[dst..dst + rw as usize * 4].copy_from_slice(src);
                }
                tx += rw;
            }
            ty += rh;
        }
        gl.bind_framebuffer(GL::FRAMEBUFFER, None);
        out
    }

    /// Output size of `adj` for the current photo, at full resolution.
    pub fn output_size(&self, adj: &Adjustments) -> (u32, u32) {
        match &self.source {
            Some(s) => {
                let (w, h) = mirror_core::geometry::output_size(&adj.geometry, s.width, s.height);
                (w, h)
            }
            None => (0, 0),
        }
    }

}

fn set_linear_clamp(gl: &GL, mipmaps: bool) {
    let min = if mipmaps { GL::LINEAR_MIPMAP_LINEAR } else { GL::LINEAR };
    gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MIN_FILTER, min as i32);
    gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MAG_FILTER, GL::LINEAR as i32);
    gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_S, GL::CLAMP_TO_EDGE as i32);
    gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_T, GL::CLAMP_TO_EDGE as i32);
}

impl Drop for Renderer {
    fn drop(&mut self) {
        if let Some(s) = self.source.take() {
            self.gl.delete_texture(Some(&s.tex));
        }
    }
}
