use std::ops::Range;
use std::collections::{HashMap, HashSet};
use std::iter::FromIterator;

use yolo::Yolo;
use itertools::Itertools;

use errors::*;
use pipeline::InternalSlot;
use nlu_utils::token::Token;
use nlu_utils::string::{convert_to_char_range, suffix_from_char_index};
use utils::permutations;

const BEGINNING_PREFIX: &str = "B-";
const INSIDE_PREFIX: &str = "I-";
const LAST_PREFIX: &str = "L-";
const UNIT_PREFIX: &str = "U-";
const OUTSIDE: &str = "O";

#[derive(Copy, Clone, Debug)]
pub enum TaggingScheme {
    IO,
    BIO,
    BILOU,
}

impl TaggingScheme {
    pub fn from_u8(i: u8) -> Result<TaggingScheme> {
        match i {
            0 => Ok(TaggingScheme::IO),
            1 => Ok(TaggingScheme::BIO),
            2 => Ok(TaggingScheme::BILOU),
            _ => Err(format!("Unknown tagging scheme identifier: {}", i))?
        }
    }
}

pub fn get_substitution_label(labels: &[&str]) -> String {
    if labels.contains(&OUTSIDE) { OUTSIDE.to_string() } else { labels[0].to_string() }
}

pub fn replace_builtin_tags(tags: Vec<String>, builtin_slot_names: HashSet<String>) -> Vec<String> {
    tags.into_iter()
        .map(|tag| {
            if tag == OUTSIDE {
                tag
            } else {
                let slot_name = tag_name_to_slot_name(tag.to_string());
                if builtin_slot_names.contains(&slot_name) {
                    OUTSIDE.to_string()
                } else {
                    tag
                }
            }
        })
        .collect_vec()
}

pub fn tag_name_to_slot_name(tag: String) -> String {
    suffix_from_char_index(tag, 2)
}

fn is_start_of_io_slot(tags: &[String], i: usize) -> bool {
    if i == 0 {
        tags[i] != OUTSIDE
    } else if tags[i] == OUTSIDE {
        false
    } else {
        tags[i - 1] == OUTSIDE
    }
}

fn is_end_of_io_slot(tags: &[String], i: usize) -> bool {
    if i + 1 == tags.len() {
        tags[i] != OUTSIDE
    } else if tags[i] == OUTSIDE {
        false
    } else {
        tags[i + 1] == OUTSIDE
    }
}

fn is_start_of_bio_slot(tags: &[String], i: usize) -> bool {
    if i == 0 {
        tags[i] != OUTSIDE
    } else if tags[i] == OUTSIDE {
        false
    } else if tags[i].starts_with(BEGINNING_PREFIX) {
        true
    } else if tags[i - 1] != OUTSIDE {
        false
    } else {
        true
    }
}

fn is_end_of_bio_slot(tags: &[String], i: usize) -> bool {
    if i + 1 == tags.len() {
        tags[i] != OUTSIDE
    } else if tags[i] == OUTSIDE {
        false
    } else if tags[i + 1].starts_with(INSIDE_PREFIX) {
        false
    } else {
        true
    }
}

fn is_start_of_bilou_slot(tags: &[String], i: usize) -> bool {
    if i == 0 {
        tags[i] != OUTSIDE
    } else if tags[i] == OUTSIDE {
        false
    } else if tags[i].starts_with(BEGINNING_PREFIX) {
        true
    } else if tags[i].starts_with(UNIT_PREFIX) {
        true
    } else if tags[i - 1].starts_with(UNIT_PREFIX) {
        true
    } else if tags[i - 1].starts_with(LAST_PREFIX) {
        true
    } else if tags[i - 1] != OUTSIDE {
        false
    } else {
        true
    }
}

fn is_end_of_bilou_slot(tags: &[String], i: usize) -> bool {
    if i + 1 == tags.len() {
        tags[i] != OUTSIDE
    } else if tags[i] == OUTSIDE {
        false
    } else if tags[i + 1] == OUTSIDE {
        true
    } else if tags[i].starts_with(LAST_PREFIX) {
        true
    } else if tags[i].starts_with(UNIT_PREFIX) {
        true
    } else if tags[i + 1].starts_with(BEGINNING_PREFIX) {
        true
    } else if tags[i + 1].starts_with(UNIT_PREFIX) {
        true
    } else {
        false
    }
}

