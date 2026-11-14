//! Render-graph declaration + validation layer (Wave 2, seat W13 —
//! Phase 2 of the external audit's roadmap: "a render graph with
//! explicit resource dependencies").
//!
//! Today the post chain — Render → Bokeh → Bloom → Output → GRADE —
//! is hard-wired: `ab1.rs` records its passes in a fixed encoder
//! sequence, `composer.rs::PostChain` hard-codes the bloom cascade
//! inside one method, and `film.rs` / `scenefile.rs` repeat their own
//! orderings. Nothing in the tree can answer "which passes touch
//! texture X, in what order, and is that order forced by data?" —
//! this module is the layer that can.
//!
//! SCOPE — deliberately narrow. This module is *pure data +
//! algorithms*: no wgpu types, no pipelines, no execution. A graph is
//! a declaration: named resources (textures by format + size class,
//! or host data) and named passes that read/write them. Building a
//! graph validates the declaration and derives (a) a deterministic
//! execution order and (b) a per-resource lifetime table that a
//! future transient allocator can consume. Executing passes through
//! the graph is the lead's later migration (see
//! docs/integration/RENDERGRAPH-W13.md); the graph does NOT allocate
//! textures and does NOT insert barriers — wgpu's per-encoder hazard
//! tracking covers the single-encoder recording pattern every
//! renderer in this tree uses, and the note argues why that is
//! acceptable for now.
//!
//! SEMANTICS (the contract the validators enforce):
//!   * Dependencies come from *version binding*. A resource with
//!     several writers holds several values over the frame; each
//!     read binds to one writer's value — the nearest writer
//!     declared BEFORE the reader (the value current at the
//!     reader's point in the declaration, which for a transcribed
//!     graph is source order), or, if no writer precedes it, the
//!     earliest writer after it (a forward reference: legal for a
//!     lone producer/consumer pair, and what makes a true cycle
//!     expressible — and therefore detectable). A bound writer →
//!     reader pair is a dependency edge; the execution order is a
//!     topological sort of those edges.
//!   * A pass may list the same resource in reads AND writes: a
//!     read-modify-write (RMW). Its read binds to the *other*
//!     writers, never to itself, so an RMW pass on a resource nobody
//!     else writes is a read-of-never-written error. Real cases in
//!     the tree: bloom's additive blend onto its input target
//!     (LoadOp::Load in `PostChain::bloom`), the caption overlay
//!     onto the final target (ab1.rs step 8).
//!   * Write–write: two writers of one resource are legal ONLY if
//!     the dependency graph orders them (a path exists one way).
//!     Unordered double-writes would make the surviving value depend
//!     on scheduling — a `WriteConflict` error. Real case: the bloom
//!     composite writes into the cascade's horiz[0] texture
//!     (composer.rs: "Composite into horiz[0]"), legal because the
//!     cascade's data flow orders it after horiz[0]'s first writer.
//!   * Read ambiguity: a reader unordered against ANY writer of the
//!     resource it reads (other than the writer it binds to) could
//!     observe either value — an `AmbiguousRead` error. Ordered
//!     either way (writer before reader, or reader before writer)
//!     is deterministic.
//!   * External resources are declared inputs (scene geometry,
//!     uploaded textures, captured assets): readable without a
//!     writer, never writable. Sink resources are declared outputs
//!     (e.g. readback bytes): writable, exempt from unused checks.
//!   * Execution order: Kahn's algorithm over the binding edges;
//!     whenever several passes are simultaneously ready, the one
//!     DECLARED FIRST wins. The order is therefore a pure function
//!     of the declaration, and for a declaration transcribed in
//!     source order it reproduces the source encoder order exactly
//!     (the fixture test pins this).
//!
//! Determinism law (house rule): no output of this module ever
//! depends on hash-map iteration order — maps are lookup-only, all
//! iteration is over declaration-ordered vectors or sorted sets.

use std::collections::{BTreeSet, HashMap};
use std::fmt;

/// Texture pixel format, mirroring the `wgpu::TextureFormat` values
/// the renderers actually use — re-declared here so this layer stays
/// wgpu-free and unit-testable with a bare `rustc --test`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TexFormat {
    Rgba16Float,
    Rgba8Unorm,
    Depth32Float,
    /// The bokeh depth prepass target: `Rgba8Unorm` by default,
    /// `Rgba16Float` under `TALERENDER_DOF_DEPTH_FORMAT=halffloat`
    /// (ab1.rs) — one logical resource, two physical formats, so it
    /// gets its own class rather than a lie in either direction.
    PackedDepth,
}

