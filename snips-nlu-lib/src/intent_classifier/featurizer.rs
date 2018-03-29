use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use itertools::Itertools;
use ndarray::prelude::*;

use configurations::FeaturizerConfiguration;
use errors::*;
use language::{FromLanguage, LanguageConfig};
use nlu_utils::language::Language as NluUtilsLanguage;
use nlu_utils::string::normalize;
use nlu_utils::token::{compute_all_ngrams, tokenize_light};
use resources::stemmer::{StaticMapStemmer, Stemmer};
use resources::word_clusterer::{StaticMapWordClusterer, WordClusterer};

pub struct Featurizer {
    best_features: Vec<usize>,
    vocabulary: HashMap<String, usize>,
    idf_diag: Vec<f32>,
    sublinear: bool,
    language_config: LanguageConfig,
    word_clusterer: Option<StaticMapWordClusterer>,
    stemmer: Option<StaticMapStemmer>,
    entity_utterances_to_feature_names: HashMap<String, Vec<String>>,
}

impl Featurizer {
    pub fn new(config: FeaturizerConfiguration) -> Self {
        let best_features = config.best_features;
        let vocabulary = config.tfidf_vectorizer.vocab;
        let idf_diag = config.tfidf_vectorizer.idf_diag;
        let language_config = LanguageConfig::from_str(&config.language_code).unwrap();
        let word_clusterer = language_config
            .intent_classification_clusters()
            .map(|clusters_name| {
                StaticMapWordClusterer::new(language_config.language, clusters_name.to_string())
                    .ok()
            })
            .unwrap_or(None);
        let stemmer = StaticMapStemmer::new(language_config.language).ok();
        let entity_utterances_to_feature_names = config.entity_utterances_to_feature_names;

        Self {
            best_features,
            vocabulary,
            idf_diag,
            sublinear: config.config.sublinear_tf,
            language_config,
            word_clusterer,
            stemmer,
            entity_utterances_to_feature_names,
        }
    }

    pub fn transform(&self, input: &str) -> Result<Array1<f32>> {
        let preprocessed_tokens = self.preprocess_query(input);
        let vocabulary_size = self.vocabulary.values().max().unwrap() + 1;

        let mut tfidf: Vec<f32> = vec![0.; vocabulary_size];
        let mut match_idx: HashSet<usize> = HashSet::new();
        for word in preprocessed_tokens {
            if let Some(word_idx) = self.vocabulary.get(&word) {
                tfidf[*word_idx] += 1.;
                match_idx.insert(*word_idx);
            }
        }

        for ix in match_idx {
            if self.sublinear {
                tfidf[ix] = (tfidf[ix].ln() + 1.) * self.idf_diag[ix]
            } else {
                tfidf[ix] *= self.idf_diag[ix]
            }
        }

        let l2_norm: f32 = tfidf.iter().fold(0., |norm, v| norm + v * v).sqrt();
        let safe_l2_norm = if l2_norm > 0. { l2_norm } else { 1. };

        tfidf = tfidf.iter().map(|c| *c / safe_l2_norm).collect_vec();

        let selected_features =
            Array::from_iter((0..self.best_features.len()).map(|fi| tfidf[self.best_features[fi]]));
        Ok(selected_features)
    }

    fn preprocess_query(&self, query: &str) -> Vec<String> {
        let tokens = tokenize_light(
            query,
            NluUtilsLanguage::from_language(self.language_config.language),
        );
        let mut processed_tokens: Vec<String> = if let Some(ref stemmer) = self.stemmer {
            tokens.iter().map(|t| stemmer.stem(&normalize(t))).collect()
        } else {
            tokens.iter().map(|t| normalize(t)).collect()
        };
        if let Some(ref clusterer) = self.word_clusterer {
            processed_tokens.append(&mut get_word_cluster_features(&tokens, clusterer))
        }
        processed_tokens.append(&mut get_dataset_entities_features(
            &tokens,
            self.stemmer.as_ref(),
            &self.entity_utterances_to_feature_names,
        ));
        processed_tokens
    }
}

fn get_word_cluster_features<C: WordClusterer>(
    query_tokens: &[String],
    word_clusterer: &C,
) -> Vec<String> {
    let tokens_ref = query_tokens.iter().map(|t| &**t).collect_vec();
    compute_all_ngrams(&tokens_ref[..], tokens_ref.len())
        .into_iter()
        .filter_map(|ngram| word_clusterer.get_cluster(&ngram.0.to_lowercase()))
        .collect()
}

fn get_dataset_entities_features<S: Stemmer>(
    query_tokens: &[String],
    stemmer: Option<&S>,
    entity_utterances_to_feature_names: &HashMap<String, Vec<String>>,
) -> Vec<String> {
    let normalized_tokens: Vec<String> = query_tokens.iter().map(|t| normalize(t)).collect();
    let normalized_stemmed_tokens = stemmer.map_or(normalized_tokens.clone(), |stemmer| {
        normalized_tokens
            .into_iter()
            .map(|t| stem(&t, stemmer))
            .collect()
    });
    let tokens_ref = normalized_stemmed_tokens.iter().map(|t| &**t).collect_vec();
    compute_all_ngrams(&*tokens_ref, tokens_ref.len())
        .into_iter()
        .filter_map(|ngrams| entity_utterances_to_feature_names.get(&ngrams.0))
        .flat_map(|features| features)
        .map(|s| normalize(s))
        .collect()
}

