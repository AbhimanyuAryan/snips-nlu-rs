use errors::*;
use ndarray::prelude::*;

use config::ArcBoxedIntentConfig;
use config::IntentConfig;
use features::{shared_scalar, shared_vector};
use preprocessing::PreprocessorResult;
use protos::feature::{Feature, Feature_Scalar_Type, Feature_Vector_Type, Feature_Argument};

pub trait MatrixFeatureProcessor {
    fn compute_features(&self, input: &PreprocessorResult) -> Array2<f32>;
}

pub struct ProtobufMatrixFeatureProcessor<'a> {
    intent_config: ArcBoxedIntentConfig,
    feature_functions: &'a [Feature],
}

impl<'a> ProtobufMatrixFeatureProcessor<'a> {
    pub fn new(intent_config: ArcBoxedIntentConfig,
               features: &'a [Feature])
               -> ProtobufMatrixFeatureProcessor<'a> {
        ProtobufMatrixFeatureProcessor {
            intent_config: intent_config,
            feature_functions: features,
        }
    }
}

impl<'a> MatrixFeatureProcessor for ProtobufMatrixFeatureProcessor<'a> {
    fn compute_features(&self, input: &PreprocessorResult) -> Array2<f32> {
        let computed_values = self.feature_functions
            .iter()
            .flat_map(|feature_function| feature_function.compute(&**self.intent_config, input).unwrap()) // TODO: Dunno how to kill this unwrap
            .collect::<Vec<f32>>();

        let len = self.feature_functions.len();
        let feature_length = computed_values.len() / len;

        match Array::from_vec(computed_values).into_shape((len, feature_length)) {
            Ok(array) => array,
            Err(_) => panic!("A feature function doesn't have the same len as the others."),
        }
    }
}

impl Feature {
    fn compute(&self, intent_config: &IntentConfig, input: &PreprocessorResult) -> Result<Vec<f32>> {
        let arguments = self.get_arguments();

        if self.has_scalar_type() {
            Self::get_shared_scalar(intent_config, input, &self.get_scalar_type(), arguments)
        } else if self.has_vector_type() {
            Self::get_shared_vector(intent_config, input, &self.get_vector_type(), arguments)
        } else {
            bail!("No feature function passed")
        }
    }

    fn get_shared_scalar(intent_config: &IntentConfig,
                         input: &PreprocessorResult,
                         feature_type: &Feature_Scalar_Type,
                         arguments: &[Feature_Argument])
                         -> Result<Vec<f32>> {
        Ok(match *feature_type {
            Feature_Scalar_Type::HAS_GAZETTEER_HITS => {
                let gazetteer = intent_config.get_gazetteer(arguments[0].get_gazetteer())?;
                shared_scalar::has_gazetteer_hits(input, gazetteer)
            }
            Feature_Scalar_Type::NGRAM_MATCHER => {
                shared_scalar::ngram_matcher(input, arguments[0].get_str())
            }
            Feature_Scalar_Type::GET_MESSAGE_LENGTH => {
                let normalization = arguments[0].get_scalar() as f32;
                shared_scalar::get_message_length(input, normalization)
            }
        })
    }

    fn get_shared_vector(intent_config: &IntentConfig,
                         input: &PreprocessorResult,
                         feature_type: &Feature_Vector_Type,
                         arguments: &[Feature_Argument])
                         -> Result<Vec<f32>> {
        Ok(match *feature_type {
            Feature_Vector_Type::HAS_GAZETTEER_HITS => {
                let gazetteer = intent_config.get_gazetteer(arguments[0].get_gazetteer())?;
                shared_vector::has_gazetteer_hits(input, gazetteer)
            }
            Feature_Vector_Type::NGRAM_MATCHER => {
                shared_vector::ngram_matcher(input, arguments[0].get_str())
            }
            Feature_Vector_Type::IS_CAPITALIZED => shared_vector::is_capitalized(input),
            Feature_Vector_Type::IS_DATE => shared_vector::is_date(input),
            Feature_Vector_Type::IS_FIRST_WORD => shared_vector::is_first_word(input),
            Feature_Vector_Type::IS_LAST_WORD => shared_vector::is_last_word(input),
            Feature_Vector_Type::CONTAINS_POSSESSIVE => shared_vector::contains_possessive(input),
            Feature_Vector_Type::CONTAINS_DIGITS => shared_vector::contains_digits(input),
        })
    }
}

#[cfg(test)]
mod test {
    use std::fs;
    use std::path;

    use protobuf;

    use file_path;
    use config::{ AssistantConfig, FileBasedAssistantConfig };
    use preprocessing::preprocess;
    use protos::model_configuration::ModelConfiguration;
    use testutils::parse_json;
    use testutils::create_transposed_array;
    use super::{MatrixFeatureProcessor, ProtobufMatrixFeatureProcessor};

    #[derive(Deserialize)]
    struct TestDescription {
        text: String,
        //output: Vec<Vec<f32>>,
        features: Vec<Vec<f32>>,
    }

    #[test]
    #[ignore]
    // QKFIX: Temporarily ignore this test, waiting for update of protobufs
    fn feature_processor_works() {
        let paths =
            fs::read_dir(file_path("snips-sdk-models-protobuf/tests/intent_classification/"))
                .unwrap();

        for path in paths {
            let path = path.unwrap().path();
            let tests: Vec<TestDescription> = parse_json(path.to_str().unwrap());

            let model_name = path.file_stem().unwrap().to_str().unwrap();
            let intent_config = FileBasedAssistantConfig::default().get_intent_configuration(model_name).unwrap();
            let pb_intent_config = intent_config.get_pb_config().unwrap();
            let mut intent_classifier_config = intent_config.get_file(path::Path::new(pb_intent_config.get_intent_classifier_path())).unwrap();
            let pb_model_config = protobuf::parse_from_reader::<ModelConfiguration>(&mut intent_classifier_config).unwrap();

            for test in tests {
                // TODO: Replace preprocess by json
                let preprocessor_result = preprocess(&test.text, "").unwrap();
                let feature_processor = ProtobufMatrixFeatureProcessor::new(intent_config.clone(),
                                                                            &pb_model_config.get_features());

                let result = feature_processor.compute_features(&preprocessor_result);
                assert_eq!(result,
                           create_transposed_array(&test.features),
                           "for {:?}, input: {}",
                           path.file_stem().unwrap(),
                           &test.text);
            }
        }
    }
}
