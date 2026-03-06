use subset::Subset;

#[allow(dead_code)]
struct User {
    username: String,
    email: String,
    followers: usize,
    following: usize,
}

impl User {
    fn calculate_follower_ratio(&self) -> f64 {
        self.followers as f64 / self.following as f64
    }
}

#[derive(Subset)]
#[subset(from = "User")]
#[subset(functions = ["from::calculate_follower_ratio"])]
struct AliasedUser {
    username: String,
    #[subset(alias = "followers")]
    aliased_followers: usize,
    #[subset(alias = "following")]
    aliased_following: usize,
}

#[test]
fn functions_construct_through_path() {
    let user = User {
        username: "Jerry".to_string(),
        email: String::new(),
        followers: 2,
        following: 1,
    };
    assert_eq!(2.0, user.calculate_follower_ratio());
    let aliased_user: AliasedUser = user.into();
    assert_eq!(aliased_user.username, "Jerry");
    assert_eq!(aliased_user.aliased_followers, 2);
    assert_eq!(2.0, aliased_user.calculate_follower_ratio());
}
