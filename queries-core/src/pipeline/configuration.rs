use std::collections::HashMap;

use pipeline::rule_based::RuleBasedParserConfiguration;
use pipeline::probabilistic::ProbabilisticParserConfiguration;

#[derive(Debug, Deserialize)]
pub struct NLUEngineConfiguration {
    pub model: Model,
    pub entities: HashMap<String, Entity>
}

#[derive(Debug, Deserialize)]
pub struct Model {
    pub rule_based_parser: Option<RuleBasedParserConfiguration>,
    pub probabilistic_parser: Option<ProbabilisticParserConfiguration>,
}

#[derive(Debug, Deserialize)]
pub struct Entity {
    pub automatically_extensible: bool,
    pub utterances: HashMap<String, String>
}

#[cfg(test)]
mod tests {
    use super::NLUEngineConfiguration;

    use utils;

    #[test]
    fn deserialization_works() {
        let retrieved: NLUEngineConfiguration = utils::parse_json("tests/assistants/sample/trained_assistant.json");
        assert_eq!("en", retrieved.model.rule_based_parser.unwrap().language);
        assert_eq!("en", retrieved.model.probabilistic_parser.unwrap().language_code);
    }
}
