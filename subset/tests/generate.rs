use subset::Subset;

#[allow(dead_code)]
#[derive(Debug)]
struct User {
    id: u32,
    username: String,
    email: String,
    last_login: chrono::DateTime<chrono::Utc>,
}

fn make_display(username: &str, id: u32) -> String {
    format!("{}#{}", username, id)
}

#[derive(Debug, Subset)]
#[subset(from = "User")]
struct DisplayUser {
    #[subset(generate = "make_display(&from.username, from.id)")]
    display_name_functioned: String,
    #[subset(generate = "format!(\"{}#{}\", from.username, from.id)")]
    display_name_closured: String,
}

#[test]
fn converts_user_with_alias_into_public_user() {
    let user = User {
        id: 1234,
        username: "Jerry".to_string(),
        email: String::new(),
        last_login: chrono::DateTime::<chrono::Utc>::MIN_UTC,
    };
    let public_user: DisplayUser = user.into();
    assert_eq!(public_user.display_name_functioned, "Jerry#1234");
    assert_eq!(public_user.display_name_closured, "Jerry#1234");
}
