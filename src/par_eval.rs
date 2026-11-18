//! GOV-PAR (Batch 3, Part 6b): ordered parallel evaluation helper
//! for the telemetry/check per-sample loops.
//!
//! The design law: parallelise EVALUATION only, never aggregation.
//! Workers evaluate contiguous blocks of sample indices and return
//! per-sample results; the caller stores them indexed by sample
//! and replays every order-sensitive consumer (the `Audit`, the
//! presence accumulator, the paint-cache statistics simulation) in
//! frame order on the calling thread. A verdict therefore cannot
//! depend on thread scheduling: the parallel path computes the
//! same per-sample values as the serial path and aggregates them
//! through the same code in the same order.
//!
//! std::thread only — no new dependencies. When the governor is
//! disabled or its thread cap is 1, `worker_count` yields 1 and
//! `parallel_block_map` runs the single block inline, which is the
//! serial path unchanged.

/// How many evaluation workers the governor permits for `n_items`
/// samples: 1 when the governor is off, otherwise its thread cap
/// clamped to the item count (and at least 1).
pub(crate) fn worker_count(n_items: usize) -> usize {
    let gov = crate::resource::governor();
    if !gov.enabled() {
        return 1;
    }
    gov.thread_cap().clamp(1, n_items.max(1))
}

/// Evaluate `n` items as `workers` contiguous blocks, in parallel,
/// returning one result per item INDEXED BY ITEM (never by
/// completion order).
///
/// `f` is called once per block with that block's index range and
/// must return exactly one `Result` per index in the range, in
/// range order. Block boundaries are a pure partition of
/// `0..n`: block `b` covers `[b*n/workers, (b+1)*n/workers)`.
/// With `workers <= 1` (or fewer than 2 items) `f` runs once,
/// inline, over the whole range — the serial path.
///
/// A panicking worker does not lose the failure or fabricate a
/// partial result: every item of its block becomes an `Err`
/// naming the block, and the caller's lowest-index-error rule
/// (see the telemetry collectors) reports it exactly as a serial
/// failure of that block's first item would be reported.
pub(crate) fn parallel_block_map<T, F>(n: usize, workers: usize, f: F) -> Vec<Result<T, String>>
where
    T: Send,
    F: Fn(std::ops::Range<usize>) -> Vec<Result<T, String>> + Sync,
{
    if workers <= 1 || n < 2 {
        if n == 0 {
            return Vec::new();
        }
        return f(0..n);
    }
    let workers = workers.min(n);
    let bounds: Vec<(usize, usize)> = (0..workers)
        .map(|b| (b * n / workers, (b + 1) * n / workers))
        .collect();
    let mut slots: Vec<Option<Result<T, String>>> = Vec::new();
    slots.resize_with(n, || None);
    std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for (b, (start, end)) in bounds.iter().enumerate() {
            let (start, end) = (*start, *end);
            let f_ref = &f;
            handles.push((b, start, end, scope.spawn(move || f_ref(start..end))));
        }
        for (b, start, end, handle) in handles {
            match handle.join() {
                Ok(results) => {
                    assert_eq!(
                        results.len(),
                        end - start,
                        "par_eval block {b} returned {} results for {} items",
                        results.len(),
                        end - start
                    );
                    for (i, r) in results.into_iter().enumerate() {
                        slots[start + i] = Some(r);
                    }
                }
                Err(_) => {
                    for slot in slots.iter_mut().take(end).skip(start) {
                        *slot = Some(Err(format!(
                            "par_eval: evaluation worker for block {b} (samples {start}..{end}) panicked"
                        )));
                    }
                }
            }
        }
    });
    slots
        .into_iter()
        .map(|s| s.expect("par_eval: every slot filled by construction"))
        .collect()
}

/// The serial failure rule, shared by both telemetry collectors:
/// a run fails with the LOWEST-index sample error (the error the
/// serial loop would have hit first), or yields all values in
/// index order.
pub(crate) fn first_error<T>(results: Vec<Result<T, String>>) -> Result<Vec<T>, String> {
    let mut out = Vec::with_capacity(results.len());
    for r in results {
        match r {
            Ok(v) => out.push(v),
            Err(e) => return Err(e),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn squares(range: std::ops::Range<usize>) -> Vec<Result<usize, String>> {
        range.map(|i| Ok(i * i)).collect()
    }

    #[test]
    fn parallel_results_are_indexed_not_completion_ordered() {
        // 37 items over 4 workers = uneven blocks (9/9/9/10).
        let serial = parallel_block_map(37, 1, squares);
        let parallel = parallel_block_map(37, 4, squares);
        let unwrap = |v: Vec<Result<usize, String>>| {
            v.into_iter().map(|r| r.expect("ok")).collect::<Vec<_>>()
        };
        assert_eq!(unwrap(serial), unwrap(parallel.clone()));
        // Determinism: a second parallel run is identical.
        let parallel2 = parallel_block_map(37, 4, squares);
        assert_eq!(unwrap(parallel), unwrap(parallel2));
    }

    #[test]
    fn first_error_is_the_lowest_index_error() {
        // Errors at indices 5 and 20: serial would report 5's,
        // however the blocks are scheduled.
        let f = |range: std::ops::Range<usize>| {
            range
                .map(|i| {
                    if i == 5 || i == 20 {
                        Err(format!("failure at {i}"))
                    } else {
                        Ok(i)
                    }
                })
                .collect::<Vec<Result<usize, String>>>()
        };
        let results = parallel_block_map(24, 4, f);
        assert_eq!(first_error(results).unwrap_err(), "failure at 5");
    }

    #[test]
    fn empty_and_single_item_inputs() {
        assert!(parallel_block_map(0, 4, squares).is_empty());
        let one = parallel_block_map(1, 4, squares);
        assert_eq!(one.len(), 1);
        assert_eq!(one[0].as_ref().expect("ok"), &0);
    }

    #[test]
    fn worker_count_never_exceeds_items_and_is_at_least_one() {
        assert_eq!(worker_count(0), 1);
        assert_eq!(worker_count(1), 1);
        assert!(worker_count(1000) >= 1);
    }
}
