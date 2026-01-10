// Basic vertex and fragment shaders for rendering colored triangles

// Vertex input structure
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
}

// Vertex output / Fragment input structure
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
}

// Uniform buffer for transformation matrices
struct Uniforms {
    view_proj: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

// Vertex shader
@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    
    // Transform vertex position by view-projection matrix
    output.clip_position = uniforms.view_proj * vec4<f32>(input.position, 1.0);
    
    // Pass through vertex color
    output.color = input.color;
    
    return output;
}

// Fragment shader
@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Output the interpolated vertex color with full alpha
    return vec4<f32>(input.color, 1.0);
}