//! OpenGL Shader Management
//!
//! Handles shader compilation and program linking.

use gl::types::*;
use std::ffi::CString;
use std::ptr;
use std::str;

/// Shader program wrapper
pub struct ShaderProgram {
    pub id: GLuint,
}

impl ShaderProgram {
    /// Create a new shader program from vertex and fragment sources
    pub fn new(vertex_source: &str, fragment_source: &str) -> Result<Self, String> {
        unsafe {
            // Compile vertex shader
            let vertex_shader = compile_shader(vertex_source, gl::VERTEX_SHADER)?;
            
            // Compile fragment shader
            let fragment_shader = compile_shader(fragment_source, gl::FRAGMENT_SHADER)?;
            
            // Link program
            let program = gl::CreateProgram();
            gl::AttachShader(program, vertex_shader);
            gl::AttachShader(program, fragment_shader);
            gl::LinkProgram(program);
            
            // Check for linking errors
            let mut success = gl::FALSE as GLint;
            gl::GetProgramiv(program, gl::LINK_STATUS, &mut success);
            
            if success != gl::TRUE as GLint {
                let mut len = 0;
                gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);
                let mut buffer = vec![0u8; len as usize];
                gl::GetProgramInfoLog(
                    program,
                    len,
                    ptr::null_mut(),
                    buffer.as_mut_ptr() as *mut GLchar,
                );
                let error = String::from_utf8_lossy(&buffer);
                return Err(format!("Program linking failed: {}", error));
            }
            
            // Clean up shaders (they're linked into the program now)
            gl::DeleteShader(vertex_shader);
            gl::DeleteShader(fragment_shader);
            
            Ok(Self { id: program })
        }
    }

    /// Activate this shader program
    pub fn use_program(&self) {
        unsafe {
            gl::UseProgram(self.id);
        }
    }

    /// Get uniform location
    pub fn get_uniform_location(&self, name: &str) -> GLint {
        let c_name = CString::new(name).unwrap();
        unsafe { gl::GetUniformLocation(self.id, c_name.as_ptr()) }
    }

    /// Set uniform mat4
    pub fn set_mat4(&self, name: &str, value: &[f32; 16]) {
        unsafe {
            let location = self.get_uniform_location(name);
            gl::UniformMatrix4fv(location, 1, gl::FALSE, value.as_ptr());
        }
    }
}

impl Drop for ShaderProgram {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.id);
        }
    }
}

/// Compile a single shader
unsafe fn compile_shader(source: &str, shader_type: GLenum) -> Result<GLuint, String> {
    let shader = gl::CreateShader(shader_type);
    let c_source = CString::new(source).unwrap();
    
    gl::ShaderSource(shader, 1, &c_source.as_ptr(), ptr::null());
    gl::CompileShader(shader);
    
    // Check for compilation errors
    let mut success = gl::FALSE as GLint;
    gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut success);
    
    if success != gl::TRUE as GLint {
        let mut len = 0;
        gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
        let mut buffer = vec![0u8; len as usize];
        gl::GetShaderInfoLog(
            shader,
            len,
            ptr::null_mut(),
            buffer.as_mut_ptr() as *mut GLchar,
        );
        let error = String::from_utf8_lossy(&buffer);
        let shader_type_name = if shader_type == gl::VERTEX_SHADER {
            "vertex"
        } else {
            "fragment"
        };
        return Err(format!("{} shader compilation failed: {}", shader_type_name, error));
    }
    
    Ok(shader)
}

/// Default vertex shader for 2D line rendering
pub const DEFAULT_VERTEX_SHADER: &str = r#"
#version 330 core

layout (location = 0) in vec2 aPos;
layout (location = 1) in vec4 aColor;

out vec4 vColor;

uniform mat4 uProjection;
uniform mat4 uView;

void main() {
    gl_Position = uProjection * uView * vec4(aPos, 0.0, 1.0);
    vColor = aColor;
}
"#;

/// Default fragment shader for 2D line rendering
pub const DEFAULT_FRAGMENT_SHADER: &str = r#"
#version 330 core

in vec4 vColor;
out vec4 FragColor;

void main() {
    FragColor = vColor;
}
"#;
