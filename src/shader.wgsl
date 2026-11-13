// TaleRender Milestone 1 smoke shader.
// A single fullscreen triangle; the fragment shader draws a simple test
// scene (sky gradient, sun disc, ground band, character-silhouette card)
// so the first native frame is a real wgpu render of a 1080x1920 target,
// not a solid colour. Geometry, materials and lighting arrive in later
// milestones — this shader is scaffolding and is labelled as such.

struct VsOut {
    @builtin(position) pos: vec4f,
    @location(0) uv: vec2f,
};

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VsOut {
    var corners = array<vec2f, 3>(
        vec2f(-1.0, -1.0),
        vec2f(3.0, -1.0),
        vec2f(-1.0, 3.0),
    );
    let p = corners[idx];
    var out: VsOut;
    out.pos = vec4f(p, 0.0, 1.0);
    out.uv = p * 0.5 + vec2f(0.5, 0.5);
    return out;
}

fn sd_circle(p: vec2f, c: vec2f, r: f32) -> f32 {
    return length(p - c) - r;
}

fn sd_box(p: vec2f, c: vec2f, b: vec2f) -> f32 {
    let d = abs(p - c) - b;
    return length(max(d, vec2f(0.0, 0.0))) + min(max(d.x, d.y), 0.0);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4f {
    // Work in a portrait aspect space so shapes are not stretched.
    let aspect = 1080.0 / 1920.0;
    var p = vec2f(in.uv.x * aspect, in.uv.y);

    // Sky: vertical gradient, deep blue-grey to warm horizon.
    let sky_top = vec3f(0.10, 0.16, 0.30);
    let sky_low = vec3f(0.85, 0.62, 0.42);
    var col = mix(sky_low, sky_top, smoothstep(0.25, 1.0, in.uv.y));

    // Sun disc with a soft edge.
    let sun = sd_circle(p, vec2f(0.36 * aspect + 0.22, 0.78), 0.085);
    col = mix(vec3f(1.0, 0.85, 0.55), col, smoothstep(-0.01, 0.015, sun));

    // Ground band.
    let ground = step(in.uv.y, 0.28);
    col = mix(col, vec3f(0.16, 0.28, 0.16), ground);

    // Character card: a standing rounded-rect silhouette + head circle,
    // the placeholder for the 2D character plane (Milestone 3/4 work).
    let body = sd_box(p, vec2f(aspect * 0.5, 0.36), vec2f(0.055, 0.13));
    let head = sd_circle(p, vec2f(aspect * 0.5, 0.565), 0.052);
    let figure = min(body, head);
    let aa = 0.004;
    col = mix(vec3f(0.13, 0.13, 0.16), col, smoothstep(-aa, aa, figure));
    // Rim light on the figure's sun side.
    let rim = smoothstep(0.0, -0.012, figure) * smoothstep(-0.05, 0.0, figure)
        * step(0.0, p.x - aspect * 0.5);
    col += vec3f(0.35, 0.22, 0.08) * rim;

    return vec4f(col, 1.0);
}
