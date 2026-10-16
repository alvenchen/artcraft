/// Map the desktop omni model id (`seedance_2p0` / `seedance_1p5_pro` / …) to the
/// Volcano Engine ARK model id (`doubao-seedance-…`).
///
/// TODO(BYOK/Seedance): the dated `doubao-seedance-*` ids below are placeholders
/// copied from ARK doc snapshots and MUST be verified against the current model
/// catalog before this ships. Slugs without a confirmed id return `None` so the
/// caller reports "not yet implemented" rather than submitting a bogus model id.
/// BytePlus (`seedance_2p0_bp*` / `seedance_2p0_bpu*`) variants are intentionally
/// absent — their ARK model ids are not mapped yet. The base URL is user-supplied,
/// so supporting them is a matter of filling in the ids below.
pub fn seedance_model_id(omni_model: &str) -> Option<&'static str> {
  match omni_model {
    // TODO: verify these dated ids against the live ARK catalog.
    "seedance_2p0" => Some("doubao-seedance-2-0-260128"),
    "seedance_1p5_pro" => Some("doubao-seedance-1-5-pro-251215"),
    "seedance_2p5" => Some("doubao-seedance-2-5-260628"),
    // TODO: fill in the remaining volcengine ids.
    "seedance_1p0_lite" => None,
    "seedance_2p0_fast" => None,
    "seedance_2p0_mini" => None,
    "seedance_2p5_u" => None,
    _ => None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn maps_volcengine_ids() {
    assert_eq!(seedance_model_id("seedance_2p0"), Some("doubao-seedance-2-0-260128"));
    assert_eq!(seedance_model_id("seedance_1p5_pro"), Some("doubao-seedance-1-5-pro-251215"));
    assert_eq!(seedance_model_id("seedance_2p5"), Some("doubao-seedance-2-5-260628"));
  }

  #[test]
  fn unmapped_and_byteplus_models_return_none() {
    assert_eq!(seedance_model_id("seedance_1p0_lite"), None);
    assert_eq!(seedance_model_id("seedance_2p0_bp"), None);
    assert_eq!(seedance_model_id("future_video"), None);
  }
}
