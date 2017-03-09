use regex::Regex;

use preprocessing::PreprocessorResult;
use models::gazetteer::Gazetteer;

pub fn has_gazetteer_hits<T: Gazetteer>(preprocessor_result: &PreprocessorResult,
                                        gazetteer: &T)
                                        -> Vec<f64> {
    let mut result = vec![0.0; preprocessor_result.tokens.len()];

    for ref ngram in &preprocessor_result.normalized_ngrams {
        if gazetteer.contains(&ngram.0) {
            for index in &ngram.1 {
                result[*index as usize] = 1.0;
            }
        }
    }
    result
}

pub fn ngram_matcher(preprocessor_result: &PreprocessorResult, ngram_to_check: &str) -> Vec<f64> {
    let mut result = vec![0.0; preprocessor_result.tokens.len()];

    for ref ngram in &preprocessor_result.formatted_ngrams {
        if &ngram.0 == ngram_to_check {
            for index in &ngram.1 {
                result[*index as usize] = 1.0;
            }
        }
    }
    result
}

pub fn is_capitalized(preprocessor_result: &PreprocessorResult) -> Vec<f64> {
    preprocessor_result.tokens
        .iter()
        .map(|token| {
            if let Some(first_char) = token.value.chars().next() {
               if first_char.is_uppercase() { 1.0 } else { 0.0 }
            } else {
               0.0
            }
        })
        .collect()
}

#[allow(non_snake_case)]
pub fn is_first_word(preprocessor_result: &PreprocessorResult) -> Vec<f64> {
    // TODO: lazy static
    let PUNCTUATIONS = vec![",", ".", "?"];

    let ref tokens = preprocessor_result.tokens;
    let tokens_count = tokens.len();
    let mut result = vec![0.0; tokens_count];

    let mut i = 0;
    while i < tokens_count && PUNCTUATIONS.contains(&&*tokens[i].normalized_value) {
        i = i + 1;
    }
    if i < tokens_count {
        result[i] = 1.0;
    }
    result
}

#[allow(non_snake_case)]
pub fn is_last_word(preprocessor_result: &PreprocessorResult) -> Vec<f64> {
    // TODO: lazy static
    let PUNCTUATIONS = vec![",", ".", "?"];

    let ref tokens = preprocessor_result.tokens;
    let tokens_count = tokens.len();
    let mut result = vec![0.0; tokens_count];

    let mut i = tokens_count - 1;
    while PUNCTUATIONS.contains(&&*tokens[i].normalized_value) {
        i = i - 1;
    }
    result[i] = 1.0;
    result
}

pub fn contains_possessive(preprocessor_result: &PreprocessorResult) -> Vec<f64> {
    lazy_static! {
        static ref POSSESSIVE_REGEX: Regex = Regex::new(r"'s\b").unwrap();
    }

    let ref tokens = preprocessor_result.tokens;
    tokens.iter()
        .map(|t| {
            if POSSESSIVE_REGEX.is_match(&t.normalized_value) { 1.0 } else { 0.0 }
        })
        .collect()
}

#[cfg(test)]
mod test {
    use std::ops::Range;
    use std::path;

    use super::has_gazetteer_hits;
    use super::ngram_matcher;
    use super::is_capitalized;
    use super::is_first_word;
    use super::is_last_word;
    use super::contains_possessive;
    use preprocessing::{NormalizedToken, PreprocessorResult};
    use preprocessing::convert_byte_index;
    use models::gazetteer::{HashSetGazetteer};
    use testutils::parse_json;
    use FileConfiguration;

    #[derive(Deserialize)]
    struct TestDescription {
        //description: String,
        input: Input,
        args: Vec<Arg>,
        output: Vec<f64>,
    }

    #[derive(Deserialize)]
    struct Input {
        text: String,
        tokens: Vec<Token>,
    }

    #[derive(Deserialize)]
    struct Token {
        #[serde(rename = "startIndex")]
        start_index: usize,
        #[serde(rename = "endIndex")]
        end_index: usize,
        normalized: String,
        value: String,
        entity: Option<String>,
    }

