use crate::{candidate::Candidate, text_matcher::TextMatcher};

use regex::Regex;
use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

const SENSITIVITY: f64 = 0.6;
const FILE_SENSITIVITY: f64 = 0.87;
const PLACE_SEARCH_SENSITIVITY: f64 = 0.7;
const KEEP: usize = 50;
const PATH_TO_PLACES: &str = "./test_data/places.txt";
const PATH_TO_PLZS_DIR: &str = "./test_data/plzs/";
const PATH_TO_PLACES_DIR: &str = "./test_data/places/";

#[derive(Debug)]
pub struct MatchedStreet {
    pub street: Option<String>,
    pub file_found: Option<PathBuf>,
}

fn does_start_with_number(street: &str) -> bool {
    Regex::new(r"^\d+,?\s.+").unwrap().is_match(street)
        || Regex::new(r"^\w\d+,?\s").unwrap().is_match(street)
}

fn clean_street(street: &str) -> String {
    let mut street = street.trim().to_string().to_lowercase();
    // Matches: '76 chemin des clos' or 'a4 résidence du golf'
    if does_start_with_number(&street) {
        let mut parts = street.split_whitespace();
        let number = parts
            .next()
            .expect("the string format is correct due to regex");
        street = format!("{} {}", parts.collect::<Vec<&str>>().join(" "), number);
    }
    // TODO: add punctuation removing after the next regex
    // Matches: eisfeldstrasse 21/23, milchstrasse 2-10a, milchstrasse 2,10a, bernstrasse 7 8
    match Regex::new(r"(.*?\s\d*?\s?\w?)[/,\-\s]")
        .unwrap()
        .find(&street)
    {
        // but not bernstrasse 7 A
        Some(mat) if !Regex::new(r"\s\d+[/,\-\s]\w$").unwrap().is_match(&street) => mat.as_str(),
        _ => street.as_str(),
    }
    .trim()
    .to_string()
}

fn does_contain_numbers(street: &str) -> bool {
    street.chars().map(char::is_numeric).count() > 0
}

pub struct StreetMatcher {
    pub street: String,
    pub sensitivity: f64,
    pub file_sensitivity: f64,
}

impl StreetMatcher {
    /// StreetMatcher constructor gives possibility to finetune the matching process
    /// by setting custom sensitivity and file_sensitivity values (each from 0.0 - keep all matches to 1.0 - keep only direct matches).
    /// However, it's recommended to keep default values, i.e. sensitivity == 0.6 - dir seacrh, and file_sensitivity == 0.87 - file search
    ///
    /// # Panics
    ///
    /// Panics if `street` does not contain a number (as each valid street MUST contain an any number)
    pub fn new(street: &str, sensitivity: Option<f64>, file_sensitivity: Option<f64>) -> Self {
        if !does_contain_numbers(street) {
            panic!(
                "Argument 'street' must contain street number! Got: '{}'",
                street
            );
        }
        Self {
            street: clean_street(street),
            sensitivity: sensitivity.unwrap_or(SENSITIVITY),
            file_sensitivity: file_sensitivity.unwrap_or(FILE_SENSITIVITY),
        }
    }

    fn _find_matches_in_dir(&self, dir: &Path, is_first_letters_eq: bool) -> Vec<Candidate> {
        TextMatcher::find_matches_in_dir(
            self.sensitivity,
            KEEP,
            &self.street,
            dir.to_path_buf(),
            None,
            is_first_letters_eq,
        )
    }

    fn _search_in_dir(&self, dir: &Path, file_candidate: Option<PathBuf>) -> MatchedStreet {
        let mut mat = self._find_matches_in_dir(dir, true);
        if mat.is_empty() {
            mat = self._find_matches_in_dir(dir, false);
        }
        let best_match = if !mat.is_empty() {
            Some(mat[0].text.clone())
        } else {
            None
        };
        MatchedStreet {
            street: best_match.clone(),
            file_found: file_candidate.and_then(|file| {
                if mat
                    .iter()
                    .filter(|candidate| {
                        candidate.file_found == file
                            && best_match
                                .as_ref()
                                .map_or(false, |street| candidate.text == *street)
                    })
                    .count()
                    > 0
                {
                    Some(file)
                } else {
                    None
                }
            }),
        }
    }

