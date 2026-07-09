use serde_json::json;

/// Simple pixel-diff between two PNG/JPEG images using the `image` crate.
pub fn pixel_diff(
    baseline: &[u8],
    current: &[u8],
    threshold: f64,
) -> Result<serde_json::Value, String> {
    use image::{GenericImageView, Pixel};

    let b = image::load_from_memory(baseline).map_err(|e| e.to_string())?;
    let c = image::load_from_memory(current).map_err(|e| e.to_string())?;
    if b.dimensions() != c.dimensions() {
        return Ok(json!({
            "match": false,
            "reason": "dimension_mismatch",
            "baseline": b.dimensions(),
            "current": c.dimensions(),
        }));
    }
    let (w, h) = b.dimensions();
    let total = (w as u64) * (h as u64);
    let mut diff_pixels = 0u64;
    let mut sum_delta = 0u64;
    let b = b.to_rgba8();
    let c = c.to_rgba8();
    for (pb, pc) in b.pixels().zip(c.pixels()) {
        let cb = pb.channels();
        let cc = pc.channels();
        let d = (cb[0] as i32 - cc[0] as i32).unsigned_abs() as u64
            + (cb[1] as i32 - cc[1] as i32).unsigned_abs() as u64
            + (cb[2] as i32 - cc[2] as i32).unsigned_abs() as u64;
        if d > 0 {
            diff_pixels += 1;
            sum_delta += d;
        }
    }
    let ratio = diff_pixels as f64 / total as f64;
    let match_ok = ratio <= threshold;
    Ok(json!({
        "match": match_ok,
        "diff_ratio": ratio,
        "diff_pixels": diff_pixels,
        "total_pixels": total,
        "mean_channel_delta": if diff_pixels > 0 { sum_delta as f64 / (diff_pixels as f64 * 3.0) } else { 0.0 },
        "threshold": threshold,
    }))
}
