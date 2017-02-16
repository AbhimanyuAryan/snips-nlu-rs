#[macro_use]
extern crate error_chain;
extern crate itertools;
#[macro_use]
extern crate lazy_static;
#[macro_use(stack)]
extern crate ndarray;
extern crate protobuf;
extern crate rayon;
extern crate regex;
extern crate rulinalg;
extern crate unicode_normalization;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate tensorflow;

use std::cmp::Ordering;
use std::path;
use std::collections::HashMap;

use itertools::Itertools;
use rayon::prelude::*;

use models::IntentConfiguration;
use pipeline::Probability;
use pipeline::intent_classifier::IntentClassifier;
use pipeline::tokens_classifier::TokensClassifier;
use pipeline::slot_filler::compute_slots;

pub use preprocessing::preprocess;
pub use errors::*;

#[cfg(test)]
mod testutils;

pub mod errors;
pub mod features;
pub mod models;
pub mod pipeline;
pub mod preprocessing;

#[derive(Debug)]
pub struct IntentClassifierResult {
    pub intent_name: String,
    pub probability: Probability,
}

#[derive(Debug, Clone)]
pub struct FileConfiguration {
    pub configurations_dir: String,
    pub tokens_classifiers_dir: String,
    pub intent_classifiers_dir: String,
    pub cnn_classifiers_dir: String,
    pub gazetteer_dir: String,
}

impl FileConfiguration {
    pub fn configuration_path(&self, classifier_name: &str) -> ::path::PathBuf {
        return ::path::Path::new(&self.configurations_dir).join(classifier_name).with_extension("pb");
    }

    pub fn intent_classifier_path(&self, classifier_name: &str) -> ::path::PathBuf {
        return ::path::Path::new(&self.intent_classifiers_dir).join(classifier_name).with_extension("pb");
    }

    pub fn tokens_classifier_path(&self, classifier_name: &str) -> ::path::PathBuf {
        return ::path::Path::new(&self.tokens_classifiers_dir).join(classifier_name).with_extension("pb");
    }

    pub fn gazetteer_path(&self, gazetteer_name: &str) -> ::path::PathBuf {
        return ::path::Path::new(&self.gazetteer_dir).join(gazetteer_name).with_extension("json");
    }

    pub fn default() -> FileConfiguration {
        return FileConfiguration {
            configurations_dir: "../data/snips-sdk-models-protobuf/configuration/".to_string(),
            intent_classifiers_dir: "../data/snips-sdk-models-protobuf/intent_classification/".to_string(),
            tokens_classifiers_dir: "../data/snips-sdk-models-protobuf/tokens_classification/".to_string(),
            cnn_classifiers_dir: "".to_string(),
            gazetteer_dir: "../data/snips-sdk-gazetteers/gazetteers/".to_string(),
        }
    }
}

pub struct IntentParser {
    classifiers: HashMap<String, IntentConfiguration>
}

impl IntentParser {
    pub fn new(file_configuration: &FileConfiguration, configurations: &[&str]) -> IntentParser {
        let mut classifiers = HashMap::new();

        for c in configurations {
            let intent = IntentConfiguration::new(file_configuration, c);
            classifiers.insert(intent.intent_name.to_string(), intent);
        }

        IntentParser { classifiers: classifiers }
    }

    pub fn run_intent_classifiers(&self, input: &str, probability_threshold: f64) -> Vec<IntentClassifierResult> {
        assert!(probability_threshold >= 0.0 && probability_threshold <= 1.0, "probability_treshold should be between 0.0 and 1.0");

        let preprocessor_result = preprocess(input);

        let mut probabilities: Vec<IntentClassifierResult> = self.classifiers
            .par_iter()
            .map(|(name, intent_configuration)| {
                let probability = intent_configuration.intent_classifier.run(&preprocessor_result);
                IntentClassifierResult { intent_name: name.to_string(), probability: probability }
            })
        .filter(|result| result.probability >= probability_threshold)
            .collect();

        probabilities.sort_by(|a, b| {
            a.probability.partial_cmp(&b.probability).unwrap_or(Ordering::Equal).reverse()
        });

        probabilities
    }

    pub fn run_tokens_classifier(&self, input: &str, intent_name: &str) -> HashMap<String, String> {
        let preprocessor_result = preprocess(input);

        let intent_configuration = self.classifiers.get(intent_name).unwrap();
        let probabilities = intent_configuration.tokens_classifier.run(&preprocessor_result).unwrap();

        let token_values = preprocessor_result.tokens.iter().map(|token| &*token.value).collect_vec();
        let slot_values = compute_slots(&*token_values, &probabilities);
        let ref slot_names = intent_configuration.slot_names;

        let mut result = HashMap::new();
        for (name, value) in slot_names.iter().zip(slot_values.iter()) {
            result.insert(name.clone(), value.clone());
        }

        result
    }
}
