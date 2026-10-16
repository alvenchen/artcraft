use enums::tauri::tasks::task_failure_type::TaskFailureType;

/// Map a provider's failure reason text onto the desktop app's [`TaskFailureType`].
///
/// Providers report failures as free-form strings, so this is best-effort: exact
/// messages first, then case-insensitive substring matching, then
/// [`TaskFailureType::GenerationFailed`].
///
/// Mirrors `FailureType::classify_text` in
/// `kinovi_web_client/src/requests/poll_orders/failure_type.rs`, which does the same job
/// for the server-side Seedance2 Pro path. The two tables overlap but aren't identical
/// (different gateways, different wording), and they're duplicated rather than shared
/// because `kinovi_web_client` isn't a desktop dependency — not worth pulling an HTTP
/// client crate in for a string table. If they drift, lift the classifier into a shared
/// crate instead of importing across.
pub fn classify_failure_reason(reason: &str) -> TaskFailureType {
  match reason {
    // --- Exact matches first ---
    "Your uploaded image violates platform rules. Please modify and try again." =>
      TaskFailureType::RuleBansUserImage,
    "Face detected in uploaded media. Please adjust your media and try again." =>
      TaskFailureType::RuleBansUserImageWithFaces,
    "Your input text violates platform rules. Please modify and try again." =>
      TaskFailureType::RuleBansUserTextPrompt,
    "Content violates platform rules. Please modify and try again." =>
      TaskFailureType::RuleBansUserContent,
    "The generated video did not pass review. Credits will not be deducted." =>
      TaskFailureType::RuleBansGeneratedVideo,
    "The generated audio violates platform rules. Please adjust your prompt or images and try again." =>
      TaskFailureType::RuleBansGeneratedAudio,
    "The generated content violates platform rules. Please adjust your prompt or images and try again." =>
      TaskFailureType::RuleBansGeneratedContent,
    _ => classify_failure_reason_by_substring(reason),
  }
}

/// Substring pass, used when no exact message matched.
///
/// Order matters: the generated-media rules are checked before the generic
/// user-content rule because their messages also mention "platform rules".
fn classify_failure_reason_by_substring(reason: &str) -> TaskFailureType {
  let lower = reason.to_lowercase();

  // Provider error codes are stable identifiers, e.g. Seedance/ModelVerse:
  // "code: OutputVideoSensitiveContentDetected.PolicyViolation, ...".
  // NB: Must be checked before the generic "sensitive" rule below, which would otherwise
  // swallow it.
  if lower.contains("outputvideosensitivecontentdetected") {
    return TaskFailureType::RuleBansGeneratedVideo;
  }

  if lower.contains("generated video")
      && (lower.contains("not pass review") || lower.contains("violates"))
  {
    return TaskFailureType::RuleBansGeneratedVideo;
  }
  if lower.contains("generated audio") && lower.contains("violates") {
    return TaskFailureType::RuleBansGeneratedAudio;
  }
  if lower.contains("generated content") && lower.contains("violates") {
    return TaskFailureType::RuleBansGeneratedContent;
  }
  if lower.contains("face detected") || lower.contains("ensure no faces") {
    return TaskFailureType::RuleBansUserImageWithFaces;
  }
  if lower.contains("uploaded image violates") {
    return TaskFailureType::RuleBansUserImage;
  }
  if lower.contains("input text violates") {
    return TaskFailureType::RuleBansUserTextPrompt;
  }
  if lower.contains("content violates")
      || lower.contains("platform rules")
      || lower.contains("sensitive")
  {
    return TaskFailureType::RuleBansUserContent;
  }

  TaskFailureType::GenerationFailed
}

#[cfg(test)]
mod tests {
  use super::*;

  /// Captured verbatim from a real Seedance/ModelVerse failure in
  /// `~/Artcraft/artcraft_debug.log`.
  const MODELVERSE_POLICY_VIOLATION_REASON: &str = "code: OutputVideoSensitiveContentDetected.PolicyViolation, message: The request failed because the output video may be related to copyright restrictions. Request id: 02179211746489200000000000000000000ffffac175b70a78c95";

