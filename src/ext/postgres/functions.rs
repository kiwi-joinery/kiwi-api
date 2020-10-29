use diesel::sql_function;
use diesel::sql_types::{Integer, Text};

sql_function!(fn strpos (string: Text, substring: Text) -> Integer);
