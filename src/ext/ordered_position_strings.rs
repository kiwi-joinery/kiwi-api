use itertools::Itertools;

// The position of the first item to be stored
pub fn first_string() -> String {
    "a".to_string()
}

// The position of a new item inserted after the last of the list
pub fn next_string(x: &str) -> String {
    assert!(x.chars().all(|x| x.is_ascii_lowercase()));
    let mut base = String::new();
    let mut x = x.chars();
    loop {
        match x.next() {
            None => {
                base.push('a');
                break;
            }
            Some('z') => {
                base.push('z');
            }
            Some(x) => {
                base.push(std::char::from_u32(x as u32 + 1).unwrap());
                break;
            }
        }
    }
    base
}

// The position of a new item inserted between the list
pub fn string_in_between(after: &str, before: &str) -> String {
    assert!(after.chars().all(|x| x.is_ascii_lowercase()));
    assert!(before.chars().all(|x| x.is_ascii_lowercase()));
    assert_ne!(after, before);
    let after = after.chars().collect_vec();
    let before = before.chars().collect_vec();

    let mut matching_char_count = 0;
    for i in 0..after.len() {
        let x = after.get(i).unwrap();
        let y = match before.get(i) {
            None => break,
            Some(y) => y,
        };
        if x == y {
            matching_char_count = matching_char_count + 1;
        } else {
            break;
        }
    }

    let mut base = after[..matching_char_count].iter().collect::<String>();
    let after = &after[matching_char_count..];
    let before = &before[matching_char_count..];

    let after_first = after.get(0).map(|x| x.clone() as u32).unwrap_or(0);
    let before_first = before.get(0).map(|x| x.clone() as u32).unwrap_or(0);

    if before_first - after_first <= 1 {
        base.push(std::char::from_u32(after_first).unwrap());
        base.push('a');
    } else {
        base.push(std::char::from_u32(after_first + 1).unwrap());
    }
    base
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_string_in_between() {
        fn t(after: &str, before: &str) -> String {
            let res = string_in_between(after, before);
            let mut list = vec![res.as_str(), before, after];
            list.sort();
            assert_eq!(list, vec![after, res.as_str(), before]);
            res
        }
        assert_eq!(t("a", "c"), "b");
        assert_eq!(t("a", "b"), "aa");
        assert_eq!(t("a", "basdfasdf"), "aa");
        assert_eq!(t("aaab", "aaac"), "aaaba");
        assert_eq!(t("ggc", "ggz"), "ggd");
    }

    #[test]
    fn test_next_string() {
        assert_eq!(next_string("a"), "b");
        assert_eq!(next_string("aasgsd"), "b");
        assert_eq!(next_string("z"), "za");
        assert_eq!(next_string("zzzb"), "zzzc");
    }
}
