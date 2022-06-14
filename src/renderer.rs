// copyright 2022 Remi Bernotavicius

use euclid::{Length, Point2D, Rect, Scale, Size2D};
use wasm_bindgen::JsCast;
use web_sys::{WebGl2RenderingContext, WebGlProgram, WebGlShader, WebGlTexture};

/// Unit for pixels of the renderer
pub struct Pixels;

/// Unit for pixels in WebGl
pub struct WebGlPixels;

/// Unit for a number of bytes
struct Bytes;

pub fn compile_shader(
    context: &WebGl2RenderingContext,
    shader_type: u32,
    source: &str,
) -> Result<WebGlShader, String> {
    let shader = context
        .create_shader(shader_type)
        .ok_or_else(|| String::from("Unable to create shader object"))?;
    context.shader_source(&shader, source);
    context.compile_shader(&shader);

    if context
        .get_shader_parameter(&shader, WebGl2RenderingContext::COMPILE_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(shader)
    } else {
        Err(context
            .get_shader_info_log(&shader)
            .unwrap_or_else(|| String::from("Unknown error creating shader")))
    }
}

pub fn link_program(
    context: &WebGl2RenderingContext,
    vert_shader: &WebGlShader,
    frag_shader: &WebGlShader,
) -> Result<WebGlProgram, String> {
    let program = context
        .create_program()
        .ok_or_else(|| String::from("Unable to create shader object"))?;

    context.attach_shader(&program, vert_shader);
    context.attach_shader(&program, frag_shader);
    context.link_program(&program);

    if context
        .get_program_parameter(&program, WebGl2RenderingContext::LINK_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(program)
    } else {
        Err(context
            .get_program_info_log(&program)
            .unwrap_or_else(|| String::from("Unknown error creating program object")))
    }
}

pub struct CanvasRenderer {
    context: WebGl2RenderingContext,
    texture: WebGlTexture,
    buffer: Vec<u8>,
}

pub const RENDER_RECT: Rect<i32, Pixels> = Rect {
    origin: Point2D::<i32, Pixels>::new(0, 0),
    size: Size2D::<i32, Pixels>::new(315, 143),
};

pub const PIXEL_SCALE: Scale<i32, Pixels, WebGlPixels> = Scale::new(4);

fn get_rendering_context(canvas: &web_sys::HtmlCanvasElement) -> WebGl2RenderingContext {
    canvas
        .get_context("webgl2")
        .unwrap()
        .unwrap()
        .dyn_into::<WebGl2RenderingContext>()
        .unwrap()
}

fn set_rectangle(context: &WebGl2RenderingContext, x: f32, y: f32, width: f32, height: f32) {
    let x1 = x;
    let x2 = x + width;
    let y1 = y;
    let y2 = y + height;
    let data: [f32; 12] = [x1, y1, x2, y1, x1, y2, x1, y2, x2, y1, x2, y2];
    unsafe {
        let data_array = js_sys::Float32Array::view(&data);
        context.buffer_data_with_array_buffer_view(
            WebGl2RenderingContext::ARRAY_BUFFER,
            &data_array,
            WebGl2RenderingContext::STATIC_DRAW,
        );
    }
}

