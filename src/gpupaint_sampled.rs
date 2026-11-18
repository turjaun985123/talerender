//! GPUFIRST Batch 3 — SAMPLED seat measurement harness
//! (`--gpu-paint sampled-profile <lists_outdir>`, feature
//! `gpupaint` only; the default build does not contain this
//! module and production behaviour is unchanged).
//!
//! What it measures, and how (read before quoting any line):
//!
//! 1. STAGED profile (Profile A), exact: the Onoda film's
//!    per-frame character-paint REQUEST sequence is regenerated
//!    by calling the production staging entry
//!    `film::eval_frame` at t = n/30 for every frame n, and
//!    keying every character with a verbatim copy of
//!    `film::state_key` (film.rs's own key is private; the copy
//!    below is validated by reproducing the census paint-cache
//!    totals — see the VALIDATION lines). The requests are fed
//!    through a simulator that replicates `film::PaintCache`
//!    exactly: same LRU order discipline, the REAL
//!    `paintbudget::PaintBudget` tracker, and the constant
//!    entry size `paintbudget::CANVAS_BYTES` (every painted
//!    card is 2560x2816 RGBA, so entry bytes never vary and
//!    the simulation is exact, not sampled). Chunked runs
//!    (fresh cache per census chunk) validate against the
//!    measured census lines; a single-process run then gives
//!    the exact distinct-state count D and miss count M1 that
//!    GPUFIRST-FREQ §1.5 could only bound (562 <= D <= 2612).
//!
//! 2. SAMPLED profile (Profile B), constructed: segment s02 of
//!    the same film (frames 178..392, 214 frames, 3 walking
//!    characters, 642 requests — staging in film.rs `s02`) is
//!    re-authored the way a per-frame-sampled project authors
//!    it: every scalar character channel is baked to a pose
//!    key at EVERY frame (segment-local lt = i/30, i = frame
//!    index in segment, mirroring filmseq's lt law), and the
//!    per-frame state is then folded back through the real
//!    clip sampler `eval15::sample_scalar_keys`, exactly as
//!    `scenefile` folds a sampled scene at its `time`.
//!    Variant A bakes the staged f32 values (f64-widened);
//!    variant B bakes decimal-rounded values (6 dp), the way
//!    an external baker writing JSON pose keys does. Step
//!    channels (mood, legs) and bools (tears) are carried per
//!    frame, as step tracks fold. The generic (scenefile)
//!    path that would consume such a project has NO
//!    PaintCache (scenefile paints unconditionally), so its
//!    as-built cache line is hits=0 / paints=requests by
//!    construction; a cached counterfactual is also run
//!    through the same simulator for comparison.
//!
//! 3. CPU paint wall time for the s02 segment: real
//!    `paint_toon_full` timings (sum over the sequence, cache
//!    on/off), on the CI runner CPU. GPU-side per-state wall
//!    time is NOT measured here: this harness records display
//!    lists (staged + sampled-B, 6 states each) into
//!    <lists_outdir> and the workflow replays them with the
//!    existing `--gpu-paint replay` executor on lavapipe.
//!
//! Output grammar: lines prefixed `SAMPLED-PROFILE`.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;
use std::time::Instant;

use crate::toonfull::{FullState, Legs};

// ---------------------------------------------------------------------
// Verbatim copy of film::state_key (film.rs, private). Copied so
// this harness can key requests without touching film wiring;
// validated end-to-end by the census reproduction below — if the
// copy drifted from the original in any field, the chunked
// hits/misses/evictions would diverge from the measured census
// lines and the VALIDATION line would print MISMATCH.
// ---------------------------------------------------------------------
fn probe_state_key(look: usize, st: &FullState, hold_l: Option<&str>, hold_r: Option<&str>) -> String {
    let q = |v: f32| v.to_bits();
    let vec_key = |v: &[f32]| -> String {
        let mut s = format!("{}", v.len());
        for x in v {
            s.push(':');
            s.push_str(&x.to_bits().to_string());
        }
        s
    };
    let opt_key = |v: Option<f32>| -> String {
        match v {
            None => "0:0".to_string(),
            Some(x) => format!("1:{}", x.to_bits()),
        }
    };
    format!(
        "{look}|{:?}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        st.mood,
        q(st.look[0]), q(st.look[1]), q(st.blink), q(st.talk),
        q(st.arm_l.0), q(st.arm_l.1), q(st.arm_r.0), q(st.arm_r.1),
        q(st.phase), q(st.lean), q(st.bob), q(st.squash),
        match st.legs { Legs::Stand => "stand", Legs::Walk => "walk", Legs::Run => "run", Legs::Sit => "sit", Legs::Jump => "jump" },
        q(st.head_tilt), q(st.head_turn), q(st.cam_pitch),
        st.sit_drop.map(q).map(|v| v as i64).unwrap_or(-1),
        hold_l.unwrap_or("-"),
        hold_r.unwrap_or("-"),
        st.sweat.is_some(), st.sweat.map(|v| v.to_bits()).unwrap_or(0), st.tears,
        vec_key(&st.wing_segs), q(st.wing_a), q(st.ear_a),
        opt_key(st.ear_al), opt_key(st.ear_ar),
        vec_key(&st.hair_segs), st.flip,
        q(st.body_pitch), q(st.body_pitch_counter),
        q(st.brow_lift), q(st.brow_tilt),
    )
}

