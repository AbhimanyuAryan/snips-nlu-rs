use std::io::prelude::*;
use std::collections::HashSet;
use std::fs::File;
use rustc_serialize::json;

pub trait Gazetteer: Sized {
    fn contains(&self, value: &str) -> bool;
    fn new(json_filename: &str) -> Option<Self>;
}

pub struct HashSetGazetteer {
    values: HashSet<String>,
}

impl Gazetteer for HashSetGazetteer {
    // TODO: To be improve
    fn new(json_filename: &str) -> Option<HashSetGazetteer> {
        let mut f = File::open(format!("../data/snips-sdk-gazetteers/gazetteers/{}.json",
                                       json_filename))
            .unwrap();
        let mut s = String::new();
        assert!(f.read_to_string(&mut s).is_ok());
        let vec: Vec<String> = json::decode(&s).unwrap();
        Some(HashSetGazetteer { values: vec.iter().cloned().collect() })
    }

    fn contains(&self, value: &str) -> bool {
        self.values.contains(value)
    }
}

#[cfg(test)]
mod tests {
    use super::Gazetteer;
    use super::HashSetGazetteer;

    #[test]
    fn gazetteer_work() {
        assert!(HashSetGazetteer::new("action_verbs_infinitive").is_some())
    }
}