  mod exact_matches {
    use super::*;

    #[test]
    fn classifies_known_seedance_messages() {
      let cases = [
        (
          "Your uploaded image violates platform rules. Please modify and try again.",
          TaskFailureType::RuleBansUserImage,
        ),
        (
          "Face detected in uploaded media. Please adjust your media and try again.",
          TaskFailureType::RuleBansUserImageWithFaces,
        ),
        (
          "Your input text violates platform rules. Please modify and try again.",
          TaskFailureType::RuleBansUserTextPrompt,
        ),
        (
          "Content violates platform rules. Please modify and try again.",
          TaskFailureType::RuleBansUserContent,
        ),
        (
          "The generated video did not pass review. Credits will not be deducted.",
          TaskFailureType::RuleBansGeneratedVideo,
        ),
        (
          "The generated audio violates platform rules. Please adjust your prompt or images and try again.",
          TaskFailureType::RuleBansGeneratedAudio,
        ),
        (
          "The generated content violates platform rules. Please adjust your prompt or images and try again.",
          TaskFailureType::RuleBansGeneratedContent,
        ),
      ];

      for (reason, expected) in cases {
        assert_eq!(classify_failure_reason(reason), expected, "reason: {reason}");
      }
    }
  }

  mod substring_matches {
    use super::*;

    #[test]
    fn classifies_the_observed_modelverse_policy_violation() {
      assert_eq!(
        classify_failure_reason(MODELVERSE_POLICY_VIOLATION_REASON),
        TaskFailureType::RuleBansGeneratedVideo,
      );
    }

    #[test]
    fn matching_is_case_insensitive() {
      assert_eq!(
        classify_failure_reason("CODE: OUTPUTVIDEOSENSITIVECONTENTDETECTED.POLICYVIOLATION"),
        TaskFailureType::RuleBansGeneratedVideo,
      );
      assert_eq!(
        classify_failure_reason("YOUR UPLOADED IMAGE VIOLATES our policy."),
        TaskFailureType::RuleBansUserImage,
      );
      assert_eq!(
        classify_failure_reason("Ensure No Faces are visible."),
        TaskFailureType::RuleBansUserImageWithFaces,
      );
    }

    #[test]
    fn generated_media_rules_win_over_the_generic_content_rule() {
      // Both "generated video" and "platform rules" appear; the specific rule must win.
      assert_eq!(
        classify_failure_reason("The generated video violates platform rules."),
        TaskFailureType::RuleBansGeneratedVideo,
      );
      assert_eq!(
        classify_failure_reason("The generated audio violates platform rules."),
        TaskFailureType::RuleBansGeneratedAudio,
      );
      assert_eq!(
        classify_failure_reason("The generated content violates platform rules."),
        TaskFailureType::RuleBansGeneratedContent,
      );
    }

    #[test]
    fn classifies_paraphrased_provider_wording() {
      assert_eq!(
        classify_failure_reason("Your uploaded image violates our terms of service."),
        TaskFailureType::RuleBansUserImage,
      );
      assert_eq!(
        classify_failure_reason("input text violates community guidelines"),
        TaskFailureType::RuleBansUserTextPrompt,
      );
      assert_eq!(
        classify_failure_reason("This request violates platform rules."),
        TaskFailureType::RuleBansUserContent,
      );
      assert_eq!(
        classify_failure_reason("Request blocked: sensitive content detected."),
        TaskFailureType::RuleBansUserContent,
      );
    }
  }

  mod fallback {
    use super::*;

    #[test]
    fn unrecognized_reasons_fall_back_to_generation_failed() {
      let cases = [
        "",
        "boom",
        "No queue status URL to check",
        "Wan task was canceled",
        "Seedance task expired",
        "Job completed but no response URL available",
      ];

      for reason in cases {
        assert_eq!(
          classify_failure_reason(reason),
          TaskFailureType::GenerationFailed,
          "reason: {reason}",
        );
      }
    }
  }
}