    fn _find_matches(&self, dir: &Path, file: Option<PathBuf>) -> MatchedStreet {
        file.map_or_else(
            || self._search_in_dir(dir, None),
            |file| match TextMatcher::new(self.file_sensitivity, KEEP, false)
                .find_matches_in_file(&self.street, &file)
            {
                Ok(mat) if !mat.is_empty() => MatchedStreet {
                    street: Some(mat[0].text.clone()),
                    file_found: Some(file),
                },
                _ => self._search_in_dir(dir, Some(file)),
            },
        )
    }

    /// Search for a candidate street(s) to a target street within a Postal Code (`plz`).
    /// All official street candidates here grouped into files named by a Postal Code.
    /// `plz` must be a valid Switzerland Postal Code represented officially by government.
    /// Otherwise, if `plz` did not match any of existings Postal Codes in the directory,
    /// the search on the WHOLE directory (all files inside a directory) is provided.
    /// Also, if a candidate was not found within a given `plz`, the same logic (search on all files) is applied.
    ///
    /// ```rust
    /// # use text_matcher_rs::StreetMatcher;
    /// #
    /// # fn main() {
    /// #     let sm = StreetMatcher::new("qu du seujet 36", None, None);
    /// #     assert_eq!(sm.match_by_plz(Some(1201)).street.unwrap(), "quai du seujet 36".to_string());
    /// #}
    /// ```
    pub fn match_by_plz(&self, plz: Option<usize>) -> MatchedStreet {
        self._find_matches(
            &PathBuf::from(PATH_TO_PLZS_DIR),
            plz.map(|plz| PathBuf::from(format!("{}{}", PATH_TO_PLZS_DIR, plz))),
        )
    }

    /// Search for a candidate street(s) to a target street within a Swiss peace of territory, assigned to the Postal Code (called `place`).
    /// All official street candidates here grouped into files named by `place`.
    /// `place` could be an invalid name. In this case, the matcher will try to search for `place` candidate inside a `places.txt` file.
    /// If `place` did not match any of existings Postal Codes in the directory,
    /// the search on the WHOLE directory (all files inside a directory) is provided.
    /// Also, if a candidate was not found within a given `plz`, the same logic (search on all files) is applied.
    ///
    /// ```rust
    /// # use text_matcher_rs::StreetMatcher;
    /// #
    /// # fn main() {
    /// #     let sm = StreetMatcher::new("aarstr. 76", None, None);
    /// #     assert_eq!(sm.match_by_place(Some("Bern")).street.unwrap(), "aarstrasse 76".to_string());
    /// #}
    /// ```
    pub fn match_by_place(&self, place: Option<&str>) -> MatchedStreet {
        self._find_matches(
            &PathBuf::from(PATH_TO_PLACES_DIR),
            place.and_then(|place| {
                StreetMatcher::_match_place(place).map(|candidate| {
                    PathBuf::from(format!("{}{}", PATH_TO_PLACES_DIR, candidate.text))
                })
            }),
        )
    }

    fn _match_place(place: &str) -> Option<Candidate> {
        // FIXME: add cast to lowercase here
        let ms = TextMatcher::new(PLACE_SEARCH_SENSITIVITY, KEEP, false).find_matches_in_file(
            place,
            &PathBuf::from_str(PATH_TO_PLACES).expect("places.txt file exists"),
        );
        if let Ok(candidates) = ms {
            if !candidates.is_empty() {
                return Some(candidates[0].clone());
            }
        }
        None
    }
}
