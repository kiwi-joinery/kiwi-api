use diesel::sql_types::{Text, Integer};
use diesel::sql_function;

sql_function!(fn strpos (string: Text, substring: Text) -> Integer);