    #[derive(Deserialize)]
    struct Arg {
        //#[serde(rename = "type")]
        //kind: String,
        //name: String,
        value: String,
    }

    impl Token {
        fn to_normalized_token(&self, base_string: &str) -> NormalizedToken {
            NormalizedToken {
                value: self.value.clone(),
                normalized_value: self.normalized.clone(),
                range: Range {
                    start: convert_byte_index(base_string, self.start_index),
                    end: convert_byte_index(base_string, self.end_index),
                },
                char_range: Range {
                    start: self.start_index,
                    end: self.end_index,
                },
                entity: self.entity.clone(),
            }
        }
    }

    #[test]
    fn feature_function_works() {
        let file_configuration = FileConfiguration::default();

        let tests: Vec<(&str, Box<Fn(&FileConfiguration, &TestDescription, Vec<NormalizedToken>)>)> = vec![
            ("hasGazetteerHits", Box::new(has_gazetteer_hits_works)),
            ("ngramMatcher", Box::new(ngram_matcher_works)),
            ("isCapitalized", Box::new(is_capitalized_works)),
            ("isFirstWord", Box::new(is_first_word_works)),
            ("isLastWord", Box::new(is_last_word_works)),
            ("containsPossessive", Box::new(contains_possessive_works)),
        ];

        let path = path::PathBuf::from("snips-sdk-tests/feature_extraction/SharedVector");

        for test in tests {
            let test_name = test.0;
            let test_path = path.join(&test_name).with_extension("json");
            let parsed_tests: Vec<TestDescription> = parse_json(&test_path.to_str().unwrap());
            assert!(parsed_tests.len() != 0);

            for parsed_test in parsed_tests {
                let normalized_tokens: Vec<NormalizedToken> = parsed_test.input
                    .tokens
                    .iter()
                    .map(|test_token| test_token.to_normalized_token(&parsed_test.input.text))
                    .collect();

                test.1(&file_configuration, &parsed_test, normalized_tokens);
            }
        }
    }

    fn has_gazetteer_hits_works(file_configuration: &FileConfiguration, test: &TestDescription, normalized_tokens: Vec<NormalizedToken>) {
        let preprocessor_result = PreprocessorResult::new(normalized_tokens);
        let gazetteer = HashSetGazetteer::new(&file_configuration, &test.args[0].value).unwrap();

        let result = has_gazetteer_hits(&preprocessor_result, &gazetteer);
        assert_eq!(result, test.output)
    }

    fn ngram_matcher_works(_: &FileConfiguration, test: &TestDescription, normalized_tokens: Vec<NormalizedToken>) {
        let preprocessor_result = PreprocessorResult::new(normalized_tokens);
        let result = ngram_matcher(&preprocessor_result, &test.args[0].value);
        assert_eq!(result, test.output)
    }

    fn is_capitalized_works(_: &FileConfiguration, test: &TestDescription, normalized_tokens: Vec<NormalizedToken>) {
        let preprocessor_result = PreprocessorResult::new(normalized_tokens);
        let result = is_capitalized(&preprocessor_result);
        assert_eq!(result, test.output)
    }

    fn is_first_word_works(_: &FileConfiguration, test: &TestDescription, normalized_tokens: Vec<NormalizedToken>) {
        let preprocessor_result = PreprocessorResult::new(normalized_tokens);
        let result = is_first_word(&preprocessor_result);
        assert_eq!(result, test.output)
    }

    fn is_last_word_works(_: &FileConfiguration, test: &TestDescription, normalized_tokens: Vec<NormalizedToken>) {
        let preprocessor_result = PreprocessorResult::new(normalized_tokens);
        let result = is_last_word(&preprocessor_result);
        assert_eq!(result, test.output)
    }

    fn contains_possessive_works(_: &FileConfiguration, test: &TestDescription, normalized_tokens: Vec<NormalizedToken>) {
        let preprocessor_result = PreprocessorResult::new(normalized_tokens);
        let result = contains_possessive(&preprocessor_result);
        assert_eq!(result, test.output)
    }
}
