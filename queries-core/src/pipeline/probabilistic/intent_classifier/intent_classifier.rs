use ndarray::prelude::*;

use errors::*;
use models::logreg::MulticlassLogisticRegression;
use pipeline::IntentClassifierResult;
use pipeline::probabilistic::intent_classifier::featurizer::Featurizer;
use pipeline::probabilistic::configuration::IntentClassifierConfiguration;
use utils::miscellaneous::argmax;
use utils::token::tokenize_light;
use models::stemmer::{Stemmer, StaticMapStemmer};

pub struct IntentClassifier {
    language_code: String,
    intent_list: Vec<Option<String>>,
    featurizer: Option<Featurizer>,
    logreg: Option<MulticlassLogisticRegression>,
}

impl IntentClassifier {
    pub fn new(config: IntentClassifierConfiguration) -> Result<Self> {
        let featurizer = config.featurizer.map(Featurizer::new);
        let logreg =
            if let (Some(intercept), Some(coeffs)) = (config.intercept, config.coeffs) {
                let arr_intercept = Array::from_vec(intercept);
                let nb_classes = arr_intercept.dim();
                let nb_features = coeffs[0].len();
                // Note: the deserialized coeffs matrix is transposed
                let arr_weights = Array::from_shape_fn((nb_features, nb_classes), |(i, j)| coeffs[j][i]);
                MulticlassLogisticRegression::new(arr_intercept, arr_weights).map(Some)
            } else {
                Ok(None)
            }?;

        Ok(Self {
            language_code: config.language_code,
            intent_list: config.intent_list,
            featurizer,
            logreg
        })
    }

    pub fn get_intent(&self, input: &str) -> Result<Option<IntentClassifierResult>> {
        if input.is_empty() || self.intent_list.is_empty() || self.featurizer.is_none() ||
            self.logreg.is_none() {
            return Ok(None);
        }

        if self.intent_list.len() == 1 {
            return if let Some(ref intent_name) = self.intent_list[0] {
                Ok(Some(IntentClassifierResult { intent_name: intent_name.clone(), probability: 1.0 }))
            } else {
                Ok(None)
            }
        }

        let featurizer = self.featurizer.as_ref().unwrap(); // checked
        let log_reg = self.logreg.as_ref().unwrap(); // checked

        let stemmed_text: String = StaticMapStemmer::new(self.language_code.clone()).ok()
            .map(|stemmer| stem_sentence(input, &stemmer))
            .unwrap_or(input.to_string());

        let features = featurizer.transform(&stemmed_text)?;
        let probabilities = log_reg.run(&features.view())?;

        let (index_predicted, best_probability) = argmax(&probabilities);

        if let Some(ref intent_name) = self.intent_list[index_predicted] {
            Ok(Some(IntentClassifierResult { intent_name: intent_name.clone(), probability: best_probability }))
        } else {
            Ok(None)
        }
    }
}

fn stem_sentence<S: Stemmer>(input: &str, stemmer: &S) -> String {
    let stemmed_words: Vec<_> = tokenize_light(input)
        .iter()
        .map(|word| stemmer.stem(word))
        .collect();
    stemmed_words.join(" ")
}

#[cfg(test)]
mod tests {
    use super::IntentClassifier;
    use super::Featurizer;
    use ndarray::*;
    use models::logreg::MulticlassLogisticRegression;
    use pipeline::IntentClassifierResult;
    use pipeline::probabilistic::configuration::FeaturizerConfiguration;