/// Where a texture's extent comes from. Sizes are classes, not
/// numbers: the graph orders passes, it does not allocate (yet).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SizeClass {
    /// The frame being rendered (--width x --height).
    FullFrame,
    /// A fixed extent (shadow map 2048x2048, PMREM capture 768x1024).
    Fixed(u32, u32),
    /// UnrealBloomPass mip level n: half the frame, halved n times
    /// with the addon's rounding (composer.rs `bloom_mip_sizes`).
    BloomMip(usize),
    /// Whatever extent the asset/data natively has (painted canvas,
    /// caption strip, geometry buffers, readback bytes).
    Native,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ResourceKind {
    Texture,
    HostData,
}

/// One named resource in the graph.
#[derive(Clone, PartialEq, Debug)]
pub struct ResourceDesc {
    pub name: String,
    pub kind: ResourceKind,
    /// `Some` for textures, `None` for host data.
    pub format: Option<TexFormat>,
    pub size: SizeClass,
    /// Declared input: readable with no writer; passes may not
    /// write it.
    pub external: bool,
    /// Declared output: exempt from the unused-write warning.
    pub sink: bool,
}

impl ResourceDesc {
    pub fn texture(name: &str, format: TexFormat, size: SizeClass) -> ResourceDesc {
        ResourceDesc {
            name: name.to_string(),
            kind: ResourceKind::Texture,
            format: Some(format),
            size,
            external: false,
            sink: false,
        }
    }

    pub fn external_texture(name: &str, format: TexFormat, size: SizeClass) -> ResourceDesc {
        ResourceDesc {
            name: name.to_string(),
            kind: ResourceKind::Texture,
            format: Some(format),
            size,
            external: true,
            sink: false,
        }
    }

    /// Host-side input (geometry/uniform data uploaded per frame).
    pub fn host_input(name: &str) -> ResourceDesc {
        ResourceDesc {
            name: name.to_string(),
            kind: ResourceKind::HostData,
            format: None,
            size: SizeClass::Native,
            external: true,
            sink: false,
        }
    }

    /// Host-side output (readback bytes destined for a PNG file).
    pub fn host_sink(name: &str) -> ResourceDesc {
        ResourceDesc {
            name: name.to_string(),
            kind: ResourceKind::HostData,
            format: None,
            size: SizeClass::Native,
            external: false,
            sink: true,
        }
    }
}

/// One named pass: the resources it samples/attaches as inputs and
/// the resources it renders into / produces. A resource listed in
/// both is a read-modify-write (see module docs).
#[derive(Clone, PartialEq, Debug)]
pub struct PassDesc {
    pub name: String,
    pub reads: Vec<String>,
    pub writes: Vec<String>,
}

impl PassDesc {
    pub fn new(name: &str) -> PassDesc {
        PassDesc { name: name.to_string(), reads: Vec::new(), writes: Vec::new() }
    }

    pub fn reads(mut self, names: &[&str]) -> PassDesc {
        self.reads = names.iter().map(|n| n.to_string()).collect();
        self
    }

    pub fn writes(mut self, names: &[&str]) -> PassDesc {
        self.writes = names.iter().map(|n| n.to_string()).collect();
        self
    }
}

/// A validation failure. `build()` collects ALL failures it can
/// establish (in deterministic declaration order) rather than
/// stopping at the first, so one build report shows the whole
/// breakage.
#[derive(Clone, PartialEq, Debug)]
pub enum GraphError {
    DuplicateResource { name: String },
    DuplicatePass { name: String },
    UnknownResource { pass: String, resource: String },
    WriteToExternal { pass: String, resource: String },
    /// A read of an internal resource no *other* pass writes.
    ReadNeverWritten { pass: String, resource: String },
    /// Two writers of one resource with no dependency path between
    /// them in either direction.
    WriteConflict { resource: String, first: String, second: String },
    /// A reader unordered against some writer of the resource: the
    /// value it would observe is scheduling-dependent.
    AmbiguousRead { resource: String, reader: String, writer: String },
    /// Passes Kahn's algorithm could not schedule: the cycle members
    /// plus anything downstream of them.
    Cycle { passes: Vec<String> },
}

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GraphError::DuplicateResource { name } => write!(f, "resource '{name}' is declared twice"),
            GraphError::DuplicatePass { name } => write!(f, "pass '{name}' is declared twice"),
            GraphError::UnknownResource { pass, resource } => {
                write!(f, "pass '{pass}' references undeclared resource '{resource}'")
            }
            GraphError::WriteToExternal { pass, resource } => {
                write!(f, "pass '{pass}' writes '{resource}', which is a declared external input")
            }
            GraphError::ReadNeverWritten { pass, resource } => {
                write!(f, "pass '{pass}' reads '{resource}', which no other pass writes")
            }
            GraphError::WriteConflict { resource, first, second } => {
                write!(f, "resource '{resource}' is written by both '{first}' and '{second}' with no dependency ordering them")
            }
            GraphError::AmbiguousRead { resource, reader, writer } => {
                write!(f, "pass '{reader}' reads '{resource}' but is unordered against its writer '{writer}': the observed value is scheduling-dependent")
            }
            GraphError::Cycle { passes } => {
                write!(f, "dependency cycle: passes cannot be scheduled: {}", passes.join(", "))
            }
        }
    }
}