// ---------------------------------------------------------------------
// PaintCache simulator: replicates film::PaintCache::get_or_paint's
// hit/miss/LRU/eviction discipline with the real PaintBudget
// tracker. Entry bytes are the constant CANVAS_BYTES, so byte
// accounting is exact. Painting itself is NOT simulated here;
// the caller paints on miss when it wants wall times.
// ---------------------------------------------------------------------
struct CacheSim {
    map: HashMap<String, ()>,
    order: VecDeque<String>,
    budget: crate::paintbudget::PaintBudget,
}

impl CacheSim {
    fn new() -> CacheSim {
        let cap = crate::paintbudget::DEFAULT_ENTRY_CAP;
        CacheSim {
            map: HashMap::new(),
            order: VecDeque::new(),
            budget: crate::paintbudget::PaintBudget::new(
                crate::paintbudget::default_budget_bytes(cap, crate::paintbudget::CANVAS_BYTES),
                cap,
            ),
        }
    }
    /// Returns true on a hit (no paint needed).
    fn request(&mut self, key: &str) -> bool {
        if self.map.contains_key(key) {
            self.budget.on_hit();
            if let Some(pos) = self.order.iter().position(|k| k == key) {
                let k = self.order.remove(pos).expect("cache order");
                self.order.push_back(k);
            }
            return true;
        }
        self.budget.on_miss(key);
        self.budget.on_insert(crate::paintbudget::CANVAS_BYTES);
        self.map.insert(key.to_string(), ());
        self.order.push_back(key.to_string());
        while let Some(reason) = self.budget.eviction_reason() {
            let old = self.order.pop_front().expect("cache order");
            self.map.remove(&old).expect("cache order");
            self.budget.on_evict(&old, crate::paintbudget::CANVAS_BYTES, reason);
        }
        false
    }
    fn stats_line(&self) -> String {
        self.budget.stats_line()
    }
}

// ---------------------------------------------------------------------
// Request sequences.
// ---------------------------------------------------------------------

struct Req {
    look: usize,
    state: FullState,
    hold_l: Option<&'static str>,
    hold_r: Option<&'static str>,
}

fn key_of(r: &Req) -> String {
    probe_state_key(r.look, &r.state, r.hold_l, r.hold_r)
}

/// The staged request sequence for frames [from, to) of the
/// Onoda film, via the production staging entry point.
fn staged_requests(
    data: &crate::film::FilmData,
    looks: &[(crate::toonfull::FullLook, bool, f64)],
    from: u32,
    to: u32,
) -> Vec<Req> {
    let mut out = Vec::new();
    for n in from..to {
        let spec = crate::film::eval_frame(data, looks, n as f64 / 30.0);
        for c in &spec.chars {
            out.push(Req { look: c.look, state: copy_state(&c.state), hold_l: c.hold_l, hold_r: c.hold_r });
        }
    }
    out
}

/// Field-by-field copy of a FullState with held plates dropped
/// (the s02/s03 segments hold no plates — asserted by callers;
/// the paint key names holds separately, and resolve_chars
/// resolves them from the same names this harness keeps).
fn copy_state(st: &FullState) -> FullState {
    FullState {
        mood: st.mood,
        look: st.look,
        blink: st.blink,
        talk: st.talk,
        brow_lift: st.brow_lift,
        brow_tilt: st.brow_tilt,
        sweat: st.sweat,
        tears: st.tears,
        arm_l: st.arm_l,
        arm_r: st.arm_r,
        legs: st.legs,
        phase: st.phase,
        lean: st.lean,
        bob: st.bob,
        squash: st.squash,
        head_tilt: st.head_tilt,
        head_turn: st.head_turn,
        cam_pitch: st.cam_pitch,
        sit_drop: st.sit_drop,
        hold_l: None,
        hold_r: None,
        wing_segs: st.wing_segs.clone(),
        wing_a: st.wing_a,
        ear_a: st.ear_a,
        ear_al: st.ear_al,
        ear_ar: st.ear_ar,
        hair_segs: st.hair_segs.clone(),
        flip: st.flip,
        body_pitch: st.body_pitch,
        body_pitch_counter: st.body_pitch_counter,
    }
}