struct SlotRange {
    slot_name: String,
    range: Range<usize>,
}

fn _tags_to_slots<F1, F2>(tags: &[String],
                          tokens: &[Token],
                          is_start_of_slot: F1,
                          is_end_of_slot: F2)
                          -> Vec<SlotRange>
    where F1: Fn(&[String], usize) -> bool,
          F2: Fn(&[String], usize) -> bool
{
    let mut slots: Vec<SlotRange> = Vec::with_capacity(tags.len());

    let mut current_slot_start = 0;
    for (i, tag) in tags.iter().enumerate() {
        if is_start_of_slot(tags, i) {
            current_slot_start = i;
        }
        if is_end_of_slot(tags, i) {
            slots.push(SlotRange {
                range: tokens[current_slot_start].range.start..tokens[i].range.end,
                slot_name: tag_name_to_slot_name(tag.to_string()),
            });
            current_slot_start = i;
        }
    }
    slots
}

pub fn tags_to_slots(text: &str,
                     tokens: &[Token],
                     tags: &[String],
                     tagging_scheme: TaggingScheme,
                     intent_slots_mapping: &HashMap<String, String>)
                     -> Vec<InternalSlot> {
    let slot_ranges = match tagging_scheme {
        TaggingScheme::IO => _tags_to_slots(tags, tokens, is_start_of_io_slot, is_end_of_io_slot),
        TaggingScheme::BIO => _tags_to_slots(tags, tokens, is_start_of_bio_slot, is_end_of_bio_slot),
        TaggingScheme::BILOU => _tags_to_slots(tags, tokens, is_start_of_bilou_slot, is_end_of_bilou_slot),
    };

    slot_ranges
        .into_iter()
        .map(|s|
            InternalSlot {
                value: text[s.range.clone()].to_string(),
                range: convert_to_char_range(text, &s.range),
                entity: intent_slots_mapping[&s.slot_name].to_string(),
                slot_name: s.slot_name
            })
        .collect()
}

pub fn positive_tagging(tagging_scheme: TaggingScheme, slot_name: &str, slot_size: usize) -> Vec<String> {
    if slot_name == OUTSIDE {
        return vec![OUTSIDE.to_string(); slot_size];
    };

    match tagging_scheme {
        TaggingScheme::IO => {
            vec![format!("{}{}", INSIDE_PREFIX, slot_name); slot_size]
        }
        TaggingScheme::BIO => {
            if slot_size > 0 {
                let mut v1 = vec![format!("{}{}", BEGINNING_PREFIX, slot_name)];
                let mut v2 = vec![format!("{}{}", INSIDE_PREFIX, slot_name); slot_size - 1];
                v1.append(&mut v2);
                v1
            } else {
                vec![]
            }
        }
        TaggingScheme::BILOU => {
            match slot_size {
                0 => vec![],
                1 => vec![format!("{}{}", UNIT_PREFIX, slot_name)],
                _ => {
                    let mut v1 = vec![format!("{}{}", BEGINNING_PREFIX, slot_name)];
                    let mut v2 = vec![format!("{}{}", INSIDE_PREFIX, slot_name); slot_size - 2];
                    v1.append(&mut v2);
                    v1.push(format!("{}{}", LAST_PREFIX, slot_name));
                    v1
                }
            }
        }
    }
}

pub fn get_scheme_prefix(index: usize, indexes: &[usize], tagging_scheme: TaggingScheme) -> &str {
    match tagging_scheme {
        TaggingScheme::IO => INSIDE_PREFIX,
        TaggingScheme::BIO => {
            if index == indexes[0] {
                BEGINNING_PREFIX
            } else {
                INSIDE_PREFIX
            }
        }
        TaggingScheme::BILOU => {
            if indexes.len() == 1 {
                UNIT_PREFIX
            } else if index == indexes[0] {
                BEGINNING_PREFIX
            } else if index == *indexes.last().yolo() {
                LAST_PREFIX
            } else {
                INSIDE_PREFIX
            }
        }
    }
}