/// A non-fatal observation about a valid graph.
#[derive(Clone, PartialEq, Debug)]
pub enum GraphWarning {
    /// Written by a pass but read by none and not a declared sink.
    /// In practice: depth attachments and the MSAA colour buffer,
    /// whose consumption happens *inside* their own pass (depth
    /// testing / resolve) and so is invisible to a cross-pass graph.
    UnusedWrite { resource: String, writer: String },
    /// A declared external input no pass reads.
    UnusedExternal { resource: String },
}

impl fmt::Display for GraphWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GraphWarning::UnusedWrite { resource, writer } => {
                write!(f, "resource '{resource}' is written by '{writer}' but never read (in-pass consumption?)")
            }
            GraphWarning::UnusedExternal { resource } => {
                write!(f, "external input '{resource}' is declared but never read")
            }
        }
    }
}

/// Per-resource lifetime over the execution order: the resource must
/// exist from `first_use` through `last_use` (positions in
/// `Graph::order`). This is the interval table a transient allocator
/// consumes: two resources whose intervals do not overlap may share
/// one physical texture.
#[derive(Clone, PartialEq, Debug)]
pub struct Lifetime {
    pub resource: String,
    pub first_use: usize,
    pub last_use: usize,
    /// Pass index (declaration order) of the earliest writer in
    /// execution order; `None` for external inputs.
    pub writer: Option<usize>,
}

/// A validated graph: declaration, derived execution order, and the
/// lifetime table.
pub struct Graph {
    resources: Vec<ResourceDesc>,
    passes: Vec<PassDesc>,
    order: Vec<usize>,
    lifetimes: Vec<Lifetime>,
    warnings: Vec<GraphWarning>,
    edges: Vec<(usize, usize)>,
}

impl Graph {
    /// Pass indices in execution order.
    pub fn order(&self) -> &[usize] {
        &self.order
    }

    /// Pass names in execution order.
    pub fn order_names(&self) -> Vec<&str> {
        self.order.iter().map(|&i| self.passes[i].name.as_str()).collect()
    }

    /// Dependency edges as (writer pass, reader pass) name pairs,
    /// in deterministic (declaration, declaration) order.
    pub fn edges(&self) -> Vec<(&str, &str)> {
        self.edges
            .iter()
            .map(|&(a, b)| (self.passes[a].name.as_str(), self.passes[b].name.as_str()))
            .collect()
    }

    /// Lifetime table, in resource declaration order; resources with
    /// no uses at all are omitted (see `warnings`).
    pub fn lifetimes(&self) -> &[Lifetime] {
        &self.lifetimes
    }

    pub fn lifetime_of(&self, resource: &str) -> Option<&Lifetime> {
        self.lifetimes.iter().find(|l| l.resource == resource)
    }

    pub fn warnings(&self) -> &[GraphWarning] {
        &self.warnings
    }

    pub fn resources(&self) -> &[ResourceDesc] {
        &self.resources
    }

    pub fn passes(&self) -> &[PassDesc] {
        &self.passes
    }
}

/// Accumulates a declaration, then validates it in `build()`.
#[derive(Default)]
pub struct GraphBuilder {
    resources: Vec<ResourceDesc>,
    passes: Vec<PassDesc>,
}

impl GraphBuilder {
    pub fn new() -> GraphBuilder {
        GraphBuilder::default()
    }

    pub fn add_resource(&mut self, desc: ResourceDesc) -> &mut GraphBuilder {
        self.resources.push(desc);
        self
    }

    pub fn add_pass(&mut self, desc: PassDesc) -> &mut GraphBuilder {
        self.passes.push(desc);
        self
    }

