// Chunk rendering shaders for voxel blocks

// Vertex input structure matching ChunkVertex
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coords: vec2<f32>,
}

// Vertex output / Fragment input structure
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coords: vec2<f32>,
}

// Uniform buffer for chunk rendering
struct ChunkUniforms {
    view_proj: mat4x4<f32>,
    chunk_position: vec3<f32>,
    _padding: f32,
}

@group(0) @binding(0)
var<uniform> uniforms: ChunkUniforms;

// Vertex shader
@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    
    // Calculate world position by adding chunk position offset
    let world_pos = input.position + uniforms.chunk_position;
    
    // Transform vertex position by view-projection matrix
    output.clip_position = uniforms.view_proj * vec4<f32>(world_pos, 1.0);
    
    // Pass through world position for fragment shader
    output.world_position = world_pos;
    
    // Pass through normal (could be transformed by model matrix if needed)
    output.normal = input.normal;
    
    // Pass through texture coordinates
    output.tex_coords = input.tex_coords;
    
    return output;
}

// Fragment shader
@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Simple lighting calculation using normal
    let light_dir = normalize(vec3<f32>(0.5, 1.0, 0.3));
    let light_intensity = max(dot(normalize(input.normal), light_dir), 0.2);
    
    // Base color based on texture coordinates (simple checkerboard pattern)
    let checker = floor(input.tex_coords.x * 8.0) + floor(input.tex_coords.y * 8.0);
    let base_color = select(vec3<f32>(0.8, 0.8, 0.8), vec3<f32>(0.6, 0.6, 0.6), (checker % 2.0) == 0.0);
    
    // Apply lighting
    let final_color = base_color * light_intensity;
    
    return vec4<f32>(final_color, 1.0);
}