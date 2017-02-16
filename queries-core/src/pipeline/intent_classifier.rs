use std::fs;

use protobuf;
use ndarray::prelude::*;

use ::FileConfiguration;
use models::model::{Model, Matrix};
use models::classifiers::{Classifier, LogisticRegression};
use preprocessing::PreprocessorResult;
use pipeline::Probability;
use pipeline::feature_processor::{MatrixFeatureProcessor, ProtobufMatrixFeatureProcessor};

pub trait IntentClassifier {
    fn run(&self, preprocessor_result: &PreprocessorResult) -> Probability;
}

pub struct ProtobufIntentClassifier {
    file_configuration: FileConfiguration,
    model: Model,
}

impl ProtobufIntentClassifier {
    pub fn new(file_configuration: &FileConfiguration, classifier_name: &str) -> ProtobufIntentClassifier {
        let classifier_path = file_configuration.intent_classifier_path(classifier_name);

        let mut model_file = fs::File::open(classifier_path).unwrap();
        let model = protobuf::parse_from_reader::<Model>(&mut model_file).unwrap();
        ProtobufIntentClassifier { file_configuration: file_configuration.clone(), model: model }
    }
}

impl IntentClassifier for ProtobufIntentClassifier {
    fn run(&self, preprocessor_result: &PreprocessorResult) -> Probability {
        let feature_processor = ProtobufMatrixFeatureProcessor::new(&self.file_configuration, &self.model.get_features());
        let computed_features = feature_processor.compute_features(preprocessor_result);

        let classifier =
            LogisticRegression::new(self.model.get_arguments()[0].get_matrix().to_array2());
        let probabilities = classifier.run(&computed_features);

        probabilities[[0, 0]]
    }
}

impl Matrix {
    fn to_array2(&self) -> Array2<f64> {
        let matrix_buffer = self.get_buffer();

        match Array::from_vec(matrix_buffer.to_vec())
            .into_shape((self.rows as usize, self.cols as usize)) {
            Ok(array) => array,
            Err(error) => panic!("Can't convert matrix into array2. Reason: {}", error),
        }
    }
}

#[cfg(test)]
mod test {
    use std::fs;

    use ndarray::arr2;

    use preprocessing::preprocess;
    use testutils::file_configuration;
    use testutils::parse_json;
    use testutils::create_array;
    use testutils::assert_epsilon_eq;
    use super::{IntentClassifier, ProtobufIntentClassifier};

    #[derive(Deserialize)]
    struct TestDescription {
        text: String,
        output: Vec<Vec<f64>>,
//        features: Vec<Vec<f64>>,
    }

    #[test]
    fn intent_classifier_works() {
        let file_configuration = file_configuration();
        let paths = fs::read_dir("../data/snips-sdk-models/tests/intent_classification/").unwrap();

        for path in paths {
            let path = path.unwrap().path();
            let tests: Vec<TestDescription> = parse_json(path.to_str().unwrap());

            let model_name = path.file_stem().unwrap().to_str().unwrap();
            let intent_classifier = ProtobufIntentClassifier::new(&file_configuration, model_name);

            for test in tests {
                let preprocess_result = preprocess(&test.text);
                let result = intent_classifier.run(&preprocess_result);
                assert_epsilon_eq(arr2(&[[result]]), create_array(&test.output), 1e-9);
            }
        }
    }
}
