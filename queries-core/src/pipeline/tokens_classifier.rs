use std::path;
use std::sync;

use protobuf;
use ndarray::prelude::*;

use errors::*;

use config::IntentConfig;
use preprocessing::PreprocessorResult;
use pipeline::Probability;
use pipeline::feature_processor::{MatrixFeatureProcessor, ProtobufMatrixFeatureProcessor};
use protos::model_configuration::ModelConfiguration;
use models::tf::{TensorFlowClassifier, Classifier};

pub trait TokensClassifier {
    fn run(&self, preprocessor_result: &PreprocessorResult) -> Result<Array2<Probability>>;
}

pub struct ProtobufTokensClassifier {
    intent_config: sync::Arc<Box<IntentConfig>>,
    intent_model: ModelConfiguration,
    classifier: TensorFlowClassifier,
}


// TODO merge code with protobuf intent classifier
impl ProtobufTokensClassifier {
    pub fn new(intent_config: sync::Arc<Box<IntentConfig>>) -> Result<ProtobufTokensClassifier> {
        let pb_config = intent_config.get_pb_config()?;
        let model_path = path::Path::new(pb_config.get_tokens_classifier_path());
        let mut model_file = intent_config.get_file(model_path)?;
        let intent_model = protobuf::parse_from_reader::<ModelConfiguration>(&mut model_file)?;


        let classifier = TensorFlowClassifier::new(&mut intent_config.get_file(path::Path::new(&intent_model.get_model_path()))?);
        Ok(ProtobufTokensClassifier { intent_config: intent_config.clone(), intent_model: intent_model, classifier: classifier? })
    }
}

impl TokensClassifier for ProtobufTokensClassifier {
    fn run(&self, preprocessor_result: &PreprocessorResult) -> Result<Array2<Probability>> {
        let feature_processor = ProtobufMatrixFeatureProcessor::new(self.intent_config.clone(), self.intent_model.get_features());
        let computed_features = feature_processor.compute_features(preprocessor_result);
        Ok(self.classifier.predict_proba(&computed_features.t())?)
    }
}

#[cfg(test)]
mod test {
    use preprocessing::preprocess;
    use FileConfiguration;
    use super::{TokensClassifier, ProtobufTokensClassifier};

    #[test]
    #[ignore]
    // QKFIX: Temporarily ignore this test, waiting for update of protobufs
    fn tokens_classifier_works() {
        let file_configuration = FileConfiguration::default();
        let model_name = "BookRestaurant_features";
        let cnn_name = "BookRestaurant_model";

        let preprocessor_result = preprocess("Book me a table for two people at Le Chalet Savoyard");

        let tokens_classifier = ProtobufTokensClassifier::new(&file_configuration, model_name).unwrap();
        let probabilities = tokens_classifier.run(&preprocessor_result);

        println!("probabilities: {:?}", probabilities);
        println!("shape: {:?}", probabilities.unwrap().shape());
    }
}