    pub fn build(self) -> Result<Graph, Vec<GraphError>> {
        let mut errors: Vec<GraphError> = Vec::new();

        // ---- name indices (first declaration wins; dups are errors)
        let mut res_idx: HashMap<&str, usize> = HashMap::new();
        for (i, r) in self.resources.iter().enumerate() {
            if res_idx.insert(r.name.as_str(), i).is_some() {
                errors.push(GraphError::DuplicateResource { name: r.name.clone() });
            }
        }
        let mut pass_idx: HashMap<&str, usize> = HashMap::new();
        for (i, p) in self.passes.iter().enumerate() {
            if pass_idx.insert(p.name.as_str(), i).is_some() {
                errors.push(GraphError::DuplicatePass { name: p.name.clone() });
            }
        }

        // ---- resolve pass references; check external writes
        // reads_of[p] / writes_of[p]: resource indices, declaration
        // order preserved, duplicates within one pass collapsed.
        let np = self.passes.len();
        let mut reads_of: Vec<Vec<usize>> = vec![Vec::new(); np];
        let mut writes_of: Vec<Vec<usize>> = vec![Vec::new(); np];
        for (pi, p) in self.passes.iter().enumerate() {
            for name in &p.reads {
                match res_idx.get(name.as_str()) {
                    Some(&ri) => {
                        if !reads_of[pi].contains(&ri) {
                            reads_of[pi].push(ri);
                        }
                    }
                    None => errors.push(GraphError::UnknownResource {
                        pass: p.name.clone(),
                        resource: name.clone(),
                    }),
                }
            }
            for name in &p.writes {
                match res_idx.get(name.as_str()) {
                    Some(&ri) => {
                        if !writes_of[pi].contains(&ri) {
                            writes_of[pi].push(ri);
                        }
                        if self.resources[ri].external {
                            errors.push(GraphError::WriteToExternal {
                                pass: p.name.clone(),
                                resource: name.clone(),
                            });
                        }
                    }
                    None => errors.push(GraphError::UnknownResource {
                        pass: p.name.clone(),
                        resource: name.clone(),
                    }),
                }
            }
        }

        // ---- per-resource writer/reader tables (declaration order)
        let nr = self.resources.len();
        let mut writers: Vec<Vec<usize>> = vec![Vec::new(); nr];
        let mut readers: Vec<Vec<usize>> = vec![Vec::new(); nr];
        for pi in 0..np {
            for &ri in &writes_of[pi] {
                writers[ri].push(pi);
            }
            for &ri in &reads_of[pi] {
                readers[ri].push(pi);
            }
        }

        // ---- version binding -> dependency edges. Each read binds
        // to the nearest writer declared before the reader (never
        // to the reader itself), else to the earliest writer after
        // it (a forward reference). A read with no bindable writer
        // — no writers at all, or only itself (a lone RMW) — is a
        // read-of-never-written error; external inputs bind to
        // nothing and are always readable.
        let mut adj: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); np];
        for ri in 0..nr {
            if self.resources[ri].external {
                continue;
            }
            for &rd in &readers[ri] {
                let bound = writers[ri]
                    .iter()
                    .copied()
                    .filter(|&w| w != rd && w < rd)
                    .max()
                    .or_else(|| writers[ri].iter().copied().filter(|&w| w != rd && w > rd).min());
                match bound {
                    Some(w) => {
                        adj[w].insert(rd);
                    }
                    None => errors.push(GraphError::ReadNeverWritten {
                        pass: self.passes[rd].name.clone(),
                        resource: self.resources[ri].name.clone(),
                    }),
                }
            }
        }
        let mut edges: Vec<(usize, usize)> = Vec::new();
        for (a, outs) in adj.iter().enumerate() {
            for &b in outs {
                edges.push((a, b));
            }
        }

        // ---- deterministic topological order (Kahn; among ready
        // passes the lowest declaration index wins — see module docs)
        let mut indeg: Vec<usize> = vec![0; np];
        for outs in &adj {
            for &b in outs {
                indeg[b] += 1;
            }
        }
        let mut order: Vec<usize> = Vec::with_capacity(np);
        let mut scheduled = vec![false; np];
        loop {
            let next = (0..np).find(|&i| !scheduled[i] && indeg[i] == 0);
            match next {
                Some(i) => {
                    scheduled[i] = true;
                    order.push(i);
                    for &b in &adj[i] {
                        indeg[b] -= 1;
                    }
                }
                None => break,
            }
        }
        if order.len() < np {
            let stuck: Vec<String> = (0..np)
                .filter(|&i| !scheduled[i])
                .map(|i| self.passes[i].name.clone())
                .collect();
            errors.push(GraphError::Cycle { passes: stuck });
        }

        // ---- reachability (transitive closure over RAW edges)
        let reach: Vec<BTreeSet<usize>> = (0..np)
            .map(|src| {
                let mut seen = BTreeSet::new();
                let mut stack: Vec<usize> = adj[src].iter().copied().collect();
                while let Some(n) = stack.pop() {
                    if seen.insert(n) {
                        stack.extend(adj[n].iter().copied());
                    }
                }
                seen
            })
            .collect();
        let ordered = |a: usize, b: usize| reach[a].contains(&b) || reach[b].contains(&a);

        // ---- write-write conflicts and ambiguous reads
        for ri in 0..nr {
            let rname = self.resources[ri].name.clone();
            let ws = &writers[ri];
            for i in 0..ws.len() {
                for j in (i + 1)..ws.len() {
                    if !ordered(ws[i], ws[j]) {
                        errors.push(GraphError::WriteConflict {
                            resource: rname.clone(),
                            first: self.passes[ws[i]].name.clone(),
                            second: self.passes[ws[j]].name.clone(),
                        });
                    }
                }
            }
            if !self.resources[ri].external {
                for &rd in &readers[ri] {
                    for &w in &writers[ri] {
                        if w != rd && !ordered(w, rd) {
                            errors.push(GraphError::AmbiguousRead {
                                resource: rname.clone(),
                                reader: self.passes[rd].name.clone(),
                                writer: self.passes[w].name.clone(),
                            });
                        }
                    }
                }
            }
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        // ---- positions + lifetimes
        let mut pos = vec![0usize; np];
        for (k, &pi) in order.iter().enumerate() {
            pos[pi] = k;
        }
        let mut lifetimes: Vec<Lifetime> = Vec::new();
        for ri in 0..nr {
            let mut uses: Vec<usize> = Vec::new();
            for &pi in writers[ri].iter().chain(readers[ri].iter()) {
                uses.push(pos[pi]);
            }
            if uses.is_empty() {
                continue;
            }
            let writer = writers[ri].iter().min_by_key(|&&pi| pos[pi]).copied();
            lifetimes.push(Lifetime {
                resource: self.resources[ri].name.clone(),
                first_use: *uses.iter().min().unwrap(),
                last_use: *uses.iter().max().unwrap(),
                writer,
            });
        }

        // ---- warnings (declaration order)
        let mut warnings: Vec<GraphWarning> = Vec::new();
        for ri in 0..nr {
            let r = &self.resources[ri];
            if r.external {
                if readers[ri].is_empty() {
                    warnings.push(GraphWarning::UnusedExternal { resource: r.name.clone() });
                }
            } else if !r.sink && !writers[ri].is_empty() && readers[ri].is_empty() {
                warnings.push(GraphWarning::UnusedWrite {
                    resource: r.name.clone(),
                    writer: self.passes[writers[ri][0]].name.clone(),
                });
            }
        }

        Ok(Graph {
            resources: self.resources,
            passes: self.passes,
            order,
            lifetimes,
            warnings,
            edges,
        })
    }
}

