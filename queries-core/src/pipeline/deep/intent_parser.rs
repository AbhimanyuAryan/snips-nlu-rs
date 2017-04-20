use std::cmp::Ordering;
use std::collections::HashMap;

use rayon::prelude::*;

use errors::*;

use preprocessing::preprocess;
use config::AssistantConfig;

use pipeline::{ClassifierWrapper, IntentClassifierResult, IntentParser, Slots};
use super::intent_configuration::IntentConfiguration;
use super::slot_filler::compute_slots;

pub struct DeepIntentParser {
    classifiers: HashMap<String, IntentConfiguration>,
}

impl DeepIntentParser {
    pub fn new(assistant_config: &AssistantConfig) -> Result<DeepIntentParser> {
        let mut classifiers = HashMap::new();

        for ref c in assistant_config.get_available_intents_names()? {
            let intent_config = assistant_config.get_intent_configuration(c)?;
            let intent = IntentConfiguration::new(intent_config)?;
            classifiers.insert(intent.intent_name.to_string(), intent);
        }

        Ok(DeepIntentParser { classifiers: classifiers })
    }
}

impl IntentParser for DeepIntentParser {
    fn get_intent(&self,
                  input: &str,
                  probability_threshold: f32,
                  entities: &str)
                  -> Result<Vec<IntentClassifierResult>> {
        ensure!(probability_threshold >= 0.0 && probability_threshold <= 1.0, "probability_threshold must be between 0.0 and 1.0");

        let preprocessor_result = preprocess(input, entities)?;

        let mut probabilities: Vec<IntentClassifierResult> = self.classifiers
            .par_iter()
            .map(|(name, intent_configuration)| {
                let probability = intent_configuration.intent_classifier.run(&preprocessor_result);
                IntentClassifierResult {
                    name: name.to_string(),
                    probability: probability.unwrap_or_else(|e| {
                        println!("could not run intent classifier for {} : {:?}", name, e);
                        -1.0
                    }),
                }
            })
            .filter(|result| result.probability >= probability_threshold)
            .collect();

        probabilities.sort_by(|a, b| {
            a.probability.partial_cmp(&b.probability).unwrap_or(Ordering::Equal).reverse()
        });

        Ok(probabilities)
    }

    fn get_entities(&self,
                    input: &str,
                    intent_name: &str,
                    entities: &str)
                    -> Result<Slots> {
        let preprocessor_result = preprocess(input, entities)?;

        let intent_configuration = self.classifiers
            .get(intent_name)
            .ok_or(format!("intent {:?} not found", intent_name))?;
        let predictions = intent_configuration.tokens_classifier.run(&preprocessor_result)?;

        let slot_names = &intent_configuration.slot_names;
        let slot_values = &compute_slots(&preprocessor_result, slot_names.len(), &predictions);

        let mut result = HashMap::new();
        for (name, value) in slot_names.iter().zip(slot_values.iter()) {
            result.insert(name.clone(), value.clone());
        }

        Ok(result)
    }
}