// Scalar channels baked per frame in the sampled workload, in a
// fixed order (indices into the channel table).
const NCH: usize = 17;
fn channels_of(st: &FullState) -> [f64; NCH] {
    [
        st.look[0] as f64, st.look[1] as f64, st.blink as f64, st.talk as f64,
        st.brow_lift as f64, st.brow_tilt as f64,
        st.arm_l.0 as f64, st.arm_l.1 as f64, st.arm_r.0 as f64, st.arm_r.1 as f64,
        st.phase as f64, st.lean as f64, st.bob as f64, st.squash as f64,
        st.head_tilt as f64, st.head_turn as f64, st.cam_pitch as f64,
    ]
}
fn state_with_channels(base: &FullState, ch: &[f64; NCH]) -> FullState {
    let mut st = copy_state(base);
    st.look = [ch[0] as f32, ch[1] as f32];
    st.blink = ch[2] as f32;
    st.talk = ch[3] as f32;
    st.brow_lift = ch[4] as f32;
    st.brow_tilt = ch[5] as f32;
    st.arm_l = (ch[6] as f32, ch[7] as f32);
    st.arm_r = (ch[8] as f32, ch[9] as f32);
    st.phase = ch[10] as f32;
    st.lean = ch[11] as f32;
    st.bob = ch[12] as f32;
    st.squash = ch[13] as f32;
    st.head_tilt = ch[14] as f32;
    st.head_turn = ch[15] as f32;
    st.cam_pitch = ch[16] as f32;
    st
}

/// Bake per-frame keys for one character slot from the staged
/// sequence and fold them back at each frame's segment-local
/// lt, through eval15's real clip sampler. `decimal6` selects
/// variant B (values rounded to 6 decimal places at bake time,
/// as an external JSON baker would write them).
fn sampled_fold(staged: &[Req], slot: usize, nslots: usize, decimal6: bool) -> Vec<Req> {
    let nframes = staged.len() / nslots;
    // keys[slot channel] = per-frame ScalarKeys at lt = i/30.
    let mut keys: Vec<Vec<crate::eval15::ScalarKey>> = Vec::new();
    for c in 0..NCH {
        let mut ks = Vec::with_capacity(nframes);
        for i in 0..nframes {
            let st = &staged[i * nslots + slot].state;
            let mut v = channels_of(st)[c];
            if decimal6 {
                v = (v * 1e6).round() / 1e6;
            }
            ks.push(crate::eval15::ScalarKey { t: i as f64 / 30.0, value: v, ease: "linear".to_string() });
        }
        keys.push(ks);
    }
    let mut out = Vec::with_capacity(nframes);
    for i in 0..nframes {
        let lt = i as f64 / 30.0;
        let mut ch = [0.0f64; NCH];
        for c in 0..NCH {
            ch[c] = crate::eval15::sample_scalar_keys(&keys[c], lt);
        }
        let base = &staged[i * nslots + slot];
        out.push(Req { look: base.look, state: state_with_channels(&base.state, &ch), hold_l: base.hold_l, hold_r: base.hold_r });
    }
    out
}

fn distinct_count(reqs: &[Req]) -> usize {
    let mut set: HashSet<String> = HashSet::new();
    for r in reqs {
        set.insert(key_of(r));
    }
    set.len()
}

fn run_sim(reqs: &[Req]) -> (CacheSim, u64, u64) {
    let mut sim = CacheSim::new();
    let (mut hits, mut misses) = (0u64, 0u64);
    for r in reqs {
        if sim.request(&key_of(r)) {
            hits += 1;
        } else {
            misses += 1;
        }
    }
    (sim, hits, misses)
}

fn median_ms(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    if v.is_empty() { 0.0 } else { v[v.len() / 2] }
}

