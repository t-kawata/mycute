use anyhow::{anyhow, Result};
use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera::tokenizer::Tokenizer;

/// Get a Lindera tokenizer initialized with the embedded IPADIC dictionary.
pub fn get_tokenizer() -> Result<Tokenizer> {
    let dictionary = load_dictionary("embedded://ipadic").map_err(|e| anyhow!("Failed to load IPADIC dictionary: {}", e))?;
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
    let tokenizer = Tokenizer::new(segmenter);
    Ok(tokenizer)
}
