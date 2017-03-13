#[macro_use]
extern crate bencher;

extern crate queries_core;

use bencher::Bencher;
use queries_core::FileConfiguration;
use queries_core::IntentParser;

fn load_intent_parser(bench: &mut Bencher) {
    let file_configuration = FileConfiguration::default();

    bench.iter(|| {
        let _ = IntentParser::new(&file_configuration, Some(&["BookRestaurant"])).unwrap();
    });
}

fn run_intent_classifications(bench: &mut Bencher) {
    let file_configuration = FileConfiguration::default();
    let intent_parser = IntentParser::new(&file_configuration, Some(&["BookRestaurant"])).unwrap();

    bench.iter(|| {
        let _ = intent_parser.run_intent_classifiers("Book me a restaurant for two peoples at Le Chalet Savoyard", 0.4, None);
    });
}

fn run_everything(bench: &mut Bencher) {
    let file_configuration = FileConfiguration::default();
    
    bench.iter(|| {
        let intent_parser = IntentParser::new(&file_configuration, Some(&["BookRestaurant"])).unwrap();
        let text = "Book me a restaurant for two peoples at Le Chalet Savoyard";
        let result = intent_parser.run_intent_classifiers(text, 0.4, None);
        let intent = intent_parser.run_tokens_classifier(text, "BookRestaurant");
    });
}

benchmark_group!(load, load_intent_parser);
benchmark_group!(run, run_intent_classifications);
benchmark_group!(everything, run_everything);
benchmark_main!(load, run, everything);