/// Time real CPU paints: every request painted unconditionally
/// (as-built generic path), and the same sequence with the
/// cache simulator deciding (paint only on miss).
fn time_paints(looks: &[(crate::toonfull::FullLook, bool, f64)], reqs: &[Req]) -> (f64, f64, Vec<f64>) {
    let mut per_paint: Vec<f64> = Vec::new();
    let t0 = Instant::now();
    for r in reqs {
        let tp = Instant::now();
        let _ = crate::toonfull::paint_toon_full(&looks[r.look].0, &r.state);
        per_paint.push(tp.elapsed().as_secs_f64() * 1000.0);
    }
    let uncached_ms = t0.elapsed().as_secs_f64() * 1000.0;
    let mut sim = CacheSim::new();
    let t0 = Instant::now();
    for r in reqs {
        if !sim.request(&key_of(r)) {
            let _ = crate::toonfull::paint_toon_full(&looks[r.look].0, &r.state);
        }
    }
    let cached_ms = t0.elapsed().as_secs_f64() * 1000.0;
    (uncached_ms, cached_ms, per_paint)
}

fn record_list(looks: &[(crate::toonfull::FullLook, bool, f64)], r: &Req, dir: &str, name: &str) {
    crate::gputrace::begin(2560, 2816);
    let _ = crate::toonfull::paint_toon_full(&looks[r.look].0, &r.state);
    let list = crate::gputrace::take();
    let path = format!("{dir}/{name}.json");
    match std::fs::write(&path, list.to_json()) {
        Ok(()) => println!("SAMPLED-PROFILE LIST wrote={} prims={}", path, list.prims.len()),
        Err(e) => eprintln!("SAMPLED-PROFILE LIST error writing {path}: {e}"),
    }
}

