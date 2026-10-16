/// Translate the desktop's option spellings into the values upstream video
/// APIs expect.
///
/// The desktop sends its own enum spellings (`wide_sixteen_by_nine`,
/// `seven_twenty_p`), while both Volcano ARK and ModelVerse take ratio strings
/// (`16:9`) and named resolution tiers (`720p`). The BYOK handlers bypass
/// `artcraft_router`, so nothing else performs this mapping for them.
///
/// Ratios and resolutions the API doesn't offer snap to the nearest option it
/// does, matching the router's mitigation for the same models. Unrecognized
/// values return `None` so the caller omits the field and the upstream applies
/// its own default rather than rejecting the request.
pub fn wire_ratio(value: &str) -> Option<&'static str> {
  match value {
    "auto" | "adaptive" => Some("adaptive"),

    "square" | "square_hd" | "1:1" => Some("1:1"),

    "wide_three_by_two" | "wide_four_by_three" | "wide_five_by_four" | "4:3" => Some("4:3"),

    "wide_sixteen_by_nine" | "wide" | "16:9" => Some("16:9"),

    "wide_twenty_one_by_nine" | "21:9" => Some("21:9"),

    "tall_two_by_three" | "tall_three_by_four" | "tall_four_by_five" | "3:4" => Some("3:4"),

    "tall_nine_by_sixteen" | "tall_nine_by_twenty_one" | "tall" | "9:16" => Some("9:16"),

    _ => None,
  }
}

/// Map a resolution to the tier name the video APIs use. `half_k` and the 1K–4K
/// tiers exist for image models; they snap to the nearest tier the Seedance
/// family offers.
pub fn wire_resolution(value: &str) -> Option<&'static str> {
  match value {
    "half_k" | "four_eighty_p" | "480p" => Some("480p"),

    "one_k" | "seven_twenty_p" | "720p" => Some("720p"),

    "two_k" | "three_k" | "four_k" | "ten_eighty_p" | "1080p" => Some("1080p"),

    _ => None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  mod ratio_tests {
    use super::*;

    #[test]
    fn maps_desktop_spellings_to_wire_values() {
      assert_eq!(wire_ratio("wide_sixteen_by_nine"), Some("16:9"));
      assert_eq!(wire_ratio("tall_nine_by_sixteen"), Some("9:16"));
      assert_eq!(wire_ratio("wide_four_by_three"), Some("4:3"));
      assert_eq!(wire_ratio("tall_three_by_four"), Some("3:4"));
      assert_eq!(wire_ratio("square"), Some("1:1"));
      assert_eq!(wire_ratio("wide_twenty_one_by_nine"), Some("21:9"));
      assert_eq!(wire_ratio("auto"), Some("adaptive"));
    }

    #[test]
    fn maps_semantic_values() {
      assert_eq!(wire_ratio("wide"), Some("16:9"));
      assert_eq!(wire_ratio("tall"), Some("9:16"));
    }

    #[test]
    fn snaps_unsupported_ratios_to_the_nearest_option() {
      // The API offers 16:9, 9:16, 4:3, 3:4, 1:1, 21:9 and adaptive.
      assert_eq!(wire_ratio("wide_three_by_two"), Some("4:3"));
      assert_eq!(wire_ratio("wide_five_by_four"), Some("4:3"));
      assert_eq!(wire_ratio("tall_two_by_three"), Some("3:4"));
      assert_eq!(wire_ratio("tall_four_by_five"), Some("3:4"));
      assert_eq!(wire_ratio("tall_nine_by_twenty_one"), Some("9:16"));
    }

    #[test]
    fn accepts_wire_values_so_callers_can_pass_either_spelling() {
      for value in ["16:9", "9:16", "4:3", "3:4", "1:1", "21:9", "adaptive"] {
        assert_eq!(wire_ratio(value), Some(value), "{:?} should map to itself", value);
      }
    }

    #[test]
    fn unknown_values_fall_through() {
      assert_eq!(wire_ratio("some_new_ratio"), None);
      assert_eq!(wire_ratio(""), None);
    }
  }

  mod resolution_tests {
    use super::*;

    #[test]
    fn maps_desktop_spellings_to_wire_values() {
      assert_eq!(wire_resolution("four_eighty_p"), Some("480p"));
      assert_eq!(wire_resolution("seven_twenty_p"), Some("720p"));
      assert_eq!(wire_resolution("ten_eighty_p"), Some("1080p"));
    }

    #[test]
    fn snaps_unsupported_tiers_to_the_nearest_option() {
      assert_eq!(wire_resolution("half_k"), Some("480p"));
      assert_eq!(wire_resolution("one_k"), Some("720p"));
      assert_eq!(wire_resolution("two_k"), Some("1080p"));
      assert_eq!(wire_resolution("four_k"), Some("1080p"));
    }

    #[test]
    fn accepts_wire_values_so_callers_can_pass_either_spelling() {
      for value in ["480p", "720p", "1080p"] {
        assert_eq!(wire_resolution(value), Some(value), "{:?} should map to itself", value);
      }
    }

    #[test]
    fn unknown_values_fall_through() {
      assert_eq!(wire_resolution("eight_k"), None);
      assert_eq!(wire_resolution(""), None);
    }
  }
}
