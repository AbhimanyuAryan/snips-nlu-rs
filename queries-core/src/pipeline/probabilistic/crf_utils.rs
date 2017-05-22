use std::ops::Range;
use std::collections::HashMap;

use yolo::Yolo;

use pipeline::SlotValue;
use preprocessing::Token;
use utils::convert_char_range;

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
    fn all() -> Vec<TaggingScheme> {
        vec![TaggingScheme::IO, TaggingScheme::BIO, TaggingScheme::BILOU]
    }
}

fn tag_name_to_slot_name(tag: &str) -> &str {
    &tag[2..]
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
                           slot_name: tag_name_to_slot_name(tag).to_string(),
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
                     -> Vec<(String, SlotValue)> {
    let slots = match tagging_scheme {
        TaggingScheme::IO => _tags_to_slots(tags, tokens, is_start_of_io_slot, is_end_of_io_slot),
        TaggingScheme::BIO => _tags_to_slots(tags, tokens, is_start_of_bio_slot, is_end_of_bio_slot),
        TaggingScheme::BILOU => _tags_to_slots(tags, tokens, is_start_of_bilou_slot, is_end_of_bilou_slot),
    };

    slots
        .into_iter()
        .map(|s| {
            let slot_value = SlotValue {
                range: convert_char_range(text, &s.range),
                value: text[s.range].to_string(),
                entity: intent_slots_mapping[&s.slot_name].to_string(),
            };

            (s.slot_name, slot_value)
        })
        .collect()
}

fn positive_tagging(tagging_scheme: TaggingScheme, slot_name: &str, slot_size: usize) -> Vec<String> {
    match tagging_scheme {
        TaggingScheme::IO => {
            vec![format!("{}{}", INSIDE_PREFIX, slot_name); slot_size]
        },
        TaggingScheme::BIO => {
            if slot_size > 0 {
                let mut v1 = vec![format!("{}{}", BEGINNING_PREFIX, slot_name)];
                let mut v2 = vec![format!("{}{}", INSIDE_PREFIX, slot_name); slot_size - 1];
                v1.append(&mut v2);
                v1
            } else {
                vec![]
            }
        },
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

fn negative_tagging(size: usize) -> Vec<String> {
    vec![OUTSIDE.to_string(); size]
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
        },
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

mod tests {
    use itertools::Itertools;

    use preprocessing::light::tokenize;
    use super::*;

    struct Test {
        text: String,
        tags: Vec<String>,
        expected_slots: Vec<(String, SlotValue)>,
    }

    #[test]
    fn test_io_tags_to_slots() {
        // Given
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
                    (slot_name.to_string(),
                    SlotValue {
                        range: 7..16,
                        value: "blue bird".to_string(),
                        entity: slot_name.to_string(),
                    })
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
                    (slot_name.to_string(),
                    SlotValue {
                        range: 7..11,
                        value: "bird".to_string(),
                        entity: slot_name.to_string(),
                    })
                ]
            },
            Test {
                text: "bird".to_string(),
                tags: vec![format!("{}{}", INSIDE_PREFIX, slot_name)],
                expected_slots: vec![
                    (slot_name.to_string(),
                    SlotValue {
                        range: 0..4,
                        value: "bird".to_string(),
                        entity: slot_name.to_string(),
                    })
                ]
            },
            Test {
                text: "blue bird".to_string(),
                tags: vec![
                    format!("{}{}", INSIDE_PREFIX, slot_name),
                    format!("{}{}", INSIDE_PREFIX, slot_name)
                ],
                expected_slots: vec![
                    (slot_name.to_string(),
                    SlotValue {
                        range: 0..9,
                        value: "blue bird".to_string(),
                        entity: slot_name.to_string(),
                    })
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
                    (slot_name.to_string(),
                    SlotValue {
                        range: 0..25,
                        value: "light blue bird blue bird".to_string(),
                        entity: slot_name.to_string(),
                    })
                ]
            },
            Test {
                text: "bird birdy".to_string(),
                tags: vec![
                    format!("{}{}", INSIDE_PREFIX, slot_name),
                    format!("{}{}", INSIDE_PREFIX, slot_name),
                ],
                expected_slots: vec![
                    (slot_name.to_string(),
                    SlotValue {
                        range: 0..10,
                        value: "bird birdy".to_string(),
                        entity: slot_name.to_string(),
                    })
                ]
            }
        ];

        for data in tags {
            // When
            let slots = tags_to_slots(&data.text,
                                      &tokenize(&data.text),
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
                    (slot_name.to_string(),
                    SlotValue {
                        range: 7..16,
                        value: "blue bird".to_string(),
                        entity: slot_name.to_string(),
                    })
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
                    (slot_name.to_string(),
                    SlotValue {
                        range: 7..11,
                        value: "bird".to_string(),
                        entity: slot_name.to_string(),
                    })
                ]
            },
            Test {
                text: "bird".to_string(),
                tags: vec![format!("{}{}", BEGINNING_PREFIX, slot_name)],
                expected_slots: vec![
                    (slot_name.to_string(),
                    SlotValue {
                        range: 0..4,
                        value: "bird".to_string(),
                        entity: slot_name.to_string(),
                    })
                ]
            },
            Test {
                text: "blue bird".to_string(),
                tags: vec![
                    format!("{}{}", BEGINNING_PREFIX, slot_name),
                    format!("{}{}", INSIDE_PREFIX, slot_name)
                ],
                expected_slots: vec![
                    (slot_name.to_string(),
                    SlotValue {
                        range: 0..9,
                        value: "blue bird".to_string(),
                        entity: slot_name.to_string(),
                    })
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
                    (slot_name.to_string(),
                    SlotValue {
                        range: 0..15,
                        value: "light blue bird".to_string(),
                        entity: slot_name.to_string(),
                    }),
                    (slot_name.to_string(),
                    SlotValue {
                        range: 16..25,
                        value: "blue bird".to_string(),
                        entity: slot_name.to_string(),
                    })
                ]
            },
            Test {
                text: "bird birdy".to_string(),
                tags: vec![
                    format!("{}{}", BEGINNING_PREFIX, slot_name),
                    format!("{}{}", BEGINNING_PREFIX, slot_name)
                ],
                expected_slots: vec![
                    (slot_name.to_string(),
                    SlotValue {
                        range: 0..4,
                        value: "bird".to_string(),
                        entity: slot_name.to_string(),
                    }),
                    (slot_name.to_string(),
                    SlotValue {
                        range: 5..10,
                        value: "birdy".to_string(),
                        entity: slot_name.to_string(),
                    })
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
                    (slot_name.to_string(),
                    SlotValue {
                        range: 0..9,
                        value: "blue bird".to_string(),
                        entity: slot_name.to_string(),
                    }),
                    (slot_name.to_string(),
                    SlotValue {
                        range: 14..24,
                        value: "white bird".to_string(),
                        entity: slot_name.to_string(),
                    })
                ]
            }
        ];

        for data in tags {
            // When
            let slots = tags_to_slots(&data.text,
                                      &tokenize(&data.text),
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
                    (slot_name.to_string(),
                    SlotValue {
                        range: 7..16,
                        value: "blue bird".to_string(),
                        entity: slot_name.to_string(),
                    })
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
                    (slot_name.to_string(),
                    SlotValue {
                        range: 7..11,
                        value: "bird".to_string(),
                        entity: slot_name.to_string(),
                    })
                ]
            },
            Test {
                text: "bird".to_string(),
                tags: vec![format!("{}{}", UNIT_PREFIX, slot_name)],
                expected_slots: vec![
                    (slot_name.to_string(),
                    SlotValue {
                        range: 0..4,
                        value: "bird".to_string(),
                        entity: slot_name.to_string(),
                    })
                ]
            },
            Test {
                text: "blue bird".to_string(),
                tags: vec![
                    format!("{}{}", BEGINNING_PREFIX, slot_name),
                    format!("{}{}", LAST_PREFIX, slot_name)
                ],
                expected_slots: vec![
                    (slot_name.to_string(),
                    SlotValue {
                        range: 0..9,
                        value: "blue bird".to_string(),
                        entity: slot_name.to_string(),
                    })
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
                    (slot_name.to_string(),
                    SlotValue {
                        range: 0..15,
                        value: "light blue bird".to_string(),
                        entity: slot_name.to_string(),
                    }),
                    (slot_name.to_string(),
                    SlotValue {
                        range: 16..25,
                        value: "blue bird".to_string(),
                        entity: slot_name.to_string(),
                    })
                ]
            },
            Test {
                text: "bird birdy".to_string(),
                tags: vec![
                    format!("{}{}", UNIT_PREFIX, slot_name),
                    format!("{}{}", UNIT_PREFIX, slot_name)
                ],
                expected_slots: vec![
                    (slot_name.to_string(),
                    SlotValue {
                        range: 0..4,
                        value: "bird".to_string(),
                        entity: slot_name.to_string(),
                    }),
                    (slot_name.to_string(),
                    SlotValue {
                        range: 5..10,
                        value: "birdy".to_string(),
                        entity: slot_name.to_string(),
                    })
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
                    (slot_name.to_string(),
                    SlotValue {
                        range: 0..10,
                        value: "light bird".to_string(),
                        entity: slot_name.to_string(),
                    }),
                    (slot_name.to_string(),
                    SlotValue {
                        range: 11..15,
                        value: "bird".to_string(),
                        entity: slot_name.to_string(),
                    }),
                    (slot_name.to_string(),
                    SlotValue {
                        range: 16..25,
                        value: "blue bird".to_string(),
                        entity: slot_name.to_string(),
                    })
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
                    (slot_name.to_string(),
                    SlotValue {
                        range: 0..4,
                        value: "bird".to_string(),
                        entity: slot_name.to_string(),
                    }),
                    (slot_name.to_string(),
                    SlotValue {
                        range: 5..9,
                        value: "bird".to_string(),
                        entity: slot_name.to_string(),
                    }),
                    (slot_name.to_string(),
                    SlotValue {
                        range: 10..14,
                        value: "bird".to_string(),
                        entity: slot_name.to_string(),
                    })
                ]
            }
        ];

        for data in tags {
            // When
            let slots = tags_to_slots(&data.text,
                                      &tokenize(&data.text),
                                      &data.tags,
                                      TaggingScheme::BILOU,
                                      &intent_slots_mapping);
            // Then
            assert_eq!(slots, data.expected_slots);
        }
    }

    #[test]
    fn test_positive_tagging_should_handle_zero_length() {
        // Given
        let slot_name = "animal";
        let slot_size = 0;

        // When
        let mut tags = vec![];
        for scheme in TaggingScheme::all() {
            tags.push(positive_tagging(scheme, slot_name, slot_size));
        }

        // Then
        let expected_tags: Vec<Vec<String>> = vec![vec![]; TaggingScheme::all().len()];
        assert_eq!(tags, expected_tags);
    }

    #[test]
    fn test_negative_tagging() {
        // Given
        let size = 3;

        // When
        let tagging = negative_tagging(size);

        // Then
        let expected_tagging = [OUTSIDE, OUTSIDE, OUTSIDE];
        assert_eq!(tagging, expected_tagging);
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
}
