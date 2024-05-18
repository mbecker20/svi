use std::collections::HashMap;

fn main() -> svi::Result<()> {
  let input = "mongodb://[[MONGO_USERNAME]]:[[MONGO_PASSWORD]]@[[MONGO_ADDRESS]]";

  let variables = [("MONGO_ADDRESS", "localhost:27017")]
    .into_iter()
    .map(|(var, val)| (var.to_string(), val.to_string()))
    .collect::<HashMap<_, _>>();

  let (res, _) =
    svi::interpolate_variables(input, &variables, svi::Interpolator::DoubleBrackets, false)?;

  println!("input: {input}");
  println!("first replace: {res}");

  let variables = [
    ("MONGO_USERNAME", "mongo_user_123"),
    ("MONGO_PASSWORD", "mongo_pass_321"),
  ]
  .into_iter()
  .map(|(var, val)| (var.to_string(), val.to_string()))
  .collect::<HashMap<_, _>>();

  let (res, replacers) =
    svi::interpolate_variables(&res, &variables, svi::Interpolator::DoubleBrackets, true)?;

  println!("second replace: {res}");
  println!("sanitized: {}", svi::replace_in_string(&res, &replacers));

  Ok(())
}