    #[test]
    fn get_intent_works() {
        // Given
        let language_code = "en".to_string();
        let best_features = vec![1, 2, 15, 17, 19, 20, 21, 22, 28, 30, 36, 37, 44, 45, 47, 54, 55, 68, 72, 73, 82, 92, 93, 96, 97, 100, 101];
        let vocabulary = hashmap![
            "!".to_string() => 0,
            "12".to_string() => 1,
            "?".to_string() => 2,
            "a".to_string() => 3,
            "about".to_string() => 4,
            "agent".to_string() => 5,
            "albuquerque".to_string() => 6,
            "and".to_string() => 7,
            "ask".to_string() => 8,
            "assume".to_string() => 9,
            "at".to_string() => 10,
            "be".to_string() => 11,
            "believe".to_string() => 12,
            "border".to_string() => 13,
            "break".to_string() => 14,
            "brew".to_string() => 15,
            "buena".to_string() => 16,
            "can".to_string() => 17,
            "center".to_string() => 18,
            "coffe".to_string() => 19,
            "coffees".to_string() => 20,
            "cold".to_string() => 21,
            "cup".to_string() => 22,
            "do".to_string() => 23,
            "down".to_string() => 24,
            "easi".to_string() => 25,
            "feel".to_string() => 26,
            "fellas".to_string() => 27,
            "five".to_string() => 28,
            "for".to_string() => 29,
            "four".to_string() => 30,
            "france".to_string() => 31,
            "fun".to_string() => 32,
            "game".to_string() => 33,
            "gather".to_string() => 34,
            "georgina".to_string() => 35,
            "get".to_string() => 36,
            "give".to_string() => 37,
            "going".to_string() => 38,
            "he".to_string() => 39,
            "hear".to_string() => 40,
            "here".to_string() => 41,
            "him".to_string() => 42,
            "hollywood".to_string() => 43,
            "hot".to_string() => 44,
            "hundr".to_string() => 45,
            "i".to_string() => 46,
            "iced".to_string() => 47,
            "in".to_string() => 48,
            "it".to_string() => 49,
            "kind".to_string() => 50,
            "lassy".to_string() => 51,
            "like".to_string() => 52,
            "m".to_string() => 53,
            "make".to_string() => 54,
            "me".to_string() => 55,
            "miller".to_string() => 56,
            "miltan".to_string() => 57,
            "my".to_string() => 58,
            "n".to_string() => 59,
            "newhouse".to_string() => 60,
            "no".to_string() => 61,
            "of".to_string() => 62,
            "off".to_string() => 63,
            "offended".to_string() => 64,
            "offic".to_string() => 65,
            "okay".to_string() => 66,
            "on".to_string() => 67,
            "one".to_string() => 68,
            "orlando".to_string() => 69,
            "patrol".to_string() => 70,
            "plane".to_string() => 71,
            "please".to_string() => 72,
            "prepare".to_string() => 73,
            "prostitutes".to_string() => 74,
            "realli".to_string() => 75,
            "ribs".to_string() => 76,
            "roger".to_string() => 77,
            "s".to_string() => 78,
            "scrapple".to_string() => 79,
            "scumbag".to_string() => 80,
            "she".to_string() => 81,
            "six".to_string() => 82,
            "someth".to_string() => 83,
            "sound".to_string() => 84,
            "special".to_string() => 85,
            "states".to_string() => 86,
            "strike".to_string() => 87,
            "studio".to_string() => 88,
            "suerte".to_string() => 89,
            "t".to_string() => 90,
            "take".to_string() => 91,
            "tea".to_string() => 92,
            "teas".to_string() => 93,
            "the".to_string() => 94,
            "think".to_string() => 95,
            "thousand".to_string() => 96,
            "three".to_string() => 97,
            "to".to_string() => 98,
            "truth".to_string() => 99,
            "twenti".to_string() => 100,
            "two".to_string() => 101,
            "united".to_string() => 102,
            "well".to_string() => 103,
            "what".to_string() => 104,
            "when".to_string() => 105,
            "whew".to_string() => 106,
            "why".to_string() => 107,
            "with".to_string() => 108,
            "wo".to_string() => 109,
            "would".to_string() => 110,
            "wow".to_string() => 111,
            "you".to_string() => 112,
        ];

        let diag_elements = vec![
            3.56494935746,
            3.97041446557,
            3.97041446557,
            3.27726728501,
            3.97041446557,
            3.97041446557,
            3.97041446557,
            3.27726728501,
            3.97041446557,
            3.56494935746,
            3.97041446557,
            2.71765149707,
            3.97041446557,
            3.97041446557,
            3.97041446557,
            3.56494935746,
            3.97041446557,
            3.97041446557,
            3.97041446557,
            3.0541237337,
            3.97041446557,
            3.97041446557,
            2.58412010445,
            3.97041446557,
            3.97041446557,
            3.97041446557,
            3.97041446557,
            3.97041446557,
            3.56494935746,
            3.27726728501,
            3.97041446557,
            3.97041446557,
            3.97041446557,
            3.97041446557,
            3.97041446557,
            3.97041446557,
            3.97041446557,
            3.97041446557,
            3.56494935746,
            3.27726728501,
            3.97041446557,
            3.97041446557,
            3.97041446557,
            3.97041446557,
            3.97041446557,
            3.97041446557,
            2.71765149707,
            3.97041446557,
            3.97041446557,
            3.56494935746,
            3.97041446557,
            3.97041446557,
            3.97041446557,
            3.97041446557,
            2.71765149707,
            2.46633706879,
            3.97041446557,
            3.97041446557,
            3.97041446557,
            3.56494935746,
            3.97041446557,
            3.97041446557,
            2.8718021769,
            3.97041446557,
            3.97041446557,
            3.56494935746,
            3.97041446557,
            3.56494935746,
            3.27726728501,
            3.97041446557,
            3.97041446557,
            3.97041446557,
            3.56494935746,
            3.97041446557,
            3.97041446557,
            3.97041446557,
            3.97041446557,
            3.97041446557,
            3.0541237337,
            3.97041446557,
            3.97041446557,
            3.97041446557,
            3.97041446557,
            3.97041446557,
            3.97041446557,
            3.97041446557,
            3.97041446557,
            3.97041446557,
            3.97041446557,
            3.97041446557,
            3.56494935746,
            3.97041446557,
            3.56494935746,
            3.56494935746,
            3.27726728501,
            3.97041446557,
            3.97041446557,
            3.56494935746,
            3.27726728501,
            3.97041446557,
            3.56494935746,
            3.56494935746,
            3.97041446557,
            3.97041446557,
            3.97041446557,
            3.97041446557,
            3.97041446557,
            3.97041446557,
            3.56494935746,
            3.97041446557,
            3.97041446557,
            3.97041446557,
            3.27726728501
        ];

        let intent_list: Vec<Option<String>> = vec![
            Some("MakeCoffee".to_string()),
            Some("MakeTea".to_string()),
            None
        ];

        let config = FeaturizerConfiguration {
            tfidf_vectorizer_idf_diag: diag_elements,
            best_features: best_features,
            tfidf_vectorizer_vocab: vocabulary,
            tfidf_vectorizer_stop_words: None
        };

        let featurizer = Featurizer::new(config);

        let intercept = array![-0.6769558144299883, -0.6587242944035958, 0.22680835693804338];

        let coeffs_vec = vec![
            [
                0.47317020196399323,
                -0.38075250099680313,
                1.107799468598624,
                -0.38075250099680313,
                1.8336263975786775,
                0.8353246023070073,
                -0.38075250099680313,
                2.2249713330204766,
                0.08564143623516322,
                0.5332023901777503,
                -0.38075250099680313,
                0.8353246023070073,
                -0.550417616014284,
                0.7005943889737921,
                -0.6161745296811834,
                0.7232703408462136,
                1.5548021356237207,
                0.26001735853448454,
                0.40815046754904194,
                -0.550417616014284,
                0.8353246023070073,
                -1.4480803940924434,
                -0.8951192396337332,
                0.47613450034233684,
                0.30011894863821786,
                0.24107723670655656,
                0.07579876754730583
            ],
            [
                -0.36011489995898516,
                0.9544411862213601,
                -0.6209197902493954,
                0.9544411862213601,
                -1.3347876038937607,
                -0.45132716150922075,
                0.9544411862213601,
                -1.144908928720865,
                0.4753730257377091,
                -0.25761552096599194,
                0.9544411862213601,
                -0.45132716150922075,
                1.2004101968975385,
                -0.43392555576901004,
                1.2094993585173603,
                0.6986318740136787,
                -1.0131190277108526,
                0.7937664891170565,
                0.45173521169661446,
                1.2004101968975385,
                -0.45132716150922075,
                2.9446608222158592,
                1.9429554575341705,
                -0.42500086360353684,
                0.3681826115884594,
                0.3763435734118238,
                0.696370959190279
            ],
            [
                -0.3208821394723137,
                -0.4047461312958966,
                -0.73500565414034,
                -0.4047461312958966,
                -0.9726774017143353,
                -0.46703967551075193,
                -0.4047461312958966,
                -1.5028381667964201,
                -0.5558940158035158,
                -0.4547178634891068,
                -0.4047461312958966,
                -0.46703967551075193,
                -0.424271594788462,
                -0.3638848118522113,
                -0.4134263927057856,
                -1.3356856351554096,
                -1.2356655443188445,
                -0.7929704501312185,
                -0.782757638722614,
                -0.424271594788462,
                -0.46703967551075193,
                -0.8045518775902378,
                -0.7346194305470242,
                -0.21437336251972489,
                -0.61116631674614,
                -0.6014286441350187,
                -0.6979309347340573
            ]
        ];

        let coeffs: Array2<f32> = Array::from_shape_fn((27, 3), |(i, j)| coeffs_vec[j][i]);
        let logreg = MulticlassLogisticRegression::new(intercept, coeffs).unwrap();
        let classifier = IntentClassifier {
            language_code,
            intent_list,
            featurizer: Some(featurizer),
            logreg: Some(logreg)
        };

        // When
        let classification_result = classifier.get_intent("Make me two cups of tea");
        let ref actual_result = classification_result.unwrap().unwrap();
        let expected_result = IntentClassifierResult {
            intent_name: "MakeTea".to_string(),
            probability: 0.48829985
        };

        // Then

        assert_eq!(expected_result.intent_name, actual_result.intent_name);
        assert_eq!(expected_result.probability, actual_result.probability);
    }
}
