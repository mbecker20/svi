use std::collections::{HashMap, HashSet, VecDeque};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error("split is empty.")]
  SplitEmpty,
  #[error(
    "found double opener '{double_opener}{double_opener}', formatting will fail"
  )]
  FoundDoubleOpener { double_opener: String },
  #[error("no closing tags for variable at index {index}")]
  NoClosingTags { index: usize },
  #[error("did not find any value for variable {variable}")]
  NoValueFound { variable: String },
}

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

  let (double_opener, single_opener, triple_closer, double_closer) =
    match interpolator {
      Interpolator::DoubleCurlyBrackets => ("{{", "{", "}}}", "}}"),
      Interpolator::DoubleBrackets => ("[[", "[", "]]]", "]]"),
    };

  // Split the input by double opener '{{' or '[['
  let mut open_split =
    input.split(double_opener).collect::<VecDeque<_>>();

  // The first value in the split will be before the first variable. Push it to the result.
  let first = open_split.pop_front().ok_or(Error::SplitEmpty)?;
  result.push_str(first);

  // Iterate through the rest of the splits.
  // At this point, the input 'mongodb://[[MONGO_USERNAME]]:[[MONGO_PASSWORD]]@localhost:27017'
  // would have split looking like (keep in mind the beginning is already popped off):
  // ["MONGO_USERNAME]]:", "MONGO_PASSWORD]]@localhost:27017"].
  for (i, val) in open_split.iter().enumerate() {
    // Check if the input uses a disallowed 'double opener'.
    // '{{{{' or '[[[['.
    if val.get(0..1).is_none() {
      return Err(Error::FoundDoubleOpener {
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
        return Err(Error::NoClosingTags { index: i });
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
          return Err(Error::NoValueFound {
            variable: variable.to_string(),
          });
        }
      };
      // Push the rest of contents in between the variables.
      result.push_str(&close_split[1..].join(double_closer));
    }
  }

  Ok((result, replacers.into_iter().collect()))
}

pub fn replace_in_string<'a>(
  input: &str,
  replacers: impl IntoIterator<Item = &'a (String, String)>,
) -> String {
  let mut result = input.to_string();

  for (to_replace, replacer) in replacers {
    // Maybe this could be done in place...
    result = result.replace(to_replace, &format!("<{replacer}>"));
  }

  result
}

#[cfg(test)]
mod test {
  use super::*;

  fn variables(vars: &[(&str, &str)]) -> HashMap<String, String> {
    vars
      .iter()
      .map(|(k, v)| (k.to_string(), v.to_string()))
      .collect()
  }

  fn replacers(replacers: &[(&str, &str)]) -> Vec<(String, String)> {
    replacers
      .iter()
      .map(|(k, v)| (k.to_string(), v.to_string()))
      .collect()
  }

  #[test]
  fn no_vars() {
    let source = "no variables in here";
    let res = interpolate_variables(
      source,
      &Default::default(),
      Interpolator::DoubleBrackets,
      true,
    )
    .unwrap();
    assert_eq!(res, (String::from(source), Vec::new()))
  }

  #[test]
  fn start() {
    let source = "[[KEY]] at the front";
    let vars = variables(&[("KEY", "value")]);
    let res = interpolate_variables(
      source,
      &vars,
      Interpolator::DoubleBrackets,
      true,
    )
    .unwrap();
    assert_eq!(
      res,
      (
        String::from("value at the front"),
        replacers(&[("value", "KEY")])
      )
    )
  }

  #[test]
  fn middle() {
    let source = "middle [[KEY]] not at front";
    let vars = variables(&[("KEY", "value")]);
    let res = interpolate_variables(
      source,
      &vars,
      Interpolator::DoubleBrackets,
      true,
    )
    .unwrap();
    assert_eq!(
      res,
      (
        String::from("middle value not at front"),
        replacers(&[("value", "KEY")])
      )
    )
  }

  #[test]
  fn end() {
    let source = "not at front [[KEY]]";
    let vars = variables(&[("KEY", "value")]);
    let res = interpolate_variables(
      source,
      &vars,
      Interpolator::DoubleBrackets,
      true,
    )
    .unwrap();
    assert_eq!(
      res,
      (
        String::from("not at front value"),
        replacers(&[("value", "KEY")])
      )
    )
  }

  #[test]
  fn all() {
    let source = "[[FRONT]] at front, [[MIDDLE]] in middle, and on then the [[END]]";
    let vars =
      variables(&[("FRONT", "f"), ("MIDDLE", "m"), ("END", "e")]);
    let mut res = interpolate_variables(
      source,
      &vars,
      Interpolator::DoubleBrackets,
      true,
    )
    .unwrap();
    res.1.sort();
    assert_eq!(
      res,
      (
        String::from("f at front, m in middle, and on then the e"),
        replacers(&[("e", "END"), ("f", "FRONT"), ("m", "MIDDLE")])
      )
    )
  }

  #[test]
  fn escaped() {
    let source = "[[[FRONT]]] at front, [[[MIDDLE]]] in middle, and on then the [[[END]]]";
    let vars =
      variables(&[("FRONT", "f"), ("MIDDLE", "m"), ("END", "e")]);
    let res = interpolate_variables(
      source,
      &vars,
      Interpolator::DoubleBrackets,
      true,
    )
    .unwrap();
    assert_eq!(
      res,
      (
        String::from(
          "[[FRONT]] at front, [[MIDDLE]] in middle, and on then the [[END]]"
        ),
        Vec::new()
      )
    )
  }

  #[test]
  /// https://github.com/mbecker20/svi/pull/1
  fn close_without_open() {
    let source =
      "mongodb://[[USERNAME]]:mongo_password]]@127.0.0.1:27017";
    let vars = variables(&[("USERNAME", "root")]);
    let res = interpolate_variables(
      source,
      &vars,
      Interpolator::DoubleBrackets,
      true,
    )
    .unwrap();
    assert_eq!(
      res,
      (
        String::from(
          "mongodb://root:mongo_password]]@127.0.0.1:27017"
        ),
        replacers(&[("root", "USERNAME")])
      )
    )
  }
}