pub(crate) fn run(lists_dir: &str) -> i32 {
    println!("SAMPLED-PROFILE BEGIN head=01e3ea8 workload=onoda-s02-sampled + full-film-staged");
    let data = match crate::film::load_film_data(Path::new("film")) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("SAMPLED-PROFILE ERROR load_film_data(film): {e}");
            return 2;
        }
    };
    let looks = crate::film::look_registry();

    // ---- 1. Staged full film: chunked validation + single-process ----
    let chunks: [(u32, u32); 8] = [(0, 250), (250, 500), (500, 750), (750, 1000), (1000, 1250), (1250, 1500), (1500, 1750), (1750, 1811)];
    // Measured census totals (GPUFIRST-FREQ §1.2, runs 43867793062
    // + 43808791603, identical in both): hits 1000, misses 2612,
    // evictions 2042 (all byte), peak_bytes 2162688000 per full chunk.
    let (mut tot_hits, mut tot_misses, mut tot_evict) = (0u64, 0u64, 0u64);
    let mut global_distinct: HashSet<String> = HashSet::new();
    for (from, to) in chunks {
        let reqs = staged_requests(&data, &looks, from, to);
        for r in &reqs {
            global_distinct.insert(key_of(r));
        }
        let (sim, hits, misses) = run_sim(&reqs);
        tot_hits += hits;
        tot_misses += misses;
        tot_evict += sim.budget.evictions();
        println!(
            "SAMPLED-PROFILE STAGED-CHUNK frames={}..{} requests={} {}",
            from, to, reqs.len(), sim.stats_line()
        );
    }
    println!(
        "SAMPLED-PROFILE STAGED-CHUNKED-TOTAL hits={} misses={} evictions={} requests={}",
        tot_hits, tot_misses, tot_evict, tot_hits + tot_misses
    );
    let validation = if tot_hits == 1000 && tot_misses == 2612 && tot_evict == 2042 {
        "MATCH (census runs 43867793062/43808791603 via GPUFIRST-FREQ §1.2)"
    } else {
        "MISMATCH vs census (1000/2612/2042) — key copy or staging sequence drifted; do not quote the sampled numbers"
    };
    println!("SAMPLED-PROFILE VALIDATION staged-chunked-vs-census: {validation}");

    let all = staged_requests(&data, &looks, 0, 1811);
    let (sim1, h1, m1) = run_sim(&all);
    println!(
        "SAMPLED-PROFILE STAGED-SINGLEPROCESS frames=1811 requests={} distinct_states={} {}",
        all.len(), global_distinct.len(), sim1.stats_line()
    );
    let _ = (h1, m1);

    // ---- 2. s02 segment: staged vs sampled ----
    let seg = staged_requests(&data, &looks, 178, 392);
    let nslots = 3usize;
    assert!(seg.len() == 214 * nslots, "s02 must stage 3 chars x 214 frames, got {}", seg.len());
    assert!(seg.iter().all(|r| r.hold_l.is_none() && r.hold_r.is_none()), "s02 holds no plates");
    assert!(seg.iter().all(|r| r.state.sweat.is_none() && r.state.sit_drop.is_none()), "s02 sweat/sit_drop are None throughout (not baked channels)");
    println!(
        "SAMPLED-PROFILE S02-STAGED frames=214 chars=3 requests={} distinct_states={}",
        seg.len(), distinct_count(&seg)
    );
    let (sim_s02, _, _) = run_sim(&seg);
    println!("SAMPLED-PROFILE S02-STAGED-CACHE {}", sim_s02.stats_line());

    // Interleaved sampled sequences (frame-major, slot-minor),
    // rebuilt from the per-slot folds.
    let mut sampled_a: Vec<Req> = Vec::new();
    let mut sampled_b: Vec<Req> = Vec::new();
    {
        let folds_a: Vec<Vec<Req>> = (0..nslots).map(|s| sampled_fold(&seg, s, nslots, false)).collect();
        let folds_b: Vec<Vec<Req>> = (0..nslots).map(|s| sampled_fold(&seg, s, nslots, true)).collect();
        for i in 0..214 {
            for s in 0..nslots {
                // Folds own their Reqs; move them out via a
                // channel-preserving copy (holds are None here).
                let ra = &folds_a[s][i];
                sampled_a.push(Req { look: ra.look, state: copy_state(&ra.state), hold_l: ra.hold_l, hold_r: ra.hold_r });
                let rb = &folds_b[s][i];
                sampled_b.push(Req { look: rb.look, state: copy_state(&rb.state), hold_l: rb.hold_l, hold_r: rb.hold_r });
            }
        }
    }
    let a_equal = seg.iter().zip(sampled_a.iter()).filter(|(x, y)| key_of(x) == key_of(y)).count();
    println!(
        "SAMPLED-PROFILE S02-SAMPLED-A requests={} distinct_states={} fold_equals_staged={}/{}",
        sampled_a.len(), distinct_count(&sampled_a), a_equal, seg.len()
    );
    println!(
        "SAMPLED-PROFILE S02-SAMPLED-B requests={} distinct_states={}",
        sampled_b.len(), distinct_count(&sampled_b)
    );
    let (sim_a, _, _) = run_sim(&sampled_a);
    println!("SAMPLED-PROFILE S02-SAMPLED-A-CACHED-COUNTERFACTUAL {}", sim_a.stats_line());
    let (sim_b, _, _) = run_sim(&sampled_b);
    println!("SAMPLED-PROFILE S02-SAMPLED-B-CACHED-COUNTERFACTUAL {}", sim_b.stats_line());
    println!(
        "SAMPLED-PROFILE S02-SAMPLED-ASBUILT cache=none-in-generic-path hits=0 misses={} evictions=0 peak_cache_bytes=0 (scenefile paints unconditionally)",
        sampled_a.len()
    );

    // ---- 3. CPU paint wall times for the segment (CI runner CPU) ----
    let (uncached_a, cached_a, per_a) = time_paints(&looks, &sampled_a);
    println!(
        "SAMPLED-PROFILE S02-CPU-PAINT sampled-A uncached_total_ms={:.1} cached_total_ms={:.1} per_paint_median_ms={:.3} per_paint_mean_ms={:.3} n={}",
        uncached_a, cached_a, median_ms(per_a.clone()), per_a.iter().sum::<f64>() / per_a.len() as f64, per_a.len()
    );
    let (uncached_s, cached_s, per_s) = time_paints(&looks, &seg);
    println!(
        "SAMPLED-PROFILE S02-CPU-PAINT staged uncached_total_ms={:.1} cached_total_ms={:.1} per_paint_median_ms={:.3} per_paint_mean_ms={:.3} n={}",
        uncached_s, cached_s, median_ms(per_s.clone()), per_s.iter().sum::<f64>() / per_s.len() as f64, per_s.len()
    );

    // ---- 4. Display lists for GPU replay (workflow replays them) ----
    if let Err(e) = std::fs::create_dir_all(lists_dir) {
        eprintln!("SAMPLED-PROFILE ERROR create {lists_dir}: {e}");
        return 4;
    }
    crate::gputrace::set_armed(true);
    for (k, i) in [0usize, 36, 72, 108, 144, 180].iter().enumerate() {
        // Frame i, slot 0 (Onoda) in frame-major order.
        record_list(&looks, &seg[i * nslots], lists_dir, &format!("staged-f{i}"));
        record_list(&looks, &sampled_b[i * nslots], lists_dir, &format!("sampled-b-f{i}"));
        let _ = k;
    }
    crate::gputrace::set_armed(false);
    println!("SAMPLED-PROFILE END");
    0
}
