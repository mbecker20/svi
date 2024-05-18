use std::collections::{HashMap, HashSet, VecDeque};

use thiserror::Error;

/// Choose which symbol to use as the interpolator.
#[derive(Clone, Copy)]
pub enum Interpolator {
  /// Use '{{' + '}}' as the interpolator.
  DoubleCurlyBrackets,
  /// Use '[[' + ']]' as the interpolator.
  DoubleBrackets,
}

/// Takes an input string containing variables for interpolation,
/// and a map of variables to values, and interpolates the values
/// into the resulting string.
///
/// Returns: svi::Result<(`resulting string`, `replacers`)>.
///
/// - `resulting string`: The string with variables interpolated in.
/// - `replacers`: Some values should remain secret. Replacers can be used with
/// [replace_in_string] to hide the values in the resulting string with placeholders.
pub fn interpolate_variables(
  input: &str,
  variables: &HashMap<String, String>,
  interpolator: Interpolator,
  fail_on_missing_variable: bool,
) -> Result<(String, Vec<(String, String)>)> {
  let mut result = String::new();
  let mut replacers = HashSet::new();

  let (double_opener, single_opener, triple_closer, double_closer) = match interpolator {
    Interpolator::DoubleCurlyBrackets => ("{{", "{", "}}}", "}}"),
    Interpolator::DoubleBrackets => ("[[", "[", "]]]", "]]"),
  };

  // Split the input by double opener '{{' or '[['
  let mut open_split = input.split(double_opener).collect::<VecDeque<_>>();

  // The first value in the split will be before the first variable. Push it to the result.
  let first = open_split.pop_front().ok_or(SviError::SplitEmpty)?;
  result.push_str(first);

  // Iterate through the rest of the splits.
  // At this point, the input 'mongodb://[[MONGO_USERNAME]]:[[MONGO_PASSWORD]]@localhost:27017'
  // would have split looking like (keep in mind the beginning is already popped off):
  // ["MONGO_USERNAME]]:", "MONGO_PASSWORD]]@localhost:27017"].
  for (i, val) in open_split.iter().enumerate() {
    // Check if the input uses a disallowed 'double opener'.
    // '{{{{' or '[[[['.
    if val.get(0..1).is_none() {
      return Err(SviError::FoundDoubleOpener {
        double_opener: double_opener.to_string(),
      });
    }

    // Checks if the split starts with '{' or '[', this is a triple opener.
    // This escapes interpolation and '[[[dont_replace]]]' becomes '[[dont_replace]]'. (you can already use '[dont_replace] just fine')
    if &val[0..1] == single_opener {
      // push the initial '{' or '['
      result.push_str(single_opener);
      // split the rest of the value around the closing triple brackets
      let close_split = val.split(triple_closer).collect::<Vec<_>>();
      // push the parts of the split
      for i in 0..close_split.len() {
        result.push_str(close_split[i]);
        // after the first item in split (the inside of brackets), push the closing '}}' or ']]'.
        if i == 0 && close_split.len() > 1 {
          result.push_str(double_closer);
        }
      }
    } else {
      // split the value around the closing brackets '}}' or ']]'
      let close_split = val.split(double_closer).collect::<Vec<_>>();

      // a split with length <= 1 means a matching closer is not present for the opener
      if close_split.len() <= 1 {
        return Err(SviError::NoClosingTags { index: i });
      }

      // Get the variable
      let variable = close_split[0];

      match (variables.get(variable), fail_on_missing_variable) {
        (Some(value), _) => {
          // push the value onto result
          result.push_str(value);
          // add a replacer to sanitize the interpolation for logs etc.
          replacers.insert((value.clone(), variable.to_string()));
        }
        (None, false) => {
          // Basically push the original back onto the result, leaving it as is.
          result.push_str(double_opener);
          result.push_str(variable);
          result.push_str(double_closer);
        }
        (None, true) => {
          return Err(SviError::NoValueFound {
            variable: variable.to_string(),
          });
        }
      };
      // Push the rest of contents in between the variables.
      result.push_str(&close_split[1..].join(""));
    }
  }

  Ok((result, replacers.into_iter().collect()))
}

pub fn replace_in_string(input: &str, replacers: &Vec<(String, String)>) -> String {
  let mut result = input.to_string();

  for (to_replace, replacer) in replacers {
    // Maybe this could be done in place...
    result = result.replace(to_replace, &format!("<{replacer}>"));
  }

  result
}

#[derive(Error, Debug)]
pub enum SviError {
  #[error("split is empty.")]
  SplitEmpty,
  #[error("found double opener '{double_opener}{double_opener}', formatting will fail")]
  FoundDoubleOpener { double_opener: String },
  #[error("no closing tags for variable at index {index}")]
  NoClosingTags { index: usize },
  #[error("did not find any value for variable {variable}")]
  NoValueFound { variable: String },
}

pub type Result<T> = std::result::Result<T, SviError>;
