use std::ops::Range;
use std::collections::HashSet;

use errors::*;
use snips_nlu_ontology::{IntentClassifierResult, Slot};

pub mod deterministic;
pub mod probabilistic;
pub mod nlu_engine;
pub mod assistant_config;
pub mod configuration;
pub mod slot_utils;

#[derive(Debug, Clone, PartialEq)]
pub struct InternalSlot {
    pub value: String,
    pub char_range: Range<usize>,
    pub entity: String,
    pub slot_name: String,
}

trait IntentParser: Send + Sync {
    fn get_intent(
        &self,
        input: &str,
        intents: Option<&HashSet<String>>,
    ) -> Result<Option<IntentClassifierResult>>;
    fn get_slots(&self, input: &str, intent_name: &str) -> Result<Vec<Slot>>;
}

trait FeatureProcessor<I, O> {
    fn compute_features(&self, input: &I) -> O;
}