pub fn generate_slots_permutations(num_detected_builtins: i32, builtin_slots_names: Vec<&String>) -> HashSet<Vec<String>> {
    if num_detected_builtins == 0 {
        return HashSet::new();
    }
    let pool_size = builtin_slots_names.len() + (num_detected_builtins as usize);
    let permutations_pool: Vec<usize> = Vec::from_iter(0..pool_size);
    // Generate all permutations of indexes
    let permutations = permutations(&*permutations_pool, num_detected_builtins);
    // Replace slot indexes with slot names or OUTSIDE
    let slot_permutations = permutations
        .into_iter()
        .map(|perm|
            perm.into_iter()
                .map(|slot_index|
                    match builtin_slots_names.get(slot_index) {
                        Some(v) => v.to_string(),
                        None => OUTSIDE.to_string()
                    })
                .collect()
        );
    // Make permutations unique
    HashSet::from_iter(slot_permutations)
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;

    use nlu_utils::language::Language;
    use nlu_utils::token::tokenize;
    use super::*;

    struct Test {
        text: String,
        tags: Vec<String>,
        expected_slots: Vec<InternalSlot>,
    }

    #[test]
    fn test_io_tags_to_slots() {
        // Given
        let language = Language::EN;
        let slot_name = "animal";
        let intent_slots_mapping = hashmap!["animal".to_string() => "animal".to_string()];
        let tags: Vec<Test> = vec![
            Test {
                text: "".to_string(),
                tags: vec![],
                expected_slots: vec![],
            },
            Test {
                text: "nothing here".to_string(),
                tags: vec![
                    OUTSIDE.to_string(),
                    OUTSIDE.to_string(),
                ],
                expected_slots: vec![],
            },
            Test {
                text: "i am a blue bird".to_string(),
                tags: vec![
                    OUTSIDE.to_string(),
                    OUTSIDE.to_string(),
                    OUTSIDE.to_string(),
                    format!("{}{}", INSIDE_PREFIX, slot_name),
                    format!("{}{}", INSIDE_PREFIX, slot_name),
                ],
                expected_slots: vec![
                    InternalSlot {
                        range: 7..16,
                        value: "blue bird".to_string(),
                        entity: slot_name.to_string(),
                        slot_name: slot_name.to_string()
                    }
                ]
            },
            Test {
                text: "i am a bird".to_string(),
                tags: vec![
                    OUTSIDE.to_string(),
                    OUTSIDE.to_string(),
                    OUTSIDE.to_string(),
                    format!("{}{}", INSIDE_PREFIX, slot_name)
                ],
                expected_slots: vec![
                    InternalSlot {
                        range: 7..11,
                        value: "bird".to_string(),
                        entity: slot_name.to_string(),
                        slot_name: slot_name.to_string(),
                    }
                ]
            },
            Test {
                text: "bird".to_string(),
                tags: vec![format!("{}{}", INSIDE_PREFIX, slot_name)],
                expected_slots: vec![
                    InternalSlot {
                        range: 0..4,
                        value: "bird".to_string(),
                        entity: slot_name.to_string(),
                        slot_name: slot_name.to_string(),
                    }
                ]
            },
            Test {
                text: "blue bird".to_string(),
                tags: vec![
                    format!("{}{}", INSIDE_PREFIX, slot_name),
                    format!("{}{}", INSIDE_PREFIX, slot_name)
                ],
                expected_slots: vec![
                    InternalSlot {
                        range: 0..9,
                        value: "blue bird".to_string(),
                        entity: slot_name.to_string(),
                        slot_name: slot_name.to_string(),
                    }
                ]
            },
            Test {
                text: "light blue bird blue bird".to_string(),
                tags: vec![
                    format!("{}{}", INSIDE_PREFIX, slot_name),
                    format!("{}{}", INSIDE_PREFIX, slot_name),
                    format!("{}{}", INSIDE_PREFIX, slot_name),
                    format!("{}{}", INSIDE_PREFIX, slot_name),
                    format!("{}{}", INSIDE_PREFIX, slot_name),
                ],
                expected_slots: vec![
                    InternalSlot {
                        range: 0..25,
                        value: "light blue bird blue bird".to_string(),
                        entity: slot_name.to_string(),
                        slot_name: slot_name.to_string(),
                    }
                ]
            },
            Test {
                text: "bird birdy".to_string(),
                tags: vec![
                    format!("{}{}", INSIDE_PREFIX, slot_name),
                    format!("{}{}", INSIDE_PREFIX, slot_name),
                ],
                expected_slots: vec![
                    InternalSlot {
                        range: 0..10,
                        value: "bird birdy".to_string(),
                        entity: slot_name.to_string(),
                        slot_name: slot_name.to_string(),
                    }
                ]
            }
        ];

        for data in tags {
            // When
            let slots = tags_to_slots(&data.text,
                                      &tokenize(&data.text, &language),
                                      &data.tags,
                                      TaggingScheme::IO,
                                      &intent_slots_mapping);
            // Then
            assert_eq!(slots, data.expected_slots);
        }
    }

    #[test]
    fn test_bio_tags_to_slots() {
        // Given
        let language = Language::EN;
        let slot_name = "animal";
        let intent_slots_mapping = hashmap!["animal".to_string() => "animal".to_string()];
        let tags: Vec<Test> = vec![
            Test {
                text: "".to_string(),
                tags: vec![],
                expected_slots: vec![],
            },
            Test {
                text: "nothing here".to_string(),
                tags: vec![
                    OUTSIDE.to_string(),
                    OUTSIDE.to_string()
                ],
                expected_slots: vec![],
            },
            Test {
                text: "i am a blue bird".to_string(),
                tags: vec![
                    OUTSIDE.to_string(),
                    OUTSIDE.to_string(),
                    OUTSIDE.to_string(),
                    format!("{}{}", BEGINNING_PREFIX, slot_name),
                    format!("{}{}", INSIDE_PREFIX, slot_name)
                ],
                expected_slots: vec![
                    InternalSlot {
                        range: 7..16,
                        value: "blue bird".to_string(),
                        entity: slot_name.to_string(),
                        slot_name: slot_name.to_string(),
                    }
                ]
            },
            Test {
                text: "i am a bird".to_string(),
                tags: vec![
                    OUTSIDE.to_string(),
                    OUTSIDE.to_string(),
                    OUTSIDE.to_string(),
                    format!("{}{}", BEGINNING_PREFIX, slot_name)
                ],
                expected_slots: vec![
                    InternalSlot {
                        range: 7..11,
                        value: "bird".to_string(),
                        entity: slot_name.to_string(),
                        slot_name: slot_name.to_string(),
                    }
                ]
            },
            Test {
                text: "bird".to_string(),
                tags: vec![format!("{}{}", BEGINNING_PREFIX, slot_name)],
                expected_slots: vec![
                    InternalSlot {
                        range: 0..4,
                        value: "bird".to_string(),
                        entity: slot_name.to_string(),
                        slot_name: slot_name.to_string(),
                    }
                ]
            },
            Test {
                text: "blue bird".to_string(),
                tags: vec![
                    format!("{}{}", BEGINNING_PREFIX, slot_name),
                    format!("{}{}", INSIDE_PREFIX, slot_name)
                ],
                expected_slots: vec![
                    InternalSlot {
                        range: 0..9,
                        value: "blue bird".to_string(),
                        entity: slot_name.to_string(),
                        slot_name: slot_name.to_string(),
                    }
                ]
            },
            Test {
                text: "light blue bird blue bird".to_string(),
                tags: vec![
                    format!("{}{}", BEGINNING_PREFIX, slot_name),
                    format!("{}{}", INSIDE_PREFIX, slot_name),
                    format!("{}{}", INSIDE_PREFIX, slot_name),
                    format!("{}{}", BEGINNING_PREFIX, slot_name),
                    format!("{}{}", INSIDE_PREFIX, slot_name),
                ],
                expected_slots: vec![
                    InternalSlot {
                        range: 0..15,
                        value: "light blue bird".to_string(),
                        entity: slot_name.to_string(),
                        slot_name: slot_name.to_string(),
                    },
                    InternalSlot {
                        range: 16..25,
                        value: "blue bird".to_string(),
                        entity: slot_name.to_string(),
                        slot_name: slot_name.to_string(),
                    }
                ]
            },
            Test {
                text: "bird birdy".to_string(),
                tags: vec![
                    format!("{}{}", BEGINNING_PREFIX, slot_name),
                    format!("{}{}", BEGINNING_PREFIX, slot_name)
                ],
                expected_slots: vec![
                    InternalSlot {
                        range: 0..4,
                        value: "bird".to_string(),
                        entity: slot_name.to_string(),
                        slot_name: slot_name.to_string(),
                    },
                    InternalSlot {
                        range: 5..10,
                        value: "birdy".to_string(),
                        entity: slot_name.to_string(),
                        slot_name: slot_name.to_string(),
                    }
                ]
            },
            Test {
                text: "blue bird and white bird".to_string(),
                tags: vec![
                    format!("{}{}", BEGINNING_PREFIX, slot_name),
                    format!("{}{}", INSIDE_PREFIX, slot_name),
                    OUTSIDE.to_string(),
                    format!("{}{}", INSIDE_PREFIX, slot_name),
                    format!("{}{}", INSIDE_PREFIX, slot_name),
                ],
                expected_slots: vec![
                    InternalSlot {
                        range: 0..9,
                        value: "blue bird".to_string(),
                        entity: slot_name.to_string(),
                        slot_name: slot_name.to_string(),
                    },
                    InternalSlot {
                        range: 14..24,
                        value: "white bird".to_string(),
                        entity: slot_name.to_string(),
                        slot_name: slot_name.to_string(),
                    }
                ]
            }
        ];

        for data in tags {
            // When
            let slots = tags_to_slots(&data.text,
                                      &tokenize(&data.text, &language),
                                      &data.tags,
                                      TaggingScheme::BIO,
                                      &intent_slots_mapping);
            // Then
            assert_eq!(slots, data.expected_slots);
        }
    }

    #[test]
    fn test_bilou_tags_to_slots() {
        // Given
        let language = Language::EN;
        let slot_name = "animal";
        let intent_slots_mapping = hashmap!["animal".to_string() => "animal".to_string()];
        let tags: Vec<Test> = vec![
            Test {
                text: "".to_string(),
                tags: vec![],
                expected_slots: vec![],
            },
            Test {
                text: "nothing here".to_string(),
                tags: vec![
                    OUTSIDE.to_string(),
                    OUTSIDE.to_string()
                ],
                expected_slots: vec![],
            },
            Test {
                text: "i am a blue bird".to_string(),
                tags: vec![
                    OUTSIDE.to_string(),
                    OUTSIDE.to_string(),
                    OUTSIDE.to_string(),
                    format!("{}{}", BEGINNING_PREFIX, slot_name),
                    format!("{}{}", LAST_PREFIX, slot_name)
                ],
                expected_slots: vec![
                    InternalSlot {
                        range: 7..16,
                        value: "blue bird".to_string(),
                        entity: slot_name.to_string(),
                        slot_name: slot_name.to_string(),
                    }
                ]
            },
            Test {
                text: "i am a bird".to_string(),
                tags: vec![
                    OUTSIDE.to_string(),
                    OUTSIDE.to_string(),
                    OUTSIDE.to_string(),
                    format!("{}{}", UNIT_PREFIX, slot_name)
                ],
                expected_slots: vec![
                    InternalSlot {
                        range: 7..11,
                        value: "bird".to_string(),
                        entity: slot_name.to_string(),
                        slot_name: slot_name.to_string(),
                    }
                ]
            },
            Test {
                text: "bird".to_string(),
                tags: vec![format!("{}{}", UNIT_PREFIX, slot_name)],
                expected_slots: vec![
                    InternalSlot {
                        range: 0..4,
                        value: "bird".to_string(),
                        entity: slot_name.to_string(),
                        slot_name: slot_name.to_string(),
                    }
                ]
            },
            Test {
                text: "blue bird".to_string(),
                tags: vec![
                    format!("{}{}", BEGINNING_PREFIX, slot_name),
                    format!("{}{}", LAST_PREFIX, slot_name)
                ],
                expected_slots: vec![
                    InternalSlot {
                        range: 0..9,
                        value: "blue bird".to_string(),
                        entity: slot_name.to_string(),
                        slot_name: slot_name.to_string(),
                    }
                ]
            },
            Test {
                text: "light blue bird blue bird".to_string(),
                tags: vec![
                    format!("{}{}", BEGINNING_PREFIX, slot_name),
                    format!("{}{}", INSIDE_PREFIX, slot_name),
                    format!("{}{}", LAST_PREFIX, slot_name),
                    format!("{}{}", BEGINNING_PREFIX, slot_name),
                    format!("{}{}", LAST_PREFIX, slot_name),
                ],
                expected_slots: vec![
                    InternalSlot {
                        range: 0..15,
                        value: "light blue bird".to_string(),
                        entity: slot_name.to_string(),
                        slot_name: slot_name.to_string(),
                    },
                    InternalSlot {
                        range: 16..25,
                        value: "blue bird".to_string(),
                        entity: slot_name.to_string(),
                        slot_name: slot_name.to_string(),
                    }
                ]
            },
            Test {
                text: "bird birdy".to_string(),
                tags: vec![
                    format!("{}{}", UNIT_PREFIX, slot_name),
                    format!("{}{}", UNIT_PREFIX, slot_name)
                ],
                expected_slots: vec![
                    InternalSlot {
                        range: 0..4,
                        value: "bird".to_string(),
                        entity: slot_name.to_string(),
                        slot_name: slot_name.to_string(),
                    },
                    InternalSlot {
                        range: 5..10,
                        value: "birdy".to_string(),
                        entity: slot_name.to_string(),
                        slot_name: slot_name.to_string(),
                    }
                ]
            },
            Test {
                text: "light bird bird blue bird".to_string(),
                tags: vec![
                    format!("{}{}", BEGINNING_PREFIX, slot_name),
                    format!("{}{}", INSIDE_PREFIX, slot_name),
                    format!("{}{}", UNIT_PREFIX, slot_name),
                    format!("{}{}", BEGINNING_PREFIX, slot_name),
                    format!("{}{}", INSIDE_PREFIX, slot_name),
                ],
                expected_slots: vec![
                    InternalSlot {
                        range: 0..10,
                        value: "light bird".to_string(),
                        entity: slot_name.to_string(),
                        slot_name: slot_name.to_string(),
                    },
                    InternalSlot {
                        range: 11..15,
                        value: "bird".to_string(),
                        entity: slot_name.to_string(),
                        slot_name: slot_name.to_string(),
                    },
                    InternalSlot {
                        range: 16..25,
                        value: "blue bird".to_string(),
                        entity: slot_name.to_string(),
                        slot_name: slot_name.to_string(),
                    }
                ]
            },
            Test {
                text: "bird bird bird".to_string(),
                tags: vec![
                    format!("{}{}", LAST_PREFIX, slot_name),
                    format!("{}{}", BEGINNING_PREFIX, slot_name),
                    format!("{}{}", UNIT_PREFIX, slot_name),
                ],
                expected_slots: vec![
                    InternalSlot {
                        range: 0..4,
                        value: "bird".to_string(),
                        entity: slot_name.to_string(),
                        slot_name: slot_name.to_string(),
                    },
                    InternalSlot {
                        range: 5..9,
                        value: "bird".to_string(),
                        entity: slot_name.to_string(),
                        slot_name: slot_name.to_string(),
                    },
                    InternalSlot {
                        range: 10..14,
                        value: "bird".to_string(),
                        entity: slot_name.to_string(),
                        slot_name: slot_name.to_string(),
                    }
                ]
            }
        ];

        for data in tags {
            // When
            let slots = tags_to_slots(&data.text,
                                      &tokenize(&data.text, &language),
                                      &data.tags,
                                      TaggingScheme::BILOU,
                                      &intent_slots_mapping);
            // Then
            assert_eq!(slots, data.expected_slots);
        }
    }

    #[test]
    fn test_is_start_of_bio_slot() {
        // Given
        let tags = &[
            OUTSIDE.to_string(),
            BEGINNING_PREFIX.to_string(),
            INSIDE_PREFIX.to_string(),
            OUTSIDE.to_string(),
            INSIDE_PREFIX.to_string(),
            OUTSIDE.to_string(),
            BEGINNING_PREFIX.to_string(),
            OUTSIDE.to_string(),
            INSIDE_PREFIX.to_string(),
            BEGINNING_PREFIX.to_string(),
            OUTSIDE.to_string(),
            BEGINNING_PREFIX.to_string(),
            BEGINNING_PREFIX.to_string(),
            INSIDE_PREFIX.to_string(),
            INSIDE_PREFIX.to_string(),
        ];

        // When
        let starts_of_bio = tags.iter()
            .enumerate()
            .map(|(i, _)| is_start_of_bio_slot(tags, i))
            .collect_vec();

        // Then
        let expected_starts = [
            false,
            true,
            false,
            false,
            true,
            false,
            true,
            false,
            true,
            true,
            false,
            true,
            true,
            false,
            false
        ];

        assert_eq!(starts_of_bio, expected_starts);
    }

    #[test]
    fn test_is_end_of_bio_slot() {
        // Given
        let tags = &[
            OUTSIDE.to_string(),
            BEGINNING_PREFIX.to_string(),
            INSIDE_PREFIX.to_string(),
            OUTSIDE.to_string(),
            INSIDE_PREFIX.to_string(),
            OUTSIDE.to_string(),
            BEGINNING_PREFIX.to_string(),
            OUTSIDE.to_string(),
            INSIDE_PREFIX.to_string(),
            BEGINNING_PREFIX.to_string(),
            OUTSIDE.to_string(),
            BEGINNING_PREFIX.to_string(),
            BEGINNING_PREFIX.to_string(),
            INSIDE_PREFIX.to_string(),
            INSIDE_PREFIX.to_string(),
        ];

        // When
        let ends_of_bio = tags.iter()
            .enumerate()
            .map(|(i, _)| is_end_of_bio_slot(tags, i))
            .collect_vec();

        // Then
        let expected_ends = [
            false,
            false,
            true,
            false,
            true,
            false,
            true,
            false,
            true,
            true,
            false,
            true,
            false,
            false,
            true
        ];

        assert_eq!(ends_of_bio, expected_ends);
    }

    #[test]
    fn test_start_of_bilou_slot() {
        // Given
        let tags = &[
            OUTSIDE.to_string(),
            LAST_PREFIX.to_string(),
            UNIT_PREFIX.to_string(),
            BEGINNING_PREFIX.to_string(),
            UNIT_PREFIX.to_string(),
            INSIDE_PREFIX.to_string(),
            LAST_PREFIX.to_string(),
            LAST_PREFIX.to_string(),
            UNIT_PREFIX.to_string(),
            UNIT_PREFIX.to_string(),
            LAST_PREFIX.to_string(),
            OUTSIDE.to_string(),
            LAST_PREFIX.to_string(),
            BEGINNING_PREFIX.to_string(),
            INSIDE_PREFIX.to_string(),
            INSIDE_PREFIX.to_string(),
            LAST_PREFIX.to_string(),
        ];

        // When
        let starts_of_bilou = tags.iter()
            .enumerate()
            .map(|(i, _)| is_start_of_bilou_slot(tags, i))
            .collect_vec();

        // Then
        let expected_starts = [
            false,
            true,
            true,
            true,
            true,
            true,
            false,
            true,
            true,
            true,
            true,
            false,
            true,
            true,
            false,
            false,
            false
        ];

        assert_eq!(starts_of_bilou, expected_starts);
    }

    #[test]
    fn test_is_end_of_bilou_slot() {
        // Given
        let tags = &[
            OUTSIDE.to_string(),
            LAST_PREFIX.to_string(),
            UNIT_PREFIX.to_string(),
            BEGINNING_PREFIX.to_string(),
            UNIT_PREFIX.to_string(),
            INSIDE_PREFIX.to_string(),
            LAST_PREFIX.to_string(),
            LAST_PREFIX.to_string(),
            UNIT_PREFIX.to_string(),
            UNIT_PREFIX.to_string(),
            LAST_PREFIX.to_string(),
            OUTSIDE.to_string(),
            INSIDE_PREFIX.to_string(),
            BEGINNING_PREFIX.to_string(),
            OUTSIDE.to_string(),
            BEGINNING_PREFIX.to_string(),
            INSIDE_PREFIX.to_string(),
            INSIDE_PREFIX.to_string(),
            LAST_PREFIX.to_string(),
        ];

        // When
        let ends_of_bilou = tags.iter()
            .enumerate()
            .map(|(i, _)| is_end_of_bilou_slot(tags, i))
            .collect_vec();

        // Then
        let expected_ends = [
            false,
            true,
            true,
            true,
            true,
            false,
            true,
            true,
            true,
            true,
            true,
            false,
            true,
            true,
            false,
            false,
            false,
            false,
            true
        ];

        assert_eq!(ends_of_bilou, expected_ends);
    }

    #[test]
    fn get_scheme_prefix_works() {
        // Given
        let indexes = vec![3, 4, 5];

        // When
        let actual_results = vec![
            get_scheme_prefix(5, &indexes, TaggingScheme::IO).to_string(),
            get_scheme_prefix(3, &indexes, TaggingScheme::BIO).to_string(),
            get_scheme_prefix(4, &indexes, TaggingScheme::BIO).to_string(),
            get_scheme_prefix(3, &indexes, TaggingScheme::BILOU).to_string(),
            get_scheme_prefix(4, &indexes, TaggingScheme::BILOU).to_string(),
            get_scheme_prefix(5, &indexes, TaggingScheme::BILOU).to_string(),
            get_scheme_prefix(1, &vec![1], TaggingScheme::BILOU).to_string(),
        ];

        // Then
        let expected_results = vec![
            "I-".to_string(),
            "B-".to_string(),
            "I-".to_string(),
            "B-".to_string(),
            "I-".to_string(),
            "L-".to_string(),
            "U-".to_string(),
        ];
        assert_eq!(actual_results, expected_results);
    }

    #[test]
    fn test_positive_tagging_with_io() {
        // Given
        let tagging_scheme = TaggingScheme::IO;
        let slot_name = "animal";
        let slot_size = 3;

        // When
        let tags = positive_tagging(tagging_scheme, slot_name, slot_size);

        // Then
        let t = format!("{}{}", INSIDE_PREFIX, slot_name);
        let expected_tags = vec![t; 3];
        assert_eq!(tags, expected_tags);
    }

    #[test]
    fn test_positive_tagging_with_bio() {
        // Given
        let tagging_scheme = TaggingScheme::BIO;
        let slot_name = "animal";
        let slot_size = 3;

        // When
        let tags = positive_tagging(tagging_scheme, slot_name, slot_size);

        // Then
        let expected_tags = vec![
            format!("{}{}", BEGINNING_PREFIX, slot_name),
            format!("{}{}", INSIDE_PREFIX, slot_name),
            format!("{}{}", INSIDE_PREFIX, slot_name),
        ];
        assert_eq!(tags, expected_tags);
    }

    #[test]
    fn test_positive_tagging_with_bilou() {
        // Given
        let tagging_scheme = TaggingScheme::BILOU;
        let slot_name = "animal";
        let slot_size = 3;

        // When
        let tags = positive_tagging(tagging_scheme, slot_name, slot_size);

        // Then
        let expected_tags = vec![
            format!("{}{}", BEGINNING_PREFIX, slot_name),
            format!("{}{}", INSIDE_PREFIX, slot_name),
            format!("{}{}", LAST_PREFIX, slot_name),
        ];
        assert_eq!(tags, expected_tags);
    }

    #[test]
    fn test_positive_tagging_with_bilou_unit() {
        // Given
        let tagging_scheme = TaggingScheme::BILOU;
        let slot_name = "animal";
        let slot_size = 1;

        // When
        let tags = positive_tagging(tagging_scheme, slot_name, slot_size);

        // Then
        let expected_tags = vec![format!("{}{}", UNIT_PREFIX, slot_name)];
        assert_eq!(tags, expected_tags);
    }

    #[test]
    fn generate_slots_permutations_works() {
        // Given
        let s_1 = "slot1".to_string();
        let s_2 = "slot2".to_string();
        let builtin_slot_names = vec![&s_1, &s_2];
        let n_builtin_slot_in_sentences = 3;

        // When
        let slot_names_permutations = generate_slots_permutations(n_builtin_slot_in_sentences, builtin_slot_names);

        // Then
        let expected_slot_names_permutations = hashset![
            vec!["slot1".to_string(), "slot2".to_string(), OUTSIDE.to_string()],
            vec!["slot2".to_string(), "slot1".to_string(), OUTSIDE.to_string()],
            vec!["slot2".to_string(), OUTSIDE.to_string(), "slot1".to_string()],
            vec!["slot1".to_string(), OUTSIDE.to_string(), "slot2".to_string()],
            vec!["O".to_string(), "slot2".to_string(), "slot1".to_string()],
            vec!["O".to_string(), "slot1".to_string(), "slot2".to_string()],
            vec!["O".to_string(), OUTSIDE.to_string(), "slot1".to_string()],
            vec!["O".to_string(), OUTSIDE.to_string(), "slot2".to_string()],
            vec!["O".to_string(), "slot1".to_string(), OUTSIDE.to_string()],
            vec!["O".to_string(), "slot2".to_string(), OUTSIDE.to_string()],
            vec!["slot1".to_string(), OUTSIDE.to_string(), OUTSIDE.to_string()],
            vec!["slot2".to_string(), OUTSIDE.to_string(), OUTSIDE.to_string()],
            vec!["O".to_string(), OUTSIDE.to_string(), OUTSIDE.to_string()]
        ];
        assert_eq!(slot_names_permutations, expected_slot_names_permutations)
    }
}
