use std::error::Error;

/// Count tokens for given text using model-specific encoding.
///
/// Supported models:
/// - "o200k_base"  (GPT-4o)
/// - "cl100k_base" (GPT-3.5/4)
///
/// If the model encoding is unavailable this falls back to a naive
/// `text.len() / 4` heuristic which approximates tiktoken behaviour.
pub fn count_tokens(text: &str, model: &str) -> Result<usize, Box<dyn Error + Send + Sync>> {
    // tiktoken-rs can fail if the model name is unknown or the files are
    // missing.  We treat any failure by returning the fallback estimate.
    match model {
        "o200k_base" | "cl100k_base" => match tiktoken_rs::get_bpe_from_model(model) {
            Ok(encoding) => Ok(encoding.encode_ordinary(text).len()),
            Err(_) => Ok(fallback_tokens(text)),
        },
        _ => Ok(fallback_tokens(text)),
    }
}

#[inline]
fn fallback_tokens(text: &str) -> usize {
    // Simple heuristic used widely: 1 token ≈ 4 characters.
    (text.len() + 3) / 4 // round up
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn hello_world_count() {
        let text = "hello world";
        // cl100k_base encodes to 2 tokens. Allow ±1 tolerance.
        let tokens = count_tokens(text, "cl100k_base").unwrap();
        assert!(
            tokens >= 1 && tokens <= 3,
            "expected 1-3 tokens, got {}",
            tokens
        );
    }

    proptest! {
        #[test]
        fn token_bounds(s in "[a-zA-Z0-9 ]{0,200}") {
            let byte_len = s.len();
            let tokens = count_tokens(&s, "cl100k_base").unwrap();
            prop_assert!(tokens >= byte_len / 6, "too small: {} vs {}", tokens, byte_len);
            prop_assert!(tokens <= byte_len * 2 + 1, "too big: {} vs {}", tokens, byte_len);
        }
    }
}
