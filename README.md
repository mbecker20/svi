# SVI
## *string variable interpolator*

this crate contains a function that interpolates variables in a hashmap into an input string, producing both the resulting string, plus a readable string that is safe to store or print without compromising secrets.

variables can be matched with either ```[[variable_name]]``` (DoubleBrackets) or ```{{variable_name}}``` (DoubleCurlyBrackets).

## Usage
```rust
let variables: HashMap<String, String> = [
	("mongo_username", "root"),
	("mongo_password", "mng233985725"),
]
.into_iter()
.map(|(k, v)| (k.to_string(), v.to_string()))
.collect();

let to_fmt = "mongodb://[[mongo_username]]:[[mongo_password]]@127.0.0.1:27017";

let (formatted, readable, replacers) = svi::interpolate_variables(to_fmt, &variables, svi::Interpolator::DoubleBrackets)?;

println!("{formatted}"); // prints 'mongodb://root:mng233985725@127.0.0.1:27017'
println!("{readable}"); // prints 'mongodb://<mongo_username>:<mongo_password>@127.0.0.1:27017'

// can then make any output involving these replaced variables safe to print / store
let to_fmt = "got 'mongodb://root:mng233985725@127.0.0.1:27017'";
let readable = svi::replace_in_string(to_fmt, &replacers);

println!("{readable}"); // prints: got 'mongodb://<mongo_username>:<mongo_password>@127.0.0.1:27017'
```

## Escaping Interpolation
```rust
let variables = HashMap::new();

let to_fmt = "the interpolator will escape interpolation with 3 openers: [[[not a variable]]]";

let (formatted, _) = svi::interpolate_variables(to_fmt, &variables, svi::Interpolator::DoubleBrackets)?;

println!("{formatted}"); // prints 'the interpolator will escape interpolation with 3 openers: [[not a variable]]'
```