fn stem<S: Stemmer>(input: &str, stemmer: &S) -> String {
    stemmer.stem(input)
}

#[cfg(test)]
mod tests {
    use super::{get_dataset_entities_features, get_word_cluster_features, Featurizer};

    use configurations::{FeaturizerConfigConfiguration, FeaturizerConfiguration,
                         TfIdfVectorizerConfiguration};
    use nlu_utils::language::Language;
    use nlu_utils::token::tokenize_light;
    use resources::stemmer::Stemmer;
    use resources::word_clusterer::WordClusterer;
    use testutils::assert_epsilon_eq_array1;

    struct TestWordClusterer {}

    impl WordClusterer for TestWordClusterer {
        fn get_cluster(&self, word: &str) -> Option<String> {
            match word {
                "love" => Some("cluster_love".to_string()),
                "house" => Some("cluster_house".to_string()),
                _ => None,
            }
        }
    }

    struct TestStemmer {}

    impl Stemmer for TestStemmer {
        fn stem(&self, value: &str) -> String {
            match value {
                "bird" => "bir".to_string(),
                "hello" => "hell".to_string(),
                "is" => "be".to_string(),
                _ => value.to_string(),
            }
        }
    }

    #[test]
    fn transform_works() {
        // Given
        let best_features = vec![0, 1, 2, 3, 6, 7, 8, 9];
        let vocab = hashmap![
            "awful".to_string() => 0,
            "beauti".to_string() => 1,
            "bird".to_string() => 2,
            "blue".to_string() => 3,
            "hello".to_string() => 4,
            "nice".to_string() => 5,
            "world".to_string() => 6,
            "featureentityanimal".to_string() => 7,
            "featureentityword".to_string() => 8,
            "featureentitygreeting".to_string() => 9
        ];

        let idf_diag = vec![
            2.252762968495368,
            2.252762968495368,
            1.5596157879354227,
            2.252762968495368,
            1.8472978603872037,
            1.8472978603872037,
            1.5596157879354227,
            0.7,
            1.7,
            2.7,
        ];

        let entity_utterances_to_feature_names = hashmap![
            "bird".to_string() => vec!["featureentityanimal".to_string()],
            "hello".to_string() => vec!["featureentityword".to_string(), "featureentitygreeting".to_string()]
        ];
        let language_code = "en";
        let tfidf_vectorizer = TfIdfVectorizerConfiguration { idf_diag, vocab };

        let featurizer_config = FeaturizerConfiguration {
            language_code: language_code.to_string(),
            tfidf_vectorizer,
            config: FeaturizerConfigConfiguration {
                sublinear_tf: false,
            },
            best_features,
            entity_utterances_to_feature_names,
        };

        let featurizer = Featurizer::new(featurizer_config);

        // When
        let input = "Hëllo this bïrd is a beautiful Bïrd";
        let features = featurizer.transform(input).unwrap();

        // Then
        let expected_features = array![
            0.0,
            0.40887040136658365,
            0.5661321160803057,
            0.0,
            0.0,
            0.2540962231350679,
            0.30854541380686823,
            0.4900427160462025
        ];
        assert_epsilon_eq_array1(&features, &expected_features, 1e-6);
    }

    #[test]
    fn get_word_cluster_features_works() {
        // Given
        let language = Language::EN;
        let query_tokens = tokenize_light("I, love House, muSic", language);
        let word_clusterer = TestWordClusterer {};

        // When
        let augmented_query = get_word_cluster_features(&query_tokens, &word_clusterer);

        // Then
        let expected_augmented_query =
            vec!["cluster_love".to_string(), "cluster_house".to_string()];
        assert_eq!(augmented_query, expected_augmented_query)
    }

    #[test]
    fn get_dataset_entities_features_works() {
        // Given
        let language = Language::EN;
        let query_tokens = tokenize_light("Hëllo this bïrd is a beautiful Bïrd", language);
        let entity_utterances_to_feature_names = hashmap![
            "bir".to_string() => vec!["featureentityAnimal".to_string()],
            "hell this".to_string() => vec!["featureentityWord".to_string(), "featureentityGreeting".to_string()]
        ];
        let stemmer = TestStemmer {};

        // When
        let entities_features = get_dataset_entities_features(
            &query_tokens,
            Some(&stemmer),
            &entity_utterances_to_feature_names,
        );

        // Then
        let expected_entities_features = vec![
            "featureentityword".to_string(),
            "featureentitygreeting".to_string(),
            "featureentityanimal".to_string(),
            "featureentityanimal".to_string(),
        ];
        assert_eq!(entities_features, expected_entities_features)
    }
}
