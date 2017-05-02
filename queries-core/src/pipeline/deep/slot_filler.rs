use ndarray::prelude::*;

use pipeline::SlotValue;

use preprocessing::PreprocessorResult;

pub fn compute_slots(preprocessor_result: &PreprocessorResult,
                     num_slots: usize,
                     tokens_predictions: &Array1<usize>)
                     -> Vec<Vec<SlotValue>> {
    let mut result: Vec<Vec<SlotValue>> = vec![vec![]; num_slots];

    for (i, token) in preprocessor_result.tokens.iter().enumerate() {
        if tokens_predictions[i] == 0 { continue }

        let ref mut tokens = result[tokens_predictions[i] - 1];

        if tokens.is_empty() || (i > 0 && tokens_predictions[i] != tokens_predictions[i - 1]) {
            tokens.push(SlotValue { value: token.value.to_string(), range: token.char_range.clone() });
        } else {
            let existing_token = tokens.last_mut().unwrap(); // checked
            let ref mut existing_token_value = existing_token.value;
            let ref mut existing_token_range = existing_token.range;
            existing_token_value.push_str(&format!(" {}", &token.value));
            existing_token_range.end = token.char_range.end;
        }
    }
    result
}

#[cfg(test)]
mod test {
    use ndarray::prelude::*;

    use preprocessing::{DeepPreprocessor, Preprocessor};
    use super::SlotValue;
    use super::compute_slots;

    #[test]
    #[ignore]
    // QKFIX: Temporarily ignore this test, waiting for date support of Rustling 
    fn slot_filler_works() {
        let text = "Book me a table for tomorrow at Chartier in the evening";
        let tokens_predictions: Array1<usize> = arr1(&[0, 0, 0, 0, 2, 2, 0, 3, 0, 2, 2]);

        let expected = vec![
            vec![],
            vec![SlotValue { value: "for tomorrow".to_string(), range: 16..28 }],
            vec![SlotValue { value: "Chartier".to_string(), range: 32..40 }],
        ];

        let preprocessor = DeepPreprocessor::new("en").unwrap();
        let preprocess_result = preprocessor.run(text).unwrap();
        let slots = compute_slots(&preprocess_result, expected.len(), &tokens_predictions);
        assert_eq!(slots, expected);
    }
}