// ---------------------------------------------------------------------
// The real chain, transcribed.
// ---------------------------------------------------------------------

/// The production composer chain as actually executed by
/// `ab1.rs::run_ab1` at post stage 3 (the full chain), with the
/// bloom cascade expanded from `composer.rs::PostChain::bloom`.
/// Pass order below IS the source encoder order; resource names are
/// the `wgpu::TextureDescriptor` labels in source, snakified:
///
///   pass (source)                       reads -> writes
///   1. shadow (ab1.rs "shadow pass")    scene_data -> "key shadow map"
///      Rgba8Unorm 2048x2048 + "shadow depth" Depth32Float 2048x2048
///   2. depth_prepass (ab1.rs step 2)    scene_data -> "packed depth"
///      (PackedDepth class), "prepass depth" Depth32Float
///   3. scene (ab1.rs "ab1 scene pass")  scene_data, scene_textures
///      (painted canvas + sprite/ground textures), room_env (the
///      captured PMREM, Rgba16Float 768x1024), shadow_map ->
///      "hdr msaa" Rgba16Float MSAA-4x resolved into "hdr resolved",
///      "hdr depth" Depth32Float MSAA-4x
///   4. bokeh (ab1.rs step 4, shader_bokeh.wgsl)
///      hdr_resolved + packed_depth -> "bokeh out" Rgba16Float
///   5. bloom_bright (PostChain::bloom)  bokeh_out -> bright target
///      at BloomMip(0)
///   6..15. bloom_blur_hN / bloom_blur_vN (N = 0..4)
///      separable cascade: H0 reads bright, V[N] reads H[N],
///      H[N+1] reads V[N]; all Rgba16Float at BloomMip(N)
///  16. bloom_composite                 bloom_v0..v4 -> bloom_h0
///      (source really does composite INTO the horiz[0] texture)
///  17. bloom_add (CopyShader + additive blend, LoadOp::Load)
///      bloom_h0 + bokeh_out -> bokeh_out  (read-modify-write)
///  18. output (PostChain::output, OutputPass)
///      bokeh_out -> "output srgb" Rgba8Unorm
///  19. grade (PostChain::grade, GRADE)  output_srgb -> "final"
///      Rgba8Unorm
///  20. caption (ab1.rs step 8, LoadOp::Load overlay)
///      caption_strip + final -> final     (read-modify-write)
///  21. readback (copy_texture_to_buffer) final -> png_bytes (sink)
///
/// At post stage 2 (no GRADE) `output` writes `final` directly and
/// passes 19 is absent; at stage 1 passes 5..19 are absent. The
/// fixture is stage 3, the production configuration.
pub fn ab1_full_chain() -> GraphBuilder {
    let mut b = GraphBuilder::new();

    // Declared inputs.
    b.add_resource(ResourceDesc::host_input("scene_data"));
    b.add_resource(ResourceDesc::external_texture("scene_textures", TexFormat::Rgba8Unorm, SizeClass::Native));
    b.add_resource(ResourceDesc::external_texture("room_env", TexFormat::Rgba16Float, SizeClass::Fixed(768, 1024)));
    b.add_resource(ResourceDesc::external_texture("caption_strip", TexFormat::Rgba8Unorm, SizeClass::Native));

    // Graph-produced resources.
    b.add_resource(ResourceDesc::texture("shadow_map", TexFormat::Rgba8Unorm, SizeClass::Fixed(2048, 2048)));
    b.add_resource(ResourceDesc::texture("shadow_depth", TexFormat::Depth32Float, SizeClass::Fixed(2048, 2048)));
    b.add_resource(ResourceDesc::texture("packed_depth", TexFormat::PackedDepth, SizeClass::FullFrame));
    b.add_resource(ResourceDesc::texture("prepass_depth", TexFormat::Depth32Float, SizeClass::FullFrame));
    b.add_resource(ResourceDesc::texture("hdr_msaa", TexFormat::Rgba16Float, SizeClass::FullFrame));
    b.add_resource(ResourceDesc::texture("hdr_resolved", TexFormat::Rgba16Float, SizeClass::FullFrame));
    b.add_resource(ResourceDesc::texture("hdr_depth", TexFormat::Depth32Float, SizeClass::FullFrame));
    b.add_resource(ResourceDesc::texture("bokeh_out", TexFormat::Rgba16Float, SizeClass::FullFrame));
    b.add_resource(ResourceDesc::texture("bloom_bright", TexFormat::Rgba16Float, SizeClass::BloomMip(0)));
    for n in 0..5 {
        b.add_resource(ResourceDesc::texture(&format!("bloom_h{n}"), TexFormat::Rgba16Float, SizeClass::BloomMip(n)));
        b.add_resource(ResourceDesc::texture(&format!("bloom_v{n}"), TexFormat::Rgba16Float, SizeClass::BloomMip(n)));
    }
    b.add_resource(ResourceDesc::texture("output_srgb", TexFormat::Rgba8Unorm, SizeClass::FullFrame));
    b.add_resource(ResourceDesc::texture("final", TexFormat::Rgba8Unorm, SizeClass::FullFrame));
    b.add_resource(ResourceDesc::host_sink("png_bytes"));

    // Passes, in source encoder order.
    b.add_pass(PassDesc::new("shadow").reads(&["scene_data"]).writes(&["shadow_map", "shadow_depth"]));
    b.add_pass(PassDesc::new("depth_prepass").reads(&["scene_data"]).writes(&["packed_depth", "prepass_depth"]));
    b.add_pass(
        PassDesc::new("scene")
            .reads(&["scene_data", "scene_textures", "room_env", "shadow_map"])
            .writes(&["hdr_msaa", "hdr_resolved", "hdr_depth"]),
    );
    b.add_pass(PassDesc::new("bokeh").reads(&["hdr_resolved", "packed_depth"]).writes(&["bokeh_out"]));
    b.add_pass(PassDesc::new("bloom_bright").reads(&["bokeh_out"]).writes(&["bloom_bright"]));
    b.add_pass(PassDesc::new("bloom_blur_h0").reads(&["bloom_bright"]).writes(&["bloom_h0"]));
    for n in 0..5 {
        b.add_pass(
            PassDesc::new(&format!("bloom_blur_v{n}"))
                .reads(&[&format!("bloom_h{n}")])
                .writes(&[&format!("bloom_v{n}")]),
        );
        if n < 4 {
            b.add_pass(
                PassDesc::new(&format!("bloom_blur_h{}", n + 1))
                    .reads(&[&format!("bloom_v{n}")])
                    .writes(&[&format!("bloom_h{}", n + 1)]),
            );
        }
    }
    b.add_pass(
        PassDesc::new("bloom_composite")
            .reads(&["bloom_v0", "bloom_v1", "bloom_v2", "bloom_v3", "bloom_v4"])
            .writes(&["bloom_h0"]),
    );
    b.add_pass(PassDesc::new("bloom_add").reads(&["bloom_h0", "bokeh_out"]).writes(&["bokeh_out"]));
    b.add_pass(PassDesc::new("output").reads(&["bokeh_out"]).writes(&["output_srgb"]));
    b.add_pass(PassDesc::new("grade").reads(&["output_srgb"]).writes(&["final"]));
    b.add_pass(PassDesc::new("caption").reads(&["caption_strip", "final"]).writes(&["final"]));
    b.add_pass(PassDesc::new("readback").reads(&["final"]).writes(&["png_bytes"]));

    b
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_err(b: GraphBuilder) -> Vec<GraphError> {
        match b.build() {
            Ok(_) => panic!("expected validation errors, graph built cleanly"),
            Err(e) => e,
        }
    }

    // (1) The real chain validates, and the derived order reproduces
    // the source encoder order exactly (the declaration is in source
    // order and the chain's data flow forces the rest).
    #[test]
    fn composer_chain_validates_and_orders_like_source() {
        let g = ab1_full_chain().build().expect("the real ab1 chain must validate");
        assert_eq!(
            g.order_names(),
            vec![
                "shadow",
                "depth_prepass",
                "scene",
                "bokeh",
                "bloom_bright",
                "bloom_blur_h0",
                "bloom_blur_v0",
                "bloom_blur_h1",
                "bloom_blur_v1",
                "bloom_blur_h2",
                "bloom_blur_v2",
                "bloom_blur_h3",
                "bloom_blur_v3",
                "bloom_blur_h4",
                "bloom_blur_v4",
                "bloom_composite",
                "bloom_add",
                "output",
                "grade",
                "caption",
                "readback",
            ]
        );
        // Key data-flow edges exist.
        let edges = g.edges();
        for e in [
            ("shadow", "scene"),
            ("depth_prepass", "bokeh"),
            ("scene", "bokeh"),
            ("bokeh", "bloom_add"),
            ("bloom_composite", "bloom_add"),
            ("bloom_add", "output"),
            ("grade", "caption"),
            ("caption", "readback"),
        ] {
            assert!(edges.contains(&e), "missing edge {e:?}");
        }
        // The only unused writes are the in-pass-consumed depth
        // attachments and the MSAA colour buffer — exactly the four
        // the module docs predict.
        let unused: Vec<&str> = g
            .warnings()
            .iter()
            .filter_map(|w| match w {
                GraphWarning::UnusedWrite { resource, .. } => Some(resource.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(unused, vec!["shadow_depth", "prepass_depth", "hdr_msaa", "hdr_depth"]);
    }

    // (2) Lifetime table on the fixture: intervals span exactly the
    // passes that produce and consume each resource.
    #[test]
    fn composer_chain_lifetimes() {
        let g = ab1_full_chain().build().unwrap();
        let lt = |n: &str| g.lifetime_of(n).unwrap();
        // Positions follow the order asserted above.
        assert_eq!((lt("shadow_map").first_use, lt("shadow_map").last_use), (0, 2));
        assert_eq!((lt("packed_depth").first_use, lt("packed_depth").last_use), (1, 3));
        assert_eq!((lt("hdr_resolved").first_use, lt("hdr_resolved").last_use), (2, 3));
        // bokeh_out: written by bokeh (3), read-modify-written by
        // bloom_add (16), last read by output (17).
        assert_eq!((lt("bokeh_out").first_use, lt("bokeh_out").last_use), (3, 17));
        // bloom_h0: first written by blur H0 (5), re-written by the
        // composite (15), last read by bloom_add (16).
        assert_eq!((lt("bloom_h0").first_use, lt("bloom_h0").last_use), (5, 16));
        assert_eq!(lt("bloom_h0").writer, Some(5)); // blur_h0's pass index
        assert_eq!((lt("bloom_v4").first_use, lt("bloom_v4").last_use), (14, 15));
        // final: produced by grade (18), overlaid by caption (19),
        // consumed by readback (20).
        assert_eq!((lt("final").first_use, lt("final").last_use), (18, 20));
        // External inputs have no writer; room_env is used by the
        // scene pass alone.
        assert_eq!(lt("room_env").writer, None);
        assert_eq!((lt("room_env").first_use, lt("room_env").last_use), (2, 2));
    }

    // (3) A planted cycle is detected and names the stuck passes.
    #[test]
    fn planted_cycle_is_detected() {
        let mut b = GraphBuilder::new();
        b.add_resource(ResourceDesc::texture("x", TexFormat::Rgba16Float, SizeClass::FullFrame));
        b.add_resource(ResourceDesc::texture("y", TexFormat::Rgba16Float, SizeClass::FullFrame));
        b.add_pass(PassDesc::new("a").reads(&["y"]).writes(&["x"]));
        b.add_pass(PassDesc::new("b").reads(&["x"]).writes(&["y"]));
        let errs = build_err(b);
        assert!(
            errs.contains(&GraphError::Cycle { passes: vec!["a".to_string(), "b".to_string()] }),
            "expected a Cycle error, got {errs:?}"
        );
    }

    // (4) A planted write-write conflict: two passes write the same
    // resource with nothing ordering them.
    #[test]
    fn planted_write_conflict_is_detected() {
        let mut b = GraphBuilder::new();
        b.add_resource(ResourceDesc::texture("r", TexFormat::Rgba8Unorm, SizeClass::FullFrame));
        b.add_pass(PassDesc::new("p").writes(&["r"]));
        b.add_pass(PassDesc::new("q").writes(&["r"]));
        b.add_pass(PassDesc::new("sink_pass").reads(&["r"]).writes(&[]));
        let errs = build_err(b);
        assert!(
            errs.contains(&GraphError::WriteConflict {
                resource: "r".to_string(),
                first: "p".to_string(),
                second: "q".to_string(),
            }),
            "expected a WriteConflict, got {errs:?}"
        );
    }

    // (5) The same double-write IS legal when data flow orders the
    // writers — the bloom_composite-into-horiz[0] pattern in
    // miniature: write, read onward, rewrite, read.
    #[test]
    fn ordered_double_write_is_legal() {
        let mut b = GraphBuilder::new();
        b.add_resource(ResourceDesc::host_input("input"));
        b.add_resource(ResourceDesc::texture("r", TexFormat::Rgba16Float, SizeClass::FullFrame));
        b.add_resource(ResourceDesc::texture("s", TexFormat::Rgba16Float, SizeClass::FullFrame));
        b.add_pass(PassDesc::new("first").reads(&["input"]).writes(&["r"]));
        b.add_pass(PassDesc::new("middle").reads(&["r"]).writes(&["s"]));
        b.add_pass(PassDesc::new("rewrite").reads(&["s", "r"]).writes(&["r"]));
        let g = b.build().expect("ordered double-write must validate");
        assert_eq!(g.order_names(), vec!["first", "middle", "rewrite"]);
    }

    // (6) The structural error classes: unknown resource, read of
    // never-written, write to an external input.
    #[test]
    fn structural_errors_are_reported() {
        let mut b = GraphBuilder::new();
        b.add_resource(ResourceDesc::external_texture("ext", TexFormat::Rgba8Unorm, SizeClass::Native));
        b.add_resource(ResourceDesc::texture("never", TexFormat::Rgba8Unorm, SizeClass::FullFrame));
        b.add_pass(PassDesc::new("bad").reads(&["never", "ghost"]).writes(&["ext"]));
        let errs = build_err(b);
        assert!(errs.contains(&GraphError::UnknownResource {
            pass: "bad".to_string(),
            resource: "ghost".to_string(),
        }));
        assert!(errs.contains(&GraphError::ReadNeverWritten {
            pass: "bad".to_string(),
            resource: "never".to_string(),
        }));
        assert!(errs.contains(&GraphError::WriteToExternal {
            pass: "bad".to_string(),
            resource: "ext".to_string(),
        }));
    }

    // (7) Topological order is a function of dependencies, not of
    // declaration order alone — a consumer declared before its
    // producer is still scheduled after it; and among independent
    // ready passes the declaration order breaks ties (stability).
    #[test]
    fn topo_order_reorders_and_tie_breaks_by_declaration() {
        let mut b = GraphBuilder::new();
        b.add_resource(ResourceDesc::host_input("input"));
        b.add_resource(ResourceDesc::texture("x", TexFormat::Rgba16Float, SizeClass::FullFrame));
        b.add_pass(PassDesc::new("consumer").reads(&["x"]));
        b.add_pass(PassDesc::new("independent").reads(&["input"]));
        b.add_pass(PassDesc::new("producer").reads(&["input"]).writes(&["x"]));
        let g = b.build().unwrap();
        // Ready first: independent (declared 2nd) and producer (3rd);
        // declaration order wins the tie; consumer must follow its
        // producer despite being declared first.
        assert_eq!(g.order_names(), vec!["independent", "producer", "consumer"]);
    }

    // (8) An RMW pass with no prior writer is a read of the void.
    #[test]
    fn rmw_without_prior_writer_is_an_error() {
        let mut b = GraphBuilder::new();
        b.add_resource(ResourceDesc::texture("r", TexFormat::Rgba8Unorm, SizeClass::FullFrame));
        b.add_pass(PassDesc::new("only").reads(&["r"]).writes(&["r"]));
        let errs = build_err(b);
        assert!(errs.contains(&GraphError::ReadNeverWritten {
            pass: "only".to_string(),
            resource: "r".to_string(),
        }));
    }
}