fn set_up_context(context: &WebGl2RenderingContext, texture: &WebGlTexture) {
    let vert_shader = compile_shader(
        context,
        WebGl2RenderingContext::VERTEX_SHADER,
        r#"# version 300 es
        // an attribute is an input (in) to a vertex shader.
        // It will receive data from a buffer
        in vec2 a_position;
        in vec2 a_texCoord;

        // Used to pass in the resolution of the canvas
        uniform vec2 u_resolution;

        // Used to pass the texture coordinates to the fragment shader
        out vec2 v_texCoord;

        // all shaders have a main function
        void main() {

          // convert the position from pixels to 0.0 to 1.0
          vec2 zeroToOne = a_position / u_resolution;

          // convert from 0->1 to 0->2
          vec2 zeroToTwo = zeroToOne * 2.0;

          // convert from 0->2 to -1->+1 (clipspace)
          vec2 clipSpace = zeroToTwo - 1.0;

          gl_Position = vec4(clipSpace * vec2(1, -1), 0, 1);

          // pass the texCoord to the fragment shader
          // The GPU will interpolate this value between points.
          v_texCoord = a_texCoord;
        }
        "#,
    )
    .unwrap();
    let frag_shader = compile_shader(
        context,
        WebGl2RenderingContext::FRAGMENT_SHADER,
        r#"# version 300 es
        // fragment shaders don't have a default precision so we need
        // to pick one. highp is a good default. It means "high precision"
        precision highp float;

        // our texture
        uniform sampler2D u_image;

        // the texCoords passed in from the vertex shader.
        in vec2 v_texCoord;

        // we need to declare an output for the fragment shader
        out vec4 outColor;

        void main() {
          outColor = texture(u_image, v_texCoord);
        }
        "#,
    )
    .unwrap();

    let program = link_program(context, &vert_shader, &frag_shader).unwrap();

    let position_attribute_location: u32 = context
        .get_attrib_location(&program, "a_position")
        .try_into()
        .unwrap();
    let texcoord_attribute_location: u32 = context
        .get_attrib_location(&program, "a_texCoord")
        .try_into()
        .unwrap();

    let resolution_location = context
        .get_uniform_location(&program, "u_resolution")
        .unwrap();
    let image_location = context.get_uniform_location(&program, "u_image").unwrap();

    let vao = context.create_vertex_array().unwrap();
    context.bind_vertex_array(Some(&vao));

    let position_buffer = context.create_buffer().unwrap();
    context.enable_vertex_attrib_array(position_attribute_location);
    context.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&position_buffer));
    context.vertex_attrib_pointer_with_i32(
        position_attribute_location,
        2,
        WebGl2RenderingContext::FLOAT,
        false,
        0,
        0,
    );
    let texcoord_buffer = context.create_buffer().unwrap();
    context.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&texcoord_buffer));

    let data: [f32; 12] = [0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 1.0];
    unsafe {
        let data_array = js_sys::Float32Array::view(&data);
        context.buffer_data_with_array_buffer_view(
            WebGl2RenderingContext::ARRAY_BUFFER,
            &data_array,
            WebGl2RenderingContext::STATIC_DRAW,
        );
    }

    context.enable_vertex_attrib_array(texcoord_attribute_location);

    context.vertex_attrib_pointer_with_i32(
        texcoord_attribute_location,
        2,
        WebGl2RenderingContext::FLOAT,
        false,
        0,
        0,
    );

    context.active_texture(WebGl2RenderingContext::TEXTURE0);
    context.bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(texture));

    context.tex_parameteri(
        WebGl2RenderingContext::TEXTURE_2D,
        WebGl2RenderingContext::TEXTURE_WRAP_S,
        WebGl2RenderingContext::CLAMP_TO_EDGE as i32,
    );
    context.tex_parameteri(
        WebGl2RenderingContext::TEXTURE_2D,
        WebGl2RenderingContext::TEXTURE_WRAP_T,
        WebGl2RenderingContext::CLAMP_TO_EDGE as i32,
    );
    context.tex_parameteri(
        WebGl2RenderingContext::TEXTURE_2D,
        WebGl2RenderingContext::TEXTURE_MIN_FILTER,
        WebGl2RenderingContext::NEAREST as i32,
    );
    context.tex_parameteri(
        WebGl2RenderingContext::TEXTURE_2D,
        WebGl2RenderingContext::TEXTURE_MAG_FILTER,
        WebGl2RenderingContext::NEAREST as i32,
    );

    let screen_rect = RENDER_RECT * PIXEL_SCALE;

    context.viewport(0, 0, screen_rect.size.width, screen_rect.size.height);
    context.use_program(Some(&program));

    let width = screen_rect.size.width as f32;
    let height = screen_rect.size.height as f32;

    context.uniform2f(Some(&resolution_location), width, height);
    context.uniform1i(Some(&image_location), 0);

    context.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&position_buffer));
    set_rectangle(context, 0.0, 0.0, width, height);
}

#[derive(Clone, Debug, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// red, green, blue, and alpha
const BYTES_PER_PIXEL: Scale<usize, Pixels, Bytes> = Scale::new(4);

impl CanvasRenderer {
    pub fn new() -> Self {
        let canvas = super::canvas();
        let context = get_rendering_context(&canvas);
        let texture = context.create_texture().unwrap();
        set_up_context(&context, &texture);
        Self {
            context,
            texture,
            buffer: vec![
                u8::MAX;
                (Length::new(RENDER_RECT.area() as usize) * BYTES_PER_PIXEL).get()
            ],
        }
    }

    pub fn render(&self) {
        self.context
            .draw_arrays(WebGl2RenderingContext::TRIANGLES, 0, 6);
    }

    #[inline(always)]
    pub fn color_pixel(&mut self, pos: Point2D<i32, Pixels>, color: Color) {
        assert!(RENDER_RECT.contains(pos), "{pos:?} not in {RENDER_RECT:?}");

        let i = (Length::new((pos.y * RENDER_RECT.size.width + pos.x) as usize) * BYTES_PER_PIXEL)
            .get();
        self.buffer[i] = color.r;
        self.buffer[i + 1] = color.g;
        self.buffer[i + 2] = color.b;
        self.buffer[i + 3] = 255;
    }

    pub fn present(&mut self) {
        self.context
            .bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(&self.texture));

        self.context
            .tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
                WebGl2RenderingContext::TEXTURE_2D,
                0,
                WebGl2RenderingContext::RGBA as i32,
                RENDER_RECT.size.width,
                RENDER_RECT.size.height,
                0,
                WebGl2RenderingContext::RGBA,
                WebGl2RenderingContext::UNSIGNED_BYTE,
                Some(&self.buffer[..]),
            )
            .unwrap();
    }
}

impl Default for CanvasRenderer {
    fn default() -> Self {
        Self::new()
    }
}
