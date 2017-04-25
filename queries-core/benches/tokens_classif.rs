#[macro_use]
extern crate bencher;
extern crate queries_core;
extern crate ndarray;
extern crate yolo;

use bencher::Bencher;
use ndarray::prelude::*;
use yolo::Yolo;

use queries_core::{AssistantConfig, FileBasedAssistantConfig};
use queries_core::pipeline::deep::tf_classifier_wrapper::TFClassifierWrapper;
use queries_core::pipeline::ClassifierWrapper;

fn get_tokens_classifier(classifier: &str) -> TFClassifierWrapper<Array1<usize>> {
    let root_dir = queries_core::file_path("untracked");
    let assistant_config = FileBasedAssistantConfig::new(root_dir).yolo();
    let intent_config = assistant_config
        .get_intent_configuration(classifier)
        .yolo();
    TFClassifierWrapper::new_tokens_classifier(intent_config).yolo()
}

macro_rules! load_classifier {
    ($name:ident, $classifier:expr) => {
        fn $name(bench: &mut Bencher) {
            bench.iter(|| {
                let _ = get_tokens_classifier($classifier);
            });
        }
    }
}

macro_rules! run_classifier {
    ($name:ident, $classifier:expr, $input:expr) => {
        fn $name(bench: &mut Bencher) {
            let classifier = get_tokens_classifier($classifier);
            let preprocessor_result = queries_core::preprocess($input).yolo();

            bench.iter(|| {
                let _ = classifier.run(&preprocessor_result);
            });
        }
    }
}

load_classifier!(load_book_restaurant, "BookRestaurant");
load_classifier!(load_get_weather, "GetWeather");
load_classifier!(load_play_music, "PlayMusic");

run_classifier!(run_book_restaurant, "BookRestaurant",
"Book me a table for four people at Le Chalet Savoyard tonight");
run_classifier!(run_get_weather, "GetWeather",
"What will be the weather tomorrow in Paris ?");
run_classifier!(run_play_music, "PlayMusic",
"Give me some psychedelic hip-hop please");

benchmark_group!(load, load_book_restaurant, load_get_weather, load_play_music);
benchmark_group!(run, run_book_restaurant, run_get_weather, run_play_music);

benchmark_main!(load, run);